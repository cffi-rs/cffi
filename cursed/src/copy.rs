use super::{FromForeign, InputType, ReturnType, ToForeign};
use std::convert::Infallible;

pub struct CopyMarshaler<T: Copy>(std::marker::PhantomData<T>);

impl<T: Copy> InputType for CopyMarshaler<T> {
    type Foreign = T;
}

impl<T: Copy + Default> ReturnType for CopyMarshaler<T> {
    type Foreign = T;

    fn foreign_default() -> T {
        T::default()
    }
}

impl<T: Copy> FromForeign<T, T> for CopyMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn from_foreign(x: T) -> Result<T, Self::Error> {
        Ok(x)
    }
}

impl<T: Copy> ToForeign<T, T> for CopyMarshaler<T> {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn to_foreign(x: T) -> Result<T, Self::Error> {
        Ok(x)
    }
}
