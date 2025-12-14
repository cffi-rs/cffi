use std::convert::Infallible;
use std::error::Error;
use std::marker::PhantomData;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, Slice, ToForeign};
pub struct VecMarshaler<T>(PhantomData<T>);

impl<T> InputType for VecMarshaler<T> {
    type Foreign = Slice<T>;
    type ForeignTraitObject = ();
}

impl<T> ReturnType for VecMarshaler<T> {
    type Foreign = Slice<T>;
    type ForeignTraitObject = ();

    fn foreign_default() -> Self::Foreign {
        Slice {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }
}

impl<T> ToForeign<Vec<T>, Slice<T>> for VecMarshaler<T> {
    type Error = Infallible;

    fn to_foreign(mut vec: Vec<T>) -> Result<Slice<T>, Self::Error> {
        vec.shrink_to_fit();
        let len = vec.len();
        let data = vec.as_mut_ptr();
        std::mem::forget(vec);

        let raw = Slice { data, len };
        tracing::debug!("Ptr: {:?}", raw);
        Ok(raw)
    }
}

impl<T> FromForeign<Slice<T>, Vec<T>> for VecMarshaler<T> {
    type Error = Box<dyn Error>;

    unsafe fn from_foreign(ptr: Slice<T>) -> Result<Vec<T>, Self::Error> {
        if ptr.data.is_null() {
            return Err(null_ptr_error());
        }

        let vec = unsafe { Vec::from_raw_parts(ptr.data, ptr.len, ptr.len) };

        Ok(vec)
    }
}

impl<T> ToForeign<Result<Vec<T>, Box<dyn Error>>, Slice<T>> for VecMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(local: Result<Vec<T>, Box<dyn Error>>) -> Result<Slice<T>, Self::Error> {
        local.map(|x| VecMarshaler::to_foreign(x).unwrap())
    }
}

/// # Safety
/// The slice must have been created by this library and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cffi_vec_free(slice: Slice<libc::c_void>) {
    unsafe { Vec::from_raw_parts(slice.data, slice.len, slice.len) };
}
