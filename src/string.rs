use std::convert::Infallible;
use std::error::Error;

use super::{vec::VecMarshaler, FromForeign, InputType, ReturnType, Slice, ToForeign};

pub struct StringMarshaler;

impl InputType for StringMarshaler {
    type Foreign = Slice<u8>;
    type ForeignTraitObject = ();
}

impl ReturnType for StringMarshaler {
    type Foreign = Slice<u8>;
    type ForeignTraitObject = ();

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
            Some(v) => Ok(StringMarshaler::to_foreign(v).unwrap()),
        }
    }
}

impl<'a> FromForeign<Slice<u8>, String> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(key: Slice<u8>) -> Result<String, Self::Error> {
        VecMarshaler::from_foreign(key)
            .and_then(|vec| String::from_utf8(vec).map_err(|err| Box::new(err) as _))
    }
}

#[no_mangle]
pub unsafe extern "C" fn cffi_string_free(slice: Slice<u8>) {
    crate::vec::cffi_vec_free(slice.cast());
}
