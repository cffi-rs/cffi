use std::convert::Infallible;
use std::error::Error;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct VecRefMarshaler<T>(PhantomData<T>);

impl<T> InputType for VecRefMarshaler<T> {
    type Foreign = *const [T];
}

// impl<T> InputType for VecRefMarshaler<T>
// where
//     T: Default,
// {
//     type Local = T;

//     fn local_default() -> Self::Local {
//         T::default()
//     }
// }

impl<T> ReturnType for VecRefMarshaler<T> {
    type Foreign = *const [T];

    fn foreign_default() -> Self::Foreign {
        unsafe { std::mem::transmute::<[usize; 2], *const [T]>([0, 0]) }
    }
}
// impl<&'a T> ToForeign<&'a Vec<T>, *const c_void> for VecRefMarshaler<T> {
//     type Error = Infallible;

//     fn to_foreign(vec: Vec<T>) -> Result<*const c_void, Self::Error> {
//         Ok(Box::into_raw(vec.into_boxed_slice()) as *const _)
//     }
// }

impl<'a, T> FromForeign<*const [T], &'a [T]> for VecRefMarshaler<T> {
    type Error = Box<dyn Error>;

    fn from_foreign(ptr: *const [T]) -> Result<&'a [T], Self::Error> {
        log::debug!("vec ref ptr: {:?}", ptr);
        if ptr.is_null() {
            return Err(null_ptr_error());
        }

        // let ptr = unsafe { std::mem::transmute::<*const c_void, *mut [T]>(ptr) };
        // let boxed: Box<[T]> = unsafe { Box::from_raw(ptr as *mut _) };

        // Ok(boxed.into_vec())
        let slice = unsafe { &*ptr };
        log::debug!("Ref vec len: {}", slice.len());
        Ok(slice)
    }
}
