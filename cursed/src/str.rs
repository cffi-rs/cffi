use std::error::Error;
use std::ffi::{c_void, CStr, CString};
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct StrMarshaler<'a>(&'a PhantomData<()>);

impl InputType for StrMarshaler<'_> {
    type Foreign = *const c_void;
}

impl ReturnType for StrMarshaler<'_> {
    type Foreign = *const c_void;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl<'a> ToForeign<&'a str, *const c_void> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(input: &'a str) -> Result<*const c_void, Self::Error> {
        let c_str = CString::new(input)?;
        Ok(c_str.into_raw().cast())
    }
}

impl<'a> FromForeign<*const c_void, &'a str> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const c_void) -> Result<&'a str, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CStr::from_ptr(key.cast()) }.to_str()?)
    }
}

impl<'a> FromForeign<*const c_void, Option<&'a str>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const c_void) -> Result<Option<&'a str>, Self::Error> {
        if key.is_null() {
            return Ok(None);
        }

        Ok(Some(unsafe { CStr::from_ptr(key.cast()) }.to_str()?))
    }
}

// impl<'a> FromForeign<*const c_void, Cow<'a, str>> for StrMarshaler<'a> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     fn from_foreign(key: *const c_void) -> Result<Cow<'a, str>, Self::Error> {
//         if key.is_null() {
//             return Err(null_ptr_error());
//         }

//         Ok(unsafe { CStr::from_ptr(key.cast()) }.to_string_lossy())
//     }
// }
