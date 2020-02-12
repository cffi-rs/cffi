use std::convert::Infallible;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct ArcMarshaler<T: ?Sized>(PhantomData<T>);

impl<T> InputType for ArcMarshaler<T> {
    type Foreign = *const T;
}

impl<T> ReturnType for ArcMarshaler<T> {
    type Foreign = *const T;

    fn foreign_default() -> Self::Foreign {
        std::ptr::null_mut()
    }
}

impl<T> ToForeign<Arc<T>, *const T> for ArcMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: Arc<T>) -> Result<*const T, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as ToForeign<{ty}, {o}>>::to_foreign",
            ty = std::any::type_name::<T>(),
            o = "*const c_void"
        );

        // let pinned_arc = local.pin();
        // let garbage = unsafe { std::mem::transmute::<Pin<Arc<T>>, *const c_void>(pinned_arc) };
        // // let pinned_ref = pinned_arc.get_ref();
        // // std::mem::forget(pinned_arc);

        // Ok(pinned_ref as *const _)
        Ok(Arc::into_raw(local))
    }
}

impl<T> FromForeign<*const T, Arc<T>> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: *const T) -> Result<Arc<T>, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(Arc::from_raw(foreign as *const _))
    }
}

impl<T: ?Sized> ToForeign<Result<Arc<T>, Box<dyn Error>>, *const T> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(local: Result<Arc<T>, Box<dyn Error>>) -> Result<*const T, Self::Error> {
        local.and_then(|x| Ok(Arc::into_raw(x) as *const _))
    }
}