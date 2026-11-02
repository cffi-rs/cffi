use std::convert::Infallible;
use std::error::Error;
use std::path::PathBuf;

#[cfg(unix)]
use libc::c_char;
#[cfg(windows)]
use libc::wchar_t;

use crate::null_ptr_error;
use crate::vec::VecMarshaler;
use crate::{FromForeign, InputType, ReturnType, Slice, ToForeign};

pub struct PathBufMarshaler;

impl PathBufMarshaler {
    /// Frees a path made by `PathBufMarshaler::to_foreign`.
    ///
    /// A slice with NULL data or a length of 0 is ignored. A path crosses as UTF-16 on Windows
    /// and as bytes elsewhere, so neither `cffi_string_free` nor `cffi_vec_free` can free it on
    /// every OS; a library that returns one exports a free function of its own that calls this:
    ///
    /// ```rust
    /// use cffi::{PathBufMarshaler, ReturnType};
    ///
    /// #[unsafe(no_mangle)]
    /// pub unsafe extern "C" fn Example_Path_free(path: <PathBufMarshaler as ReturnType>::Foreign) {
    ///     unsafe { PathBufMarshaler::free(path) }
    /// }
    /// ```
    ///
    /// # Safety
    /// The slice must have NULL data, a length of 0, or have been made by
    /// `PathBufMarshaler::to_foreign`, and not already have been freed.
    pub unsafe fn free(slice: Slice<u16>) {
        unsafe { VecMarshaler::<u16>::free(slice) }
    }
}

impl InputType for PathBufMarshaler {
    type Foreign = Slice<u16>;
    type ForeignTraitObject = ();
}

impl ReturnType for PathBufMarshaler {
    type Foreign = Slice<u16>;
    type ForeignTraitObject = ();

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        Slice::default()
    }
}

impl FromForeign<Slice<u16>, PathBuf> for PathBufMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(wstr: Slice<u16>) -> Result<PathBuf, Self::Error> {
        if wstr.data.is_null() {
            return Err(null_ptr_error());
        }

        use std::os::windows::ffi::OsStringExt;
        let slice: &[u16] = std::slice::from_raw_parts(wstr.data, wstr.len);
        let osstr = std::ffi::OsString::from_wide(slice);
        Ok(osstr.into())
    }
}

impl ToForeign<PathBuf, Slice<u16>> for PathBufMarshaler {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(input: PathBuf) -> Result<Slice<u16>, Self::Error> {
        use std::os::windows::ffi::OsStrExt;

        let vec: Vec<wchar_t> = input
            .into_os_string()
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect();
        VecMarshaler::to_foreign(vec)
    }
}

impl<E> ToForeign<Result<PathBuf, E>, Slice<u16>> for PathBufMarshaler {
    type Error = E;

    #[inline(always)]
    fn to_foreign(input: Result<PathBuf, E>) -> Result<Slice<u16>, Self::Error> {
        input.and_then(|x| Ok(PathBufMarshaler::to_foreign(x).unwrap()))
    }
}
