use std::error::Error;
use std::marker::PhantomData;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, Slice};

pub struct VecRefMarshaler<T>(PhantomData<T>);

impl<T> InputType for VecRefMarshaler<T> {
    type Foreign = Slice<T>;
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
    type Foreign = Slice<T>;

    fn foreign_default() -> Self::Foreign {
        Slice {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }
}
// impl<&'a T> ToForeign<&'a Vec<T>, *const c_void> for VecRefMarshaler<T> {
//     type Error = Infallible;

//     fn to_foreign(vec: Vec<T>) -> Result<*const c_void, Self::Error> {
//         Ok(Box::into_raw(vec.into_boxed_slice()) as *const _)
//     }
// }

impl<'a, T> FromForeign<Slice<T>, &'a [T]> for VecRefMarshaler<T> {
    type Error = Box<dyn Error>;

    unsafe fn from_foreign(slice: Slice<T>) -> Result<&'a [T], Self::Error> {
        log::debug!("vec ref ptr: {:?}", slice);
        if slice.data.is_null() {
            return Err(null_ptr_error());
        }

        let slice = std::slice::from_raw_parts(slice.data, slice.len);
        Ok(slice)
    }
}
