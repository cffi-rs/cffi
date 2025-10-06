use std::error::Error;
use std::marker::PhantomData;

use super::null_ptr_error;
use super::{FromForeign, InputType};

pub struct BoxRefMarshaler<T>(PhantomData<T>);
pub struct BoxMutRefMarshaler<T>(PhantomData<T>);

impl<T> InputType for BoxRefMarshaler<T> {
    type Foreign = *mut T;
    type ForeignTraitObject = ();
}

impl<T> InputType for BoxMutRefMarshaler<T> {
    type Foreign = *mut T;
    type ForeignTraitObject = ();
}

impl<'a, T> FromForeign<*mut T, &'a T> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: *mut T) -> Result<&'a T, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as FromForeign<*mut Box<T>, &'a mut Box<T>>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(&*foreign)
    }
}

impl<'a, T> FromForeign<*mut T, &'a mut T> for BoxMutRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: *mut T) -> Result<&'a mut T, Self::Error> {
        log::debug!(
            "<BoxMutMarshaler<{ty}> as FromForeign<*mut Box<T>, &'a mut Box<T>>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(&mut *foreign)
    }
}
