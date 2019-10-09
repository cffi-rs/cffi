use std::ffi::{c_void, CStr};
use std::path::Path;

#[cfg(windows)]
use libc::wchar_t;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct PathMarshaler;

#[cfg(windows)]
impl InputType for PathMarshaler {
    type Foreign = *const wchar_t;
}

#[cfg(unix)]
impl InputType for PathMarshaler {
    type Foreign = *const libc::c_void;
}

#[cfg(windows)]
impl<'a> FromForeign<*const wchar_t, &'a Path> for PathMarshaler {
    type Error = Box<dyn Error>;

    fn from_foreign(c_wstr: *const wchar_t) -> Result<PathBuf, Self::Error> {
        let len = unsafe { libc::wcslen(c_wstr) };
        let slice: &[u16] = unsafe { std::slice::from_raw_parts(c_wstr, len) };
        let osstr = std::ffi::OsString::from_wide(slice);
        Ok(osstr.into())
    }
}

#[cfg(windows)]
impl ToForeign<PathBuf, *const wchar_t> for PathMarshaler {
    type Error = Box<dyn Error>;

    fn to_foreign(input: PathBuf) -> Result<*const wchar_t, Self::Error> {
        let mut vec: Vec<wchar_t> =
            input.into_os_string().encode_wide().chain(Some(0).into_iter()).collect();
        vec.shrink_to_fit();
        let ptr = vec.as_ptr();
        std::mem::forget(vec);
        Ok(ptr)
    }

    // fn drop_foreign(ptr: *const wchar_t) {
    //     let len = unsafe { libc::wcslen(ptr) };
    //     unsafe { Vec::from_raw_parts(ptr as *mut wchar_t, len, len) };
    // }
}

#[cfg(unix)]
impl<'a> FromForeign<*const c_void, &'a Path> for PathMarshaler {
    type Error = Box<std::io::Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const c_void) -> Result<&'a Path, Self::Error> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        let c_str = unsafe { CStr::from_ptr(foreign.cast()) };
        let os_str = OsStr::from_bytes(c_str.to_bytes());
        Ok(Path::new(os_str))
    }
}
