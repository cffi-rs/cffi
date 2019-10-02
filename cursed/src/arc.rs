use std::marker::PhantomData;
use std::sync::Arc;
use std::error::Error;
use std::ffi::c_void;

use super::null_ptr_error;
use super::{FromForeign, ToForeign, ReturnType};

pub struct ArcMarshaler<T>(PhantomData<T>);

impl<T> FromForeign<*const c_void, Arc<T>> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(arc_ptr: *const c_void) -> Result<Arc<T>, Self::Error> {
        if arc_ptr.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { Arc::from_raw(arc_ptr.cast()) })
    }
}

impl<T> ToForeign<Arc<T>, *const c_void> for ArcMarshaler<T> {
    type Error = Arc<dyn Error>;

    #[inline(always)]
    fn to_foreign(arced: Arc<T>) -> Result<*const c_void, Self::Error> {
        Ok(Arc::into_raw(arced) as *const _ as *const _)
    }
}

