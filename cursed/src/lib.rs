use std::{
    borrow::Cow,
    convert::Infallible,
    error::Error,
    ffi::{CStr, CString},
    io,
    marker::PhantomData,
    sync::Arc,
};

#[macro_export]
macro_rules! throw {
    ($error:path, $ex:ident, $fallback:expr) => {{
        use std::default::Default;

        if let Some(callback) = $ex {
            let err = format!("{:?}", $error);
            let s = std::ffi::CString::new(err)
                .unwrap_or_else(|_| std::ffi::CString::new("<unknown>".to_string()).unwrap());
            callback(s.as_ptr());
        }

        $fallback
    }};

    ($error:path, $ex:ident) => {
        $crate::throw!($error, $ex, ())
    };
}

#[macro_export]
macro_rules! try_not_null {
    ($path:expr, $ex:ident, $fallback:expr) => {{
        match $path {
            Ok(v) => v,
            Err(e) => {
                return $crate::throw!(e, $ex, $fallback);
            }
        }
    }};

    ($path:path, $ex:ident) => {
        try_not_null!($path, $ex, ())
    };
}

pub type ErrCallback = Option<extern "C" fn(*const libc::c_char)>;

pub trait ToForeign<Local, Foreign>: Sized {
    type Error;
    fn to_foreign(_: Local) -> Result<Foreign, Self::Error>;
}

pub trait ToForeignResult<Local, Foreign>: Sized {
    type Error;
    fn to_foreign(result: Result<Local, Self::Error>) -> Result<Foreign, Self::Error>;
}

pub trait FromForeign<Foreign, Local>: Sized {
    type Error;
    fn from_foreign(_: Foreign) -> Result<Local, Self::Error>;
}

pub struct BoxMarshaler<T: ?Sized>(PhantomData<T>);

impl<T> ToForeign<T, *const T> for BoxMarshaler<T> {
    type Error = Infallible;
    fn to_foreign(local: T) -> Result<*const T, Self::Error> {
        Ok(Box::into_raw(Box::new(local)))
    }
}

impl<T> ToForeign<T, *mut T> for BoxMarshaler<T> {
    type Error = Infallible;
    fn to_foreign(local: T) -> Result<*mut T, Self::Error> {
        Ok(Box::into_raw(Box::new(local)))
    }
}

impl<T: ?Sized> ToForeign<Box<T>, *mut T> for BoxMarshaler<T> {
    type Error = Infallible;
    fn to_foreign(local: Box<T>) -> Result<*mut T, Self::Error> {
        Ok(Box::into_raw(local))
    }
}

impl<'a, T: ?Sized> FromForeign<*mut T, &'a mut T> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;
    fn from_foreign(foreign: *mut T) -> Result<&'a mut T, Self::Error> {
        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { &mut *foreign })
    }
}

impl<'a, T: ?Sized> FromForeign<*const T, &'a T> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;
    fn from_foreign(foreign: *const T) -> Result<&'a T, Self::Error> {
        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { &*foreign as &'a T })
    }
}

impl<'a, T: ?Sized> FromForeign<*mut T, Box<T>> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;
    fn from_foreign(foreign: *mut T) -> Result<Box<T>, Self::Error> {
        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { Box::from_raw(foreign) })
    }
}

#[inline(always)]
fn null_ptr_error() -> Box<io::Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, "null pointer"))
}

pub struct ArcMarshaler<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> FromForeign<*const T, Arc<T>> for ArcMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(arc_ptr: *const T) -> Result<Arc<T>, Self::Error> {
        if arc_ptr.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { Arc::from_raw(arc_ptr) })
    }
}

impl<T: ?Sized> ToForeign<Arc<T>, *const T> for ArcMarshaler<T> {
    type Error = Arc<dyn Error>;

    #[inline(always)]
    fn to_foreign(arced: Arc<T>) -> Result<*const T, Self::Error> {
        Ok(Arc::into_raw(arced))
    }
}

pub struct BoolMarshaler;

impl FromForeign<u8, bool> for BoolMarshaler {
    type Error = Infallible;

    #[inline(always)]
    fn from_foreign(i: u8) -> Result<bool, Self::Error> {
        Ok(i != 0)
    }
}

impl ToForeign<bool, u8> for BoolMarshaler {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn to_foreign(b: bool) -> Result<u8, Self::Error> {
        Ok(if b { 1 } else { 0 })
    }
}

pub struct StrMarshaler<'a>(&'a PhantomData<()>);

impl ToForeign<String, *const libc::c_char> for StrMarshaler<'_> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(string: String) -> Result<*const libc::c_char, Self::Error> {
        let c_str = std::ffi::CString::new(string)?;
        Ok(CString::into_raw(c_str))
    }
}

impl<'a> ToForeign<&'a str, *const libc::c_char> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(input: &'a str) -> Result<*const libc::c_char, Self::Error> {
        let c_str = CString::new(input)?;
        Ok(c_str.into_raw())
    }
}

impl<'a> FromForeign<*const libc::c_char, &'a str> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const libc::c_char) -> Result<&'a str, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CStr::from_ptr(key) }.to_str()?)
    }
}

impl<'a> FromForeign<*const libc::c_char, Cow<'a, str>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const libc::c_char) -> Result<Cow<'a, str>, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CStr::from_ptr(key) }.to_string_lossy())
    }
}

impl<'a> FromForeign<*mut libc::c_char, CString> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *mut libc::c_char) -> Result<CString, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CString::from_raw(key) })
    }
}

impl<'a> FromForeign<*mut libc::c_char, String> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *mut libc::c_char) -> Result<String, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CString::from_raw(key) }.into_string()?)
    }
}

/// Magical catch-all implementation for `Result<Local, Error>`.
impl<T, Foreign, Local> ToForeignResult<Local, Foreign> for T
where
    T: ToForeign<Local, Foreign>,
{
    type Error = T::Error;

    fn to_foreign(result: Result<Local, T::Error>) -> Result<Foreign, Self::Error> {
        match result {
            Ok(v) => <Self as ToForeign<Local, Foreign>>::to_foreign(v),
            Err(e) => Err(e),
        }
    }
}
