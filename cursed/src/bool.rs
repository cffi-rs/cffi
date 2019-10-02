use super::{FromForeign, ToForeign, ReturnType};
use std::convert::Infallible;

pub struct BoolMarshaler;

impl FromForeign<u8, bool> for BoolMarshaler {
    type Error = Infallible;

    #[inline(always)]
    fn from_foreign(i: u8) -> Result<bool, Self::Error> {
        Ok(i != 0)
    }
}

impl ToForeign<bool, u8> for BoolMarshaler {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn to_foreign(b: bool) -> Result<u8, Self::Error> {
        Ok(if b { 1 } else { 0 })
    }
}
