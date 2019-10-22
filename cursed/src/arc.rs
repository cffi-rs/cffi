use std::convert::Infallible;
use std::error::Error;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct ArcMarshaler<T>(PhantomData<T>);

impl<T> InputType for ArcMarshaler<T> {
    type Foreign = *const T;
}

impl<T> ReturnType for ArcMarshaler<T> {
    type Foreign = *const std::ffi::c_void;

    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl<T> ToForeign<Arc<T>, *const c_void> for ArcMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: Arc<T>) -> Result<*const c_void, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as ToForeign<{ty}, {o}>>::to_foreign",
            ty = std::any::type_name::<T>(),
            o = "*const c_void"
        );
        Ok(Arc::into_raw(local) as *const _ as *const _)
    }
}

impl<T> FromForeign<*const std::ffi::c_void, Arc<T>> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const std::ffi::c_void) -> Result<Arc<T>, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { Arc::from_raw(foreign as *mut _) })
    }
}
