use std::error::Error;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct ArcMarshaler;

impl InputType for ArcMarshaler {
    type Foreign = *const c_void;
}

impl ReturnType for ArcMarshaler {
    type Foreign = *const std::ffi::c_void;

    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl<T> FromForeign<*const c_void, Arc<T>> for ArcMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(arc_ptr: *const c_void) -> Result<Arc<T>, Self::Error> {
        if arc_ptr.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { Arc::from_raw(arc_ptr.cast()) })
    }
}

impl<T> ToForeign<Arc<T>, *const c_void> for ArcMarshaler {
    type Error = Arc<dyn Error>;

    #[inline(always)]
    fn to_foreign(arced: Arc<T>) -> Result<*const c_void, Self::Error> {
        Ok(Arc::into_raw(arced) as *const _ as *const _)
    }
}
