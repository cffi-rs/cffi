use std::convert::Infallible;
use std::error::Error;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct VecMarshaler<T>(PhantomData<T>);

impl<T> InputType for VecMarshaler<T> {
    type Foreign = *const [T];
}

impl<T> ReturnType for VecMarshaler<T> {
    type Foreign = *const c_void;

    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl<T> ToForeign<Vec<T>, *const c_void> for VecMarshaler<T> {
    type Error = Infallible;

    fn to_foreign(vec: Vec<T>) -> Result<*const c_void, Self::Error> {
        Ok(Box::into_raw(vec.into_boxed_slice()) as *const _)
    }
}

impl<T> FromForeign<*const [T], Vec<T>> for VecMarshaler<T> {
    type Error = Box<dyn Error>;

    fn from_foreign(ptr: *const [T]) -> Result<Vec<T>, Self::Error> {
        if ptr.is_null() {
            return Err(null_ptr_error());
        }

        // let ptr = unsafe { std::mem::transmute::<*const c_void, *mut [T]>(ptr) };
        let boxed: Box<[T]> = unsafe { Box::from_raw(ptr as *mut _) };

        Ok(boxed.into_vec())
    }
}
