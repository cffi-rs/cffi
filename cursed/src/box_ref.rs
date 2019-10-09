use std::error::Error;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::transmute;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct BoxRefMarshaler<T>(PhantomData<T>);

impl<T> InputType for BoxRefMarshaler<T> {
    type Foreign = *const T;
}

impl<'a, T> FromForeign<*const c_void, &'a T> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const c_void) -> Result<&'a T, Self::Error> {
        log::debug!("<BoxMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, &'a T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        let ptr = unsafe { transmute::<*const c_void, *const T>(foreign) };

        Ok(unsafe { &*ptr as &'a T })
    }
}

impl<'a, T> FromForeign<*const c_void, &'a mut T> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const c_void) -> Result<&'a mut T, Self::Error> {
        log::debug!("<BoxMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, &'a mut T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        let ptr = unsafe { transmute::<*const c_void, *mut T>(foreign) };

        Ok(unsafe { &mut *ptr as &'a mut T })
    }
}
