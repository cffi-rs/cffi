use std::error::Error;
use std::marker::PhantomData;

use super::null_ptr_error;
use super::{FromForeign, InputType};

pub struct BoxRefMarshaler<T>(PhantomData<T>);

impl<T> InputType for BoxRefMarshaler<T> {
    type Foreign = *mut T;
    type ForeignTraitObject = ();
}

// impl<'a, T> FromForeign<*mut Box<T>, &'a Box<T>> for BoxRefMarshaler<T> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     unsafe fn from_foreign(foreign: *mut Box<T>) -> Result<&'a Box<T>, Self::Error> {
//         log::debug!(
//             "<BoxMarshaler<{ty}> as FromForeign<*mut Box<T>, &'a Box<T>>>::from_foreign({:?})",
//             foreign,
//             ty = std::any::type_name::<T>()
//         );

//         if foreign.is_null() {
//             return Err(null_ptr_error());
//         }

//         Ok(unsafe { &*foreign as &'a Box<T> })
//     }
// }

impl<'a, T> FromForeign<*mut T, &'a T> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: *mut T) -> Result<&'a T, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as FromForeign<*mut Box<T>, &'a mut Box<T>>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        // let mut boxed = unsafe { Box::from_raw(foreign as *mut _ as *mut _) };
        // let ptr = &mut boxed as *mut _;
        // std::mem::forget(boxed);
        // // let ptr = unsafe { std::mem::transmute::<*mut T, *mut Box<T>>(foreign) };

        Ok(&*foreign)
    }
}
