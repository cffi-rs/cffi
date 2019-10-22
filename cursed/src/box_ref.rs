use std::error::Error;
use std::marker::PhantomData;
use std::mem::transmute;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct BoxRefMarshaler<T>(PhantomData<T>);

impl<T> InputType for BoxRefMarshaler<T> {
    type Foreign = *mut Box<T>;
}

impl<'a, T> FromForeign<*mut Box<T>, &'a Box<T>> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *mut Box<T>) -> Result<&'a Box<T>, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as FromForeign<*mut Box<T>, &'a Box<T>>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { &*foreign as &'a Box<T> })
    }
}

impl<'a, T> FromForeign<*mut Box<T>, &'a mut Box<T>> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *mut Box<T>) -> Result<&'a mut Box<T>, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as FromForeign<*mut Box<T>, &'a mut Box<T>>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { &mut *foreign as &'a mut Box<T> })
    }
}
