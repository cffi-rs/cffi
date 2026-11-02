use std::convert::Infallible;
use std::error::Error;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::Arc;

use crate::TraitObject;

use super::null_ptr_error;
use super::panic::drop_caught;
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
            ty: PhantomData,
        }
    }
}

impl<T: ?Sized> ToForeign<Arc<T>, *const T> for ArcMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: Arc<T>) -> Result<*const T, Self::Error> {
        tracing::debug!(
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
        tracing::debug!(
            "<ArcMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { Arc::from_raw(foreign as *const _) })
    }
}

impl<T: ?Sized> ToForeign<Result<Arc<T>, Box<dyn Error>>, *const T> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(local: Result<Arc<T>, Box<dyn Error>>) -> Result<*const T, Self::Error> {
        local.map(|x| Arc::into_raw(x) as *const _)
    }
}

impl<T: ?Sized> ArcMarshaler<T> {
    /// Gives up a reference that `ArcMarshaler::<T>::to_foreign` handed out, dropping the value
    /// when no other reference is left.
    ///
    /// NULL is ignored, so the zero value a failed call returns can be freed like any other
    /// handle. A panic in `T`'s destructor is caught here and goes no further.
    ///
    /// A generic function cannot be exported to C, so export a free function for each handle
    /// type:
    ///
    /// ```rust
    /// use cffi::ArcMarshaler;
    ///
    /// pub struct Client {
    ///     pub name: String,
    /// }
    ///
    /// #[unsafe(no_mangle)]
    /// pub unsafe extern "C" fn Example_Client_free(client: *const Client) {
    ///     unsafe { ArcMarshaler::free(client) }
    /// }
    /// ```
    ///
    /// # Safety
    /// The pointer must be NULL or have come from `ArcMarshaler::<T>::to_foreign`, and the
    /// reference it stands for must not already have been given up.
    pub unsafe fn free(foreign: *const T) {
        if foreign.is_null() {
            return;
        }

        drop_caught(unsafe { Arc::from_raw(foreign) });
    }
}
