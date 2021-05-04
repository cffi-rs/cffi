use std::convert::Infallible;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use std::mem::MaybeUninit;

use crate::TraitObject;

use super::null_ptr_error;
use super::{FromForeign, InputType, ReturnType, ToForeign};

pub struct ArcMarshaler<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> InputType for ArcMarshaler<T> {
    type Foreign = *const T;
    type ForeignTraitObject = ();
}

impl<T: ?Sized> ReturnType for ArcMarshaler<T> {
    type Foreign = *const T;
    type ForeignTraitObject = TraitObject<T>;

    fn foreign_default() -> Self::Foreign {
        // This is not UB so long as it is only called when T is not a trait object. This is currently guaranteed by the generator.
        // Whether or not this is UB when T is a trait object is still an open question, see https://github.com/rust-lang/unsafe-code-guidelines/issues/166
        // (Although if they do make it UB they will completely break many assumptions with FFI, so they better not)
        unsafe { MaybeUninit::zeroed().assume_init() }
    }

    fn foreign_default_trait_object() -> Self::ForeignTraitObject {
        TraitObject {
            data: std::ptr::null_mut(),
            vtable: std::ptr::null_mut(),
            ty: PhantomData
        }
    }
}

impl<T: ?Sized> ToForeign<Arc<T>, *const T> for ArcMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: Arc<T>) -> Result<*const T, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as ToForeign<{ty}, {o}>>::to_foreign",
            ty = std::any::type_name::<T>(),
            o = "*const c_void"
        );

        // let pinned_arc = local.pin();
        // let garbage = unsafe { std::mem::transmute::<Pin<Arc<T>>, *const c_void>(pinned_arc) };
        // // let pinned_ref = pinned_arc.get_ref();
        // // std::mem::forget(pinned_arc);

        // Ok(pinned_ref as *const _)
        Ok(Arc::into_raw(local))
    }
}

impl<T: ?Sized> FromForeign<*const T, Arc<T>> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: *const T) -> Result<Arc<T>, Self::Error> {
        log::debug!(
            "<ArcMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(Arc::from_raw(foreign as *const _))
    }
}

impl<T: ?Sized> ToForeign<Result<Arc<T>, Box<dyn Error>>, *const T> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(local: Result<Arc<T>, Box<dyn Error>>) -> Result<*const T, Self::Error> {
        local.and_then(|x| Ok(Arc::into_raw(x) as *const _))
    }
}
