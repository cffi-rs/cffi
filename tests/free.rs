use std::alloc::{GlobalAlloc, Layout, System};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cffi::ffi::{cffi_string_free, cffi_vec_free};
use cffi::{
    ArcMarshaler, PathBufMarshaler, Slice, StrMarshaler, StringMarshaler, ToForeign, VecMarshaler,
};

/// Counts every deallocation whose layout is not the one the memory was allocated with. Freeing
/// with the wrong layout is undefined behaviour the system allocator rarely notices, so each
/// allocation records its layout in a header just before the memory it hands out.
struct LayoutChecker;

static LAYOUT_MISMATCHES: AtomicUsize = AtomicUsize::new(0);

const HEADER_ALIGN: usize = 16;

fn padded(layout: Layout) -> (Layout, usize) {
    let offset = layout.align().max(HEADER_ALIGN);
    let padded = Layout::from_size_align(layout.size() + offset, offset).unwrap();
    (padded, offset)
}

unsafe impl GlobalAlloc for LayoutChecker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (padded, offset) = padded(layout);
        let base = unsafe { System.alloc(padded) };
        if base.is_null() {
            return base;
        }

        unsafe {
            let ptr = base.add(offset);
            ptr.cast::<usize>().sub(1).write(layout.size());
            ptr.cast::<usize>().sub(2).write(layout.align());
            ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let allocated = unsafe {
            let size = ptr.cast::<usize>().sub(1).read();
            let align = ptr.cast::<usize>().sub(2).read();
            Layout::from_size_align_unchecked(size, align)
        };

        if allocated != layout {
            LAYOUT_MISMATCHES.fetch_add(1, Ordering::SeqCst);
        }

        let (padded, offset) = padded(allocated);
        unsafe { System.dealloc(ptr.sub(offset), padded) };
    }
}

#[global_allocator]
static ALLOCATOR: LayoutChecker = LayoutChecker;

fn assert_layouts_matched() {
    assert_eq!(LAYOUT_MISMATCHES.load(Ordering::SeqCst), 0);
}

/// Counts its drops, and owns heap memory of its own so a missed destructor would also leak.
struct Counted {
    drops: Arc<AtomicUsize>,
    _name: String,
}

impl Counted {
    fn new(drops: &Arc<AtomicUsize>, name: &str) -> Counted {
        Counted {
            drops: Arc::clone(drops),
            _name: name.to_owned(),
        }
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

struct PanicsOnDrop;

impl Drop for PanicsOnDrop {
    fn drop(&mut self) {
        panic!("dropped");
    }
}

#[test]
fn vec_free_drops_each_element_with_its_layout() {
    let drops = Arc::new(AtomicUsize::new(0));
    let vec = vec![
        Counted::new(&drops, "one"),
        Counted::new(&drops, "two"),
        Counted::new(&drops, "three"),
    ];

    let slice = VecMarshaler::to_foreign(vec).unwrap();
    assert_eq!(slice.len, 3);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    unsafe { VecMarshaler::free(slice) };

    assert_eq!(drops.load(Ordering::SeqCst), 3);
    assert_layouts_matched();
}

#[test]
fn vec_free_frees_spare_capacity() {
    let mut vec = Vec::with_capacity(64);
    vec.extend([1u64, 2, 3]);

    let slice = VecMarshaler::to_foreign(vec).unwrap();
    unsafe { VecMarshaler::free(slice) };

    assert_layouts_matched();
}

#[test]
fn vec_round_trips_through_from_foreign() {
    use cffi::FromForeign;

    let slice = VecMarshaler::to_foreign(vec![1u32, 2, 3]).unwrap();
    let vec: Vec<u32> = unsafe { VecMarshaler::from_foreign(slice) }.unwrap();
    assert_eq!(vec, [1, 2, 3]);
    drop(vec);

    assert_layouts_matched();
}

#[test]
fn vec_free_catches_a_panicking_destructor() {
    let slice = VecMarshaler::to_foreign(vec![PanicsOnDrop]).unwrap();
    unsafe { VecMarshaler::free(slice) };

    assert_layouts_matched();
}

#[test]
fn byte_frees_free_strings_and_bytes() {
    let string = StringMarshaler::to_foreign("owned".to_owned()).unwrap();
    unsafe { cffi_string_free(string) };

    let copied = StrMarshaler::to_foreign("copied").unwrap();
    unsafe { cffi_string_free(copied) };

    let bytes = VecMarshaler::to_foreign(vec![1u8, 2, 3]).unwrap();
    unsafe {
        cffi_vec_free(Slice {
            data: bytes.data.cast(),
            len: bytes.len,
        })
    };

    assert_layouts_matched();
}

#[test]
fn path_free_frees_a_path_with_its_layout() {
    let path = PathBufMarshaler::to_foreign(PathBuf::from("dir/file.txt")).unwrap();
    assert!(!path.data.is_null());
    unsafe { PathBufMarshaler::free(path) };

    assert_layouts_matched();
}

#[test]
fn null_and_empty_frees_do_nothing() {
    unsafe {
        cffi_string_free(Slice::default());
        cffi_vec_free(Slice::default());
        VecMarshaler::<Counted>::free(Slice::default());
        ArcMarshaler::<Counted>::free(std::ptr::null());
        PathBufMarshaler::free(Slice::default());

        // NULL data with a length, as a careless caller might pass.
        cffi_string_free(Slice {
            data: std::ptr::null_mut(),
            len: 5,
        });
        VecMarshaler::<Counted>::free(Slice {
            data: std::ptr::null_mut(),
            len: 5,
        });

        cffi_string_free(StringMarshaler::to_foreign(String::new()).unwrap());
        VecMarshaler::free(VecMarshaler::<Counted>::to_foreign(Vec::new()).unwrap());
    }

    assert_layouts_matched();
}

#[test]
fn arc_free_releases_the_handle() {
    let drops = Arc::new(AtomicUsize::new(0));
    let arc = Arc::new(Counted::new(&drops, "handle"));
    let weak = Arc::downgrade(&arc);

    let handle = ArcMarshaler::to_foreign(arc).unwrap();
    assert_eq!(weak.strong_count(), 1);

    unsafe { ArcMarshaler::free(handle) };

    assert_eq!(weak.strong_count(), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_layouts_matched();
}

#[test]
fn arc_free_leaves_other_references() {
    let drops = Arc::new(AtomicUsize::new(0));
    let arc = Arc::new(Counted::new(&drops, "shared"));

    let handle = ArcMarshaler::to_foreign(Arc::clone(&arc)).unwrap();
    unsafe { ArcMarshaler::free(handle) };

    assert_eq!(Arc::strong_count(&arc), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
}

#[test]
fn arc_free_catches_a_panicking_destructor() {
    let handle = ArcMarshaler::to_foreign(Arc::new(PanicsOnDrop)).unwrap();
    unsafe { ArcMarshaler::free(handle) };
}
