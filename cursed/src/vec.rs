use std::convert::Infallible;
use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::marker::PhantomData;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Slice<T> {
    pub data: *mut T,
    pub len: usize,
}

impl<T> std::default::Default for Slice<T> {
    fn default() -> Self {
        Slice { data: std::ptr::null_mut(), len: 0 }
    }
}

impl<T> fmt::Debug for Slice<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_struct(&format!("Slice<{}>", std::any::type_name::<T>()))
            .field("data", &self.data.cast::<std::ffi::c_void>())
            .field("len", &self.len)
            .finish()
    }
}

pub struct VecMarshaler<T>(PhantomData<T>);

impl<T> InputType for VecMarshaler<T> {
    type Foreign = Slice<T>;
}

impl<T> ReturnType for VecMarshaler<T> {
    type Foreign = Slice<T>;

    fn foreign_default() -> Self::Foreign {
        Slice { data: std::ptr::null_mut(), len: 0 }
    }
}

impl<T> ToForeign<Vec<T>, Slice<T>> for VecMarshaler<T> {
    type Error = Infallible;

    fn to_foreign(mut vec: Vec<T>) -> Result<Slice<T>, Self::Error> {
        vec.shrink_to_fit();
        let len = vec.len();
        let data = vec.as_mut_ptr();
        std::mem::forget(vec);

        // log::debug!("Vec len: {}", vec.len());
        // let raw = Box::into_raw(vec.into_boxed_slice());
        // log::debug!("Raw len: {}", unsafe { (*raw).len() });
        // log::debug!("???: {}", super::vec_ref::VecRefMarshaler::from_foreign(raw).unwrap().len());

        let raw = Slice { data, len };
        log::debug!("Ptr: {:?}", raw);
        Ok(raw)
    }
}

impl<T> FromForeign<Slice<T>, Vec<T>> for VecMarshaler<T> {
    type Error = Box<dyn Error>;

    fn from_foreign(ptr: Slice<T>) -> Result<Vec<T>, Self::Error> {
        if ptr.data.is_null() {
            return Err(null_ptr_error());
        }

        // let ptr = unsafe { std::mem::transmute::<*const c_void, *mut [T]>(ptr) };
        let vec = unsafe { Vec::from_raw_parts(ptr.data, ptr.len, ptr.len) };

        Ok(vec)
    }
}
