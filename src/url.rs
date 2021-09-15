use std::error::Error;
use url::Url;

use super::{FromForeign, InputType, ReturnType, Slice, ToForeign};

pub struct UrlMarshaler;

impl InputType for UrlMarshaler {
    type Foreign = Slice<u8>;
    type ForeignTraitObject = ();
}

impl ReturnType for UrlMarshaler {
    type Foreign = Slice<u8>;
    type ForeignTraitObject = ();

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        Default::default()
    }
}

// Url -> char pointer
impl ToForeign<Url, Slice<u8>> for UrlMarshaler {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn to_foreign(url: Url) -> Result<Slice<u8>, Self::Error> {
        let url = url.to_string();
        crate::StringMarshaler::to_foreign(url)
    }
}

// Result<Url> -> char pointer
impl ToForeign<Result<Url, Box<dyn Error>>, Slice<u8>> for UrlMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(result: Result<Url, Box<dyn Error>>) -> Result<Slice<u8>, Self::Error> {
        result.and_then(|url| Ok(UrlMarshaler::to_foreign(url).unwrap()))
    }
}

// Option<Url> -> char pointer
impl ToForeign<Option<Url>, Slice<u8>> for UrlMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(option: Option<Url>) -> Result<Slice<u8>, Self::Error> {
        option.map_or_else(
            || Ok(Default::default()),
            |url| Ok(UrlMarshaler::to_foreign(url).unwrap()),
        )
    }
}

// char pointer -> URL
impl<'a> FromForeign<Slice<u8>, Url> for UrlMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(key: Slice<u8>) -> Result<Url, Self::Error> {
        let s = crate::StringMarshaler::from_foreign(key)?;
        Url::parse(&s).map_err(|e| Box::new(e) as _)
    }
}
