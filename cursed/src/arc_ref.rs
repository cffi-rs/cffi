use std::error::Error;
use std::marker::PhantomData;
use std::mem::transmute;
use std::sync::Arc;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct ArcRefMarshaler<T>(PhantomData<T>);

impl<T> InputType for ArcRefMarshaler<T> {
    type Foreign = *const Arc<T>;
}

impl<'a, T> FromForeign<*const Arc<T>, &'a Arc<T>> for ArcRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const Arc<T>) -> Result<&'a Arc<T>, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as FromForeign<*const Arc<T>, &'a Arc<T>>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        // let ptr = unsafe { transmute::<*const T, *const T>(foreign) };

        Ok(unsafe { &*foreign as &'a Arc<T> })
    }
}

// impl<'a, T> FromForeign<*const T, &'a mut Arc<T>> for ArcRefMarshaler<T> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     fn from_foreign(foreign: *const T) -> Result<&'a mut T, Self::Error> {
//         log::debug!(
//             "<ArcMarshaler<{ty}> as FromForeign<*const T, &'a mut T>>::from_foreign({:?})",
//             foreign,
//             ty = std::any::type_name::<T>()
//         );

//         if foreign.is_null() {
//             return Err(null_ptr_error());
//         }

//         let ptr = unsafe { transmute::<*const T, *mut T>(foreign) };

//         Ok(unsafe { &mut *ptr as &'a mut T })
//     }
// }
