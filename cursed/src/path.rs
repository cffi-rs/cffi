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

pub struct PathMarshaler;

#[cfg(windows)]
impl InputType for PathMarshaler {
    type Foreign = *const wchar_t;
}

#[cfg(unix)]
impl InputType for PathMarshaler {
    type Foreign = *const c_char;
}

#[cfg(windows)]
impl ReturnType for PathMarshaler {
    type Foreign = *const libc::wchar_t;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

#[cfg(unix)]
impl ReturnType for PathMarshaler {
    type Foreign = Slice<u8>;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        Slice::<u8>::default()
    }
}

#[cfg(windows)]
impl<'a> FromForeign<*const wchar_t, PathBuf> for PathMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(c_wstr: *const wchar_t) -> Result<PathBuf, Self::Error> {
        use std::os::windows::ffi::OsStringExt;

        let len = unsafe { libc::wcslen(c_wstr) };
        let slice: &[u16] = unsafe { std::slice::from_raw_parts(c_wstr, len) };
        let osstr = std::ffi::OsString::from_wide(slice);
        Ok(osstr.into())
    }
}

#[cfg(windows)]
impl ToForeign<PathBuf, *const wchar_t> for PathMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(input: PathBuf) -> Result<*const wchar_t, Self::Error> {
        use std::os::windows::ffi::OsStrExt;

        let mut vec: Vec<wchar_t> =
            input.into_os_string().encode_wide().chain(Some(0).into_iter()).collect();
        vec.shrink_to_fit();
        let ptr = vec.as_ptr();
        std::mem::forget(vec);
        Ok(ptr)
    }
}

#[cfg(unix)]
impl FromForeign<*const c_char, PathBuf> for PathMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const c_char) -> Result<PathBuf, Self::Error> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        let c_str = unsafe { CStr::from_ptr(foreign.cast()) };
        let os_str = OsStr::from_bytes(c_str.to_bytes());
        Ok(Path::new(os_str).to_path_buf())
    }
}

#[cfg(unix)]
impl ToForeign<PathBuf, Slice<u8>> for PathMarshaler {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(input: PathBuf) -> Result<Slice<u8>, Self::Error> {
        use std::os::unix::ffi::OsStringExt;

        let vec = input.into_os_string().into_vec();
        VecMarshaler::to_foreign(vec)
    }
}

#[cfg(windows)]
impl<E> ToForeign<Result<PathBuf, E>, *const wchar_t> for PathMarshaler {
    type Error = E;

    #[inline(always)]
    fn to_foreign(input: Result<PathBuf, E>) -> Result<*const wchar_t, Self::Error> {
        input.and_then(|x| Ok(PathMarshaler::to_foreign(x).unwrap()))
    }
}

#[cfg(unix)]
impl<E> ToForeign<Result<PathBuf, E>, Slice<u8>> for PathMarshaler {
    type Error = E;

    #[inline(always)]
    fn to_foreign(input: Result<PathBuf, E>) -> Result<Slice<u8>, Self::Error> {
        input.and_then(|x| Ok(PathMarshaler::to_foreign(x).unwrap()))
    }
}
