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
    type Foreign = *const [T];

    fn foreign_default() -> Self::Foreign {
        unsafe { std::mem::transmute::<[usize; 2], *const [T]>([0, 0]) }
    }
}

impl<T> ToForeign<Vec<T>, *const [T]> for VecMarshaler<T> {
    type Error = Infallible;

    fn to_foreign(vec: Vec<T>) -> Result<*const [T], Self::Error> {
        Ok(Box::into_raw(vec.into_boxed_slice()))
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
