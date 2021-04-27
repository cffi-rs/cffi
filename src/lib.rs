pub use cffi_impl::marshal;

#[cfg(feature = "url")]
mod url;

mod arc;
mod arc_ref;
mod bool;
mod box_ref;
mod boxed;
mod copy;
mod pathbuf;
mod str;
mod string;
mod unit;
mod vec;
mod vec_ref;

/// Exported functions for consumption via C API
pub mod ffi {
    pub use super::{string::cffi_string_free, vec::cffi_vec_free};
}

#[cfg(feature = "url")]
pub use self::url::UrlMarshaler;

pub use self::bool::BoolMarshaler;
pub use self::pathbuf::PathBufMarshaler;
pub use self::str::StrMarshaler;
pub use self::vec::VecMarshaler;
pub use arc::ArcMarshaler;
pub use arc_ref::ArcRefMarshaler;
pub use box_ref::BoxRefMarshaler;
pub use boxed::BoxMarshaler;
pub use copy::CopyMarshaler;
pub use string::StringMarshaler;
pub use unit::UnitMarshaler;
pub use vec_ref::VecRefMarshaler;

use std::{io, marker::PhantomData};

pub type ErrCallback = Option<extern "C" fn(*const u8, usize)>;
pub type RetCallback<T> = Option<extern "C" fn(T)>;

pub trait ReturnType {
    type Foreign;
    type ForeignTraitObject;

    fn foreign_default() -> Self::Foreign;
    fn foreign_default_trait_object() -> Self::ForeignTraitObject {
        unimplemented!();
    }
}

pub trait InputType {
    // type Local;
    type Foreign;
    type ForeignTraitObject;

    // fn local_default() -> Self::Local;
}

pub trait ToForeign<Local, Foreign>: Sized {
    type Error;
    fn to_foreign(_: Local) -> Result<Foreign, Self::Error>;
}

pub trait ToForeignTraitObject<Local: ?Sized, Foreign: ?Sized> {
    type Error;
    fn to_foreign_trait_object(_: Local) -> Result<crate::TraitObject<Foreign>, Self::Error>;
}

pub trait FromForeign<Foreign, Local>: Sized {
    type Error;
    unsafe fn from_foreign(_: Foreign) -> Result<Local, Self::Error>;
}

#[inline(always)]
pub fn null_ptr_error() -> Box<io::Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, "null pointer"))
}

// Magical catch-all implementation for `Result<Local, Error>`.
// impl<T, Foreign, Local> ToForeign<Result<Local, T::Error>, Foreign> for T
// where
//     T: ToForeign<Local, Foreign>,
// {
//     type Error = T::Error;

//     fn to_foreign(result: Result<Local, T::Error>) -> Result<Foreign, Self::Error> {
//         match result {
//             Ok(v) => <Self as ToForeign<Local, Foreign>>::to_foreign(v),
//             Err(e) => Err(e),
//         }
//     }
// }

#[repr(C)]
pub struct Slice<T: ?Sized> {
    pub data: *mut T,
    pub len: usize,
}

impl<T> Slice<T> {
    unsafe fn cast<U>(self) -> Slice<U> {
        std::mem::transmute::<Slice<T>, Slice<U>>(self)
    }
}

impl<T> std::default::Default for Slice<T> {
    fn default() -> Self {
        Slice {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }
}

impl<T> std::fmt::Debug for Slice<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .debug_struct(&format!("Slice<{}>", std::any::type_name::<T>()))
            .field("data", &self.data.cast::<std::ffi::c_void>())
            .field("len", &self.len)
            .finish()
    }
}

impl<T> AsRef<[T]> for Slice<T> {
    fn as_ref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.data as _, self.len) }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TraitObject<T: ?Sized> {
    pub data: *mut (),
    pub vtable: *mut (),
    pub ty: PhantomData<T>,
}

#[macro_export]
macro_rules! trait_object {
    ($input:path : $ty:ty) => {
        std::mem::transmute_copy::<_, $crate::TraitObject<$ty>>(&$input)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
