use std::error::Error;
use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};
use libc::c_char;

pub struct StringMarshaler;

impl InputType for StringMarshaler {
    type Foreign = *const c_char;
}

impl ReturnType for StringMarshaler {
    type Foreign = *const c_char;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl ToForeign<String, *const c_char> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(string: String) -> Result<*const c_char, Self::Error> {
        let c_str = std::ffi::CString::new(string)?;
        Ok(CString::into_raw(c_str).cast())
    }
}

impl ToForeign<Result<String, Box<dyn Error>>, *const c_char> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(result: Result<String, Box<dyn Error>>) -> Result<*const c_char, Self::Error> {
        result.and_then(|v| StringMarshaler::to_foreign(v))
    }
}

impl ToForeign<Option<String>, *const c_char> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(option: Option<String>) -> Result<*const c_char, Self::Error> {
        option.map_or_else(|| Ok(std::ptr::null()), |v| StringMarshaler::to_foreign(v))
    }
}

impl<'a> FromForeign<*mut c_char, CString> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *mut c_char) -> Result<CString, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CString::from_raw(key.cast()) })
    }
}

impl<'a> FromForeign<*const c_char, String> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const c_char) -> Result<String, Self::Error> {
        let c_string = <Self as FromForeign<*mut c_char, CString>>::from_foreign(key as *mut _)?;
        Ok(c_string.into_string()?)
    }
}
