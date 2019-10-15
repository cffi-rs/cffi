use std::error::Error;
use std::ffi::{c_void, CString};
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct StringMarshaler;

impl InputType for StringMarshaler {
    type Foreign = *const c_void;
}

impl ReturnType for StringMarshaler {
    type Foreign = *const c_void;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl ToForeign<String, *const c_void> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(string: String) -> Result<*const c_void, Self::Error> {
        let c_str = std::ffi::CString::new(string)?;
        Ok(CString::into_raw(c_str).cast())
    }
}

impl<E> ToForeign<Result<String, E>, *const c_void> for StringMarshaler where E: std::error::Error + 'static {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(result: Result<String, E>) -> Result<*const c_void, Self::Error> {
        match result {
            Ok(v) => StringMarshaler::to_foreign(v),
            Err(e) => Err(Box::new(e))
        }
    }
}

impl<'a> FromForeign<*mut c_void, CString> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *mut c_void) -> Result<CString, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CString::from_raw(key.cast()) })
    }
}

impl<'a> FromForeign<*const c_void, String> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const c_void) -> Result<String, Self::Error> {
        let c_string = <Self as FromForeign<*mut c_void, CString>>::from_foreign(key as *mut _)?;
        Ok(c_string.into_string()?)
    }
}
