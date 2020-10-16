use std::convert::Infallible;
use std::error::Error;
use std::marker::PhantomData;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

/// The `Box` marshaler is the catch-all just-throw-it-on-the-heap opaque pointer solution.
///
/// It supports the following modes of operation:
///
/// ### To the foreign interface:
///
///   - `T` → `*const/mut T`
///   - `Box<T>` → `*const/*mut T`
///
/// ### From the foreign interface:
///
///   - `*const/mut T` → `Box<T>` (owned)
///   - `*const T` → `&T` (ref)
///   - `*mut T` → `&mut T` (mut ref)
///
/// ## Freeing `T`
///
/// Your foreign code should ensure that they call `BoxMarshaler::<*mut/const T, Box<T>>::from_foreign`,
/// which will allow you to consume the boxed `T` and allow it to drop as per Rust's usual rules.
///
/// ## Example
///
/// ```rust
/// use cffi::{BoxMarshaler, FromForeign, ToForeign};
///
/// struct Something {
///     data: Vec<u8>
/// }
///
/// fn demo() {
///     let something = Something { data: vec![1, 3, 55] };
///
///     // BoxMarshaler::to_foreign is Infallible
///     let ptr: *const Something = BoxMarshaler::to_foreign(something).unwrap();
///
///     /* send `ptr` over ffi, process it in some way, etc */
///
///     // This isn't infallible though, checks for null pointers.
///     let boxed: Box<Something> = match BoxMarshaler::from_foreign(ptr) {
///         Ok(v) => v,
///         Err(e) => panic!("!")
///     };
///
///     // Let the boxed item drop and it is freed. :)
/// }
/// ```
pub struct BoxMarshaler<T: ?Sized>(PhantomData<T>);

impl<T> InputType for BoxMarshaler<T> {
    type Foreign = *const T;
}

impl<T> ReturnType for BoxMarshaler<T> {
    type Foreign = *const T;

    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl<T> ToForeign<Box<T>, *const T> for BoxMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: Box<T>) -> Result<*const T, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as ToForeign<{ty}, {o}>>::to_foreign",
            ty = std::any::type_name::<T>(),
            o = "*const T"
        );
        Ok(Box::into_raw(local) as *const _ as *const _)
    }
}

impl<T: ?Sized> ToForeign<Result<Box<T>, Box<dyn Error>>, *const T> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(local: Result<Box<T>, Box<dyn Error>>) -> Result<*const T, Self::Error> {
        local.and_then(|x| Ok(Box::into_raw(x) as *const _ as *const _))
    }
}

// impl<T> ToForeign<Box<T>, *mut T> for BoxMarshaler<T> {
//     type Error = Infallible;

//     #[inline(always)]
//     fn to_foreign(local: Box<T>) -> Result<*mut T, Self::Error> {
//         Ok(Box::into_raw(local as *mut _ as *mut _)
//     }
// }

impl<T> FromForeign<*const T, Box<T>> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: *const T) -> Result<Box<T>, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as FromForeign<*const T, T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(Box::from_raw(foreign as *mut _))
    }
}

// impl<T: ?Sized> FromForeign<*mut T, Box<T>> for BoxMarshaler<T> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     unsafe fn from_foreign(foreign: *mut T) -> Result<Box<T>, Self::Error> {
//         log::debug!("<BoxMarshaler<{ty}> as FromForeign<*mut {ty}, &'a {ty}>>::from_foreign({:?})",
//             foreign,
//             ty = std::any::type_name::<T>()
//         );
//         if foreign.is_null() {
//             return Err(null_ptr_error());
//         }

//         Ok(unsafe { Box::from_raw(foreign) })
//     }
// }
//
// impl<'a, T: Clone> ToForeign<&'a T, *const T> for BoxMarshaler<T> {
//     type Error = Infallible;

//     #[inline(always)]
//     fn to_foreign(local: &'a T) -> Result<*const T, Self::Error> {
//         Ok(Box::into_raw(Box::new(local.clone())))
//     }
// }

// impl<'a, T: Clone> ToForeign<&'a T, *mut T> for BoxMarshaler<T> {
//     type Error = Infallible;

//     #[inline(always)]
//     fn to_foreign(local: &'a T) -> Result<*mut T, Self::Error> {
//         Ok(Box::into_raw(Box::new(local.clone())))
//     }
// }

// impl<T: ?Sized> ToForeign<Box<T>, *mut T> for BoxMarshaler<T> {
//     type Error = Infallible;

//     #[inline(always)]
//     fn to_foreign(local: Box<T>) -> Result<*mut T, Self::Error> {
//         Ok(Box::into_raw(local))
//     }
// }

// impl<'a, T: ?Sized> FromForeign<*mut T, &'a mut T> for BoxMarshaler<T> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     unsafe fn from_foreign(foreign: *mut T) -> Result<&'a mut T, Self::Error> {
//         if foreign.is_null() {
//             return Err(null_ptr_error());
//         }

//         Ok(unsafe { &mut *foreign })
//     }
// }
