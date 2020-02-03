use std::error::Error;
use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::Arc;
use std::convert::Infallible;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign, Slice, vec::VecMarshaler};
use libc::c_char;

pub struct StringMarshaler;

impl InputType for StringMarshaler {
    type Foreign = Slice<u8>;
}

impl ReturnType for StringMarshaler {
    type Foreign = Slice<u8>;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        Slice::default()
    }
}

impl ToForeign<String, Slice<u8>> for StringMarshaler {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(string: String) -> Result<Slice<u8>, Self::Error> {
        VecMarshaler::to_foreign(string.into_bytes())
    }
}

impl ToForeign<Result<String, Box<dyn Error>>, Slice<u8>> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(result: Result<String, Box<dyn Error>>) -> Result<Slice<u8>, Self::Error> {
        result.and_then(|v| Ok(StringMarshaler::to_foreign(v).unwrap()))
    }
}

impl ToForeign<Option<String>, Slice<u8>> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(option: Option<String>) -> Result<Slice<u8>, Self::Error> {
        match option {
            None => Ok(Slice::default()),
            Some(v) => Ok(StringMarshaler::to_foreign(v).unwrap())
        }
    }
}


// impl<'a> FromForeign<Slice<u8>, String> for StringMarshaler {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     fn from_foreign(key: Slice<u8>) -> Result<String, Self::Error> {
//         if slice.data.is_null() {
//             return Err(null_ptr_error());
//         }

//         let boxed: Box<[u8]> = unsafe { key.into_box() };
//         let s = unsafe { std::str::from_boxed_utf8_unchecked(boxed) };
//         Ok(s)
//     }
// }
