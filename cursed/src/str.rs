use std::error::Error;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct StrMarshaler<'a>(&'a PhantomData<()>);

impl InputType for StrMarshaler<'_> {
    type Foreign = *const libc::c_char;
}

impl ReturnType for StrMarshaler<'_> {
    type Foreign = *const libc::c_char;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl<'a> ToForeign<&'a str, *const libc::c_char> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(input: &'a str) -> Result<*const libc::c_char, Self::Error> {
        let c_str = CString::new(input)?;
        Ok(c_str.into_raw().cast())
    }
}

impl<'a> FromForeign<*const libc::c_char, &'a str> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const libc::c_char) -> Result<&'a str, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CStr::from_ptr(key.cast()) }.to_str()?)
    }
}

impl<'a> FromForeign<*const libc::c_char, Option<&'a str>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const libc::c_char) -> Result<Option<&'a str>, Self::Error> {
        if key.is_null() {
            return Ok(None);
        }

        Ok(Some(unsafe { CStr::from_ptr(key.cast()) }.to_str()?))
    }
}

// impl<'a> FromForeign<*const libc::c_char, Cow<'a, str>> for StrMarshaler<'a> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     fn from_foreign(key: *const libc::c_char) -> Result<Cow<'a, str>, Self::Error> {
//         if key.is_null() {
//             return Err(null_ptr_error());
//         }

//         Ok(unsafe { CStr::from_ptr(key.cast()) }.to_string_lossy())
//     }
// }
