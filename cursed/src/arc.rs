use std::convert::Infallible;
use std::error::Error;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct ArcMarshaler<T: ?Sized>(PhantomData<T>);

impl<T> InputType for ArcMarshaler<T> {
    type Foreign = *mut T;
}

impl<T> ReturnType for ArcMarshaler<T> {
    type Foreign = *mut T;

    fn foreign_default() -> Self::Foreign {
        std::ptr::null_mut()
    }
}

impl<T> ToForeign<Arc<T>, *mut T> for ArcMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: Arc<T>) -> Result<*mut T, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as ToForeign<{ty}, {o}>>::to_foreign",
            ty = std::any::type_name::<T>(),
            o = "*mut c_void"
        );
        Ok(Arc::into_raw(local) as *mut _)
    }
}

impl<T> FromForeign<*mut T, Arc<T>> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *mut T) -> Result<Arc<T>, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as FromForeign<*mut std::ffi::c_void, T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { Arc::from_raw(foreign as *mut _) })
    }
}

impl<T: ?Sized> ToForeign<Result<Arc<T>, Box<dyn Error>>, *mut T> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(local: Result<Arc<T>, Box<dyn Error>>) -> Result<*mut T, Self::Error> {
        local.and_then(|x| Ok(Arc::into_raw(x) as *mut _))
    }
}