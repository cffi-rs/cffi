use std::{
    borrow::Cow,
    convert::Infallible,
    error::Error,
    ffi::{CStr, CString},
    io,
    marker::PhantomData,
    sync::Arc,
};

pub trait ReturnType {
    type Foreign;

    fn foreign_default() -> Self::Foreign;
}

impl ReturnType for StrMarshaler<'_> {
    type Foreign = *const std::ffi::c_void;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

impl ReturnType for StringMarshaler {
    type Foreign = *const std::ffi::c_void;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

pub type ErrCallback = Option<extern "C" fn(*const std::ffi::c_void)>;

pub trait ToForeign<Local, Foreign>: Sized {
    type Error;
    fn to_foreign(_: Local) -> Result<Foreign, Self::Error>;
}

pub trait FromForeign<Foreign, Local>: Sized {
    type Error;
    fn from_foreign(_: Foreign) -> Result<Local, Self::Error>;
}

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
/// use cursed::{BoxMarshaler, FromForeign, ToForeign};
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
pub struct BoxMarshaler<T>(PhantomData<T>);

impl<T> ReturnType for BoxMarshaler<T> {
    type Foreign = *const std::ffi::c_void;

    fn foreign_default() -> Self::Foreign {
        std::ptr::null()
    }
}

pub struct BoxRefMarshaler<T>(PhantomData<T>);

impl<T> ToForeign<T, *const libc::c_void> for BoxMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: T) -> Result<*const libc::c_void, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as ToForeign<{ty}, {o}>>::to_foreign",
            ty = std::any::type_name::<T>(),
            o = "*const libc::c_void"
        );
        Ok(Box::into_raw(Box::new(local)) as *const _ as *const _)
    }
}

impl<T> ToForeign<T, *mut libc::c_void> for BoxMarshaler<T> {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(local: T) -> Result<*mut libc::c_void, Self::Error> {
        Ok(Box::into_raw(Box::new(local)) as *mut _ as *mut _)
    }
}

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
//     fn from_foreign(foreign: *mut T) -> Result<&'a mut T, Self::Error> {
//         if foreign.is_null() {
//             return Err(null_ptr_error());
//         }

//         Ok(unsafe { &mut *foreign })
//     }
// }

impl<'a, T> FromForeign<*const std::ffi::c_void, &'a T> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const std::ffi::c_void) -> Result<&'a T, Self::Error> {
        log::debug!("<BoxMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, &'a T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        let ptr = unsafe { std::mem::transmute::<*const std::ffi::c_void, *const T>(foreign) };

        Ok(unsafe { &*ptr as &'a T })
    }
}

impl<'a, T> FromForeign<*const std::ffi::c_void, &'a mut T> for BoxRefMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const std::ffi::c_void) -> Result<&'a mut T, Self::Error> {
        log::debug!("<BoxMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, &'a mut T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        let ptr = unsafe { std::mem::transmute::<*const std::ffi::c_void, *mut T>(foreign) };

        Ok(unsafe { &mut *ptr as &'a mut T })
    }
}

// impl<T: ?Sized> FromForeign<*mut T, Box<T>> for BoxMarshaler<T> {
//     type Error = Box<dyn Error>;

//     #[inline(always)]
//     fn from_foreign(foreign: *mut T) -> Result<Box<T>, Self::Error> {
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

impl<T> FromForeign<*const std::ffi::c_void, T> for BoxMarshaler<T> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(foreign: *const std::ffi::c_void) -> Result<T, Self::Error> {
        log::debug!(
            "<BoxMarshaler<{ty}> as FromForeign<*const std::ffi::c_void, T>>::from_foreign({:?})",
            foreign,
            ty = std::any::type_name::<T>()
        );

        if foreign.is_null() {
            return Err(null_ptr_error());
        }

        Ok(*unsafe { Box::from_raw(foreign as *mut _) })
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

impl<T: ?Sized> ToForeign<Arc<T>, *const libc::c_void> for ArcMarshaler<T> {
    type Error = Arc<dyn Error>;

    #[inline(always)]
    fn to_foreign(arced: Arc<T>) -> Result<*const libc::c_void, Self::Error> {
        Ok(Arc::into_raw(arced) as *const _ as *const _)
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
pub struct StringMarshaler;

impl ToForeign<String, *const std::ffi::c_void> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(string: String) -> Result<*const std::ffi::c_void, Self::Error> {
        let c_str = std::ffi::CString::new(string)?;
        Ok(CString::into_raw(c_str).cast())
    }
}

impl<'a> ToForeign<&'a str, *const std::ffi::c_void> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn to_foreign(input: &'a str) -> Result<*const std::ffi::c_void, Self::Error> {
        let c_str = CString::new(input)?;
        Ok(c_str.into_raw().cast())
    }
}

impl<'a> FromForeign<*const std::ffi::c_void, &'a str> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const std::ffi::c_void) -> Result<&'a str, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CStr::from_ptr(key.cast()) }.to_str()?)
    }
}

impl<'a> FromForeign<*const std::ffi::c_void, Option<&'a str>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const std::ffi::c_void) -> Result<Option<&'a str>, Self::Error> {
        if key.is_null() {
            return Ok(None);
        }

        Ok(Some(unsafe { CStr::from_ptr(key.cast()) }.to_str()?))
    }
}

impl<'a> FromForeign<*const std::ffi::c_void, Cow<'a, str>> for StrMarshaler<'a> {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const std::ffi::c_void) -> Result<Cow<'a, str>, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CStr::from_ptr(key.cast()) }.to_string_lossy())
    }
}

impl<'a> FromForeign<*mut std::ffi::c_void, CString> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *mut std::ffi::c_void) -> Result<CString, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CString::from_raw(key.cast()) })
    }
}

impl<'a> FromForeign<*const std::ffi::c_void, String> for StringMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    fn from_foreign(key: *const std::ffi::c_void) -> Result<String, Self::Error> {
        if key.is_null() {
            return Err(null_ptr_error());
        }

        Ok(unsafe { CString::from_raw(key as *mut _) }.into_string()?)
    }
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
