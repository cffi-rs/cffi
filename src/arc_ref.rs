use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::TraitObject;

use super::null_ptr_error;
use super::{FromForeign, InputType};

pub struct ArcRefMarshaler<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> InputType for ArcRefMarshaler<T> {
    type Foreign = *const T;
    type ForeignTraitObject = TraitObject<T>;
}

impl<'a, T: ?Sized> FromForeign<*const T, Arc<T>> for ArcRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: *const T) -> Result<Arc<T>, Self::Error> {
        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        let arc = Arc::from_raw(foreign);
        let cloned = Arc::clone(&arc);
        let _x = Arc::into_raw(arc);

        Ok(cloned)
    }
}

// impl<'a, T> FromForeign<*const T, &'a mut Arc<T>> for ArcRefMarshaler<T> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     unsafe fn from_foreign(foreign: *const T) -> Result<&'a mut T, Self::Error> {
//         log::debug!(
//             "<ArcMarshaler<{ty}> as FromForeign<*const T, &'a mut T>>::from_foreign({:?})",
//             foreign,
//             ty = std::any::type_name::<T>()
//         );

//         if foreign.is_null_mut() {
//             return Err(null_ptr_error());
//         }

//         let ptr = unsafe { transmute::<*const T, *mut T>(foreign) };

//         Ok(unsafe { &mut *ptr as &'a mut T })
//     }
// }
