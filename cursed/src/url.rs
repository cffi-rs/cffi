use std::error::Error;
use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::Arc;

use libc::c_char;
use url::Url;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct UrlMarshaler;

impl InputType for UrlMarshaler {
    type Foreign = *const c_char;
}

impl ReturnType for UrlMarshaler {
    type Foreign = *const c_char;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

// Url -> char pointer
impl ToForeign<Url, *const c_char> for UrlMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(url: Url) -> Result<*const c_char, Self::Error> {
        let c_str = std::ffi::CString::new(url.as_str())?;
        Ok(CString::into_raw(c_str).cast())
    }
}

// Result<Url> -> char pointer
impl ToForeign<Result<Url, Box<dyn Error>>, *const c_char> for UrlMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(result: Result<Url, Box<dyn Error>>) -> Result<*const c_char, Self::Error> {
        result.and_then(|v| UrlMarshaler::to_foreign(v))
    }
}

// Option<Url> -> char pointer
impl ToForeign<Option<Url>, *const c_char> for UrlMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(option: Option<Url>) -> Result<*const c_char, Self::Error> {
        option.map_or_else(|| Ok(std::ptr::null()), |v| UrlMarshaler::to_foreign(v))
    }
}

// char pointer -> URL
impl<'a> FromForeign<*const c_char, Url> for UrlMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const c_char) -> Result<Url, Self::Error> {
        let s = crate::StrMarshaler::from_foreign(key)?;
        Url::parse(s).map_err(|e| Box::new(e) as _)
    }
}
