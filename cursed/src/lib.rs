use std::{
    convert::Infallible,
    error::Error,
    ffi::{c_void, CStr, CString},
    io,
    marker::PhantomData,
    sync::Arc,
};

mod arc;
mod bool;
mod box_ref;
mod boxed;
mod path;
mod str;
mod string;
mod unit;
mod vec;
mod vec_ref;

pub use self::bool::BoolMarshaler;
pub use self::path::PathMarshaler;
pub use self::str::StrMarshaler;
pub use self::vec::VecMarshaler;
pub use self::vec_ref::VecRefMarshaler;
pub use arc::ArcMarshaler;
pub use box_ref::BoxRefMarshaler;
pub use boxed::BoxMarshaler;
pub use string::StringMarshaler;
pub use unit::UnitMarshaler;

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
fn null_ptr_error() -> Box<io::Error> {
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
