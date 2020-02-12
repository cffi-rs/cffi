use std::convert::Infallible;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::error::Error;

#[cfg(unix)]
use libc::c_char;
#[cfg(windows)]
use libc::wchar_t;

use super::null_ptr_error;
use super::vec::VecMarshaler;
use super::{FromForeign, InputType, ReturnType, ToForeign, Slice};

pub struct PathBufMarshaler;

impl InputType for PathBufMarshaler {
    type Foreign = Slice<u16>;
}

impl ReturnType for PathBufMarshaler {
    type Foreign = Slice<u16>;

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
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(input: PathBuf) -> Result<Slice<u16>, Self::Error> {
        use std::os::windows::ffi::OsStrExt;

        let mut vec: Vec<wchar_t> =
            input.into_os_string().encode_wide().chain(Some(0).into_iter()).collect();
        vec.shrink_to_fit();
        let ptr = vec.as_ptr();
        std::mem::forget(vec);
        Ok(ptr)
    }
}

impl<E> ToForeign<Result<PathBuf, E>, Slice<u16>> for PathBufMarshaler {
    type Error = E;

    #[inline(always)]
    fn to_foreign(input: Result<PathBuf, E>) -> Result<Slice<u16>, Self::Error> {
        input.and_then(|x| Ok(PathBufMarshaler::to_foreign(x).unwrap()))
    }
}