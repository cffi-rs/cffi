use std::error::Error;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign, Slice};

pub struct StrMarshaler<'a>(&'a PhantomData<()>);

impl InputType for StrMarshaler<'_> {
    type Foreign = Slice<u8>;
}

impl ReturnType for StrMarshaler<'_> {
    type Foreign = Slice<u8>;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        Slice::default()
    }
}

impl<'a> ToForeign<&'a str, Slice<u8>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(input: &'a str) -> Result<Slice<u8>, Self::Error> {
        let bytes = input.to_owned().into_boxed_str().into_boxed_bytes();
        let len = bytes.len();

        Ok(Slice {
            data: Box::into_raw(bytes) as _,
            len
        })
    }
}

impl<'a> FromForeign<Slice<u8>, &'a str> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(slice: Slice<u8>) -> Result<&'a str, Self::Error> {
        if slice.data.is_null() {
            return Err(null_ptr_error());
        }
        
        let r = std::slice::from_raw_parts(slice.data as _, slice.len);
        std::str::from_utf8(r).map_err(|e| Box::new(e) as _)
    }
}

impl<'a> FromForeign<Slice<u8>, Option<&'a str>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(slice: Slice<u8>) -> Result<Option<&'a str>, Self::Error> {
        if slice.data.is_null() {
            return Ok(None);
        }
        
        let r = std::slice::from_raw_parts(slice.data as _, slice.len);
        std::str::from_utf8(r)
            .map(Some)
            .map_err(|e| Box::new(e) as _)
    }
}
