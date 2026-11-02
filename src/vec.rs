use std::convert::Infallible;
use std::error::Error;
use std::marker::PhantomData;

use super::null_ptr_error;
use super::panic::drop_caught;
use super::{FromForeign, InputType, ReturnType, Slice, ToForeign};

/// Marshaler for owned vectors, which cross as a [`Slice<T>`].
///
/// `ToForeign` hands the vector's memory to the foreign caller, who gives it back by calling a
/// free function that calls [`VecMarshaler::free`] for the same `T`. [`cffi_vec_free`] knows only
/// bytes, so it can free a `Vec<u8>` but no other vector.
pub struct VecMarshaler<T>(PhantomData<T>);

impl<T> VecMarshaler<T> {
    /// Frees a slice made by `VecMarshaler::<T>::to_foreign`, dropping each of its elements.
    ///
    /// A slice with NULL data or a length of 0 is ignored, so the zero value a failed call
    /// returns can be freed like any other. A panic in an element's destructor is caught here and
    /// goes no further.
    ///
    /// A generic function cannot be exported to C, so export a free function for each element
    /// type that crosses:
    ///
    /// ```rust
    /// use cffi::{Slice, VecMarshaler};
    ///
    /// pub struct Suggestion {
    ///     pub value: String,
    /// }
    ///
    /// #[unsafe(no_mangle)]
    /// pub unsafe extern "C" fn example_suggestions_free(suggestions: Slice<Suggestion>) {
    ///     unsafe { VecMarshaler::free(suggestions) }
    /// }
    /// ```
    ///
    /// # Safety
    /// The slice must have NULL data, a length of 0, or have been made by
    /// `VecMarshaler::<T>::to_foreign` for this same `T`, and not already have been freed or taken
    /// back with `from_foreign`.
    pub unsafe fn free(slice: Slice<T>) {
        if slice.data.is_null() || slice.len == 0 {
            return;
        }

        let boxed =
            unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(slice.data, slice.len)) };
        drop_caught(boxed);
    }
}

impl<T> InputType for VecMarshaler<T> {
    type Foreign = Slice<T>;
    type ForeignTraitObject = ();
}

impl<T> ReturnType for VecMarshaler<T> {
    type Foreign = Slice<T>;
    type ForeignTraitObject = ();

    fn foreign_default() -> Self::Foreign {
        Slice {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }
}

impl<T> ToForeign<Vec<T>, Slice<T>> for VecMarshaler<T> {
    type Error = Infallible;

    fn to_foreign(vec: Vec<T>) -> Result<Slice<T>, Self::Error> {
        // A boxed slice's allocation holds exactly `len` elements: the layout `free` and
        // `from_foreign` rebuild.
        let len = vec.len();
        let data = Box::into_raw(vec.into_boxed_slice()).cast::<T>();

        let raw = Slice { data, len };
        tracing::debug!("Ptr: {:?}", raw);
        Ok(raw)
    }
}

impl<T> FromForeign<Slice<T>, Vec<T>> for VecMarshaler<T> {
    type Error = Box<dyn Error>;

    unsafe fn from_foreign(ptr: Slice<T>) -> Result<Vec<T>, Self::Error> {
        if ptr.data.is_null() {
            return Err(null_ptr_error());
        }

        let vec = unsafe { Vec::from_raw_parts(ptr.data, ptr.len, ptr.len) };

        Ok(vec)
    }
}

impl<T> ToForeign<Result<Vec<T>, Box<dyn Error>>, Slice<T>> for VecMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(local: Result<Vec<T>, Box<dyn Error>>) -> Result<Slice<T>, Self::Error> {
        local.map(|x| VecMarshaler::to_foreign(x).unwrap())
    }
}

/// Frees a slice of bytes made by this library, such as a `Vec<u8>` from [`VecMarshaler`].
///
/// This cannot know the type the slice was made from, so it frees the memory as bytes and runs no
/// destructor. A slice of any other type must be freed by a function of its own that calls
/// [`VecMarshaler::free`]. A slice with NULL data or a length of 0 is ignored.
///
/// # Safety
/// The slice must have NULL data, a length of 0, or have been made by this library from bytes,
/// and not already have been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cffi_vec_free(slice: Slice<libc::c_void>) {
    unsafe { VecMarshaler::<u8>::free(slice.cast()) };
}
