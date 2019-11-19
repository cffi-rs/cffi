use std::{
    convert::Infallible,
    error::Error,
    ffi::{c_void, CStr, CString},
    io,
    marker::PhantomData,
    sync::Arc,
};

// Re-export for certain classes of crime
pub use ctor::ctor;

#[cfg(feature = "url")]
mod url;

mod arc;
mod arc_ref;
mod bool;
mod box_ref;
mod boxed;
mod copy;
mod path;
mod str;
mod string;
mod unit;
mod vec;
mod vec_ref;

#[cfg(feature = "url")]
pub use self::url::UrlMarshaler;

pub use self::bool::BoolMarshaler;
pub use self::path::PathMarshaler;
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

pub type ErrCallback = Option<extern "C" fn(*const c_void)>;
pub type RetCallback<T> = Option<extern "C" fn(T)>;

pub trait ReturnType {
    type Foreign;

    fn foreign_default() -> Self::Foreign;
}

pub trait InputType {
    // type Local;
    type Foreign;

    // fn local_default() -> Self::Local;
}

pub trait ToForeign<Local, Foreign>: Sized {
    type Error;
    fn to_foreign(_: Local) -> Result<Foreign, Self::Error>;
}

pub trait FromForeign<Foreign, Local>: Sized {
    type Error;
    fn from_foreign(_: Foreign) -> Result<Local, Self::Error>;
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

#[cfg(test)]
mod tests {
    #[test]
    fn test() {

    }
}
