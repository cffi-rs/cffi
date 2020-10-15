use std::convert::Infallible;
use std::error::Error;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use libc::wchar_t;

use crate::null_ptr_error;
use crate::vec::VecMarshaler;
use crate::{FromForeign, InputType, ReturnType, Slice, ToForeign};

pub struct PathBufMarshaler;

#[cfg(unix)]
impl InputType for PathBufMarshaler {
    type Foreign = Slice<u8>;
}

#[cfg(unix)]
impl ReturnType for PathBufMarshaler {
    type Foreign = Slice<u8>;

    #[inline(always)]
    fn foreign_default() -> Self::Foreign {
        Slice::default()
    }
}

#[cfg(unix)]
impl FromForeign<Slice<u8>, PathBuf> for PathBufMarshaler {
    type Error = Box<dyn Error>;

    #[inline(always)]
    unsafe fn from_foreign(foreign: Slice<u8>) -> Result<PathBuf, Self::Error> {
        use std::os::unix::ffi::OsStrExt;

        if foreign.data.is_null() {
            return Err(null_ptr_error());
        }

        let slice: &[u8] = std::slice::from_raw_parts(foreign.data, foreign.len);
        let os_str = std::ffi::OsStr::from_bytes(slice);
        Ok(Path::new(os_str).to_path_buf())
    }
}

#[cfg(unix)]
impl ToForeign<PathBuf, Slice<u8>> for PathBufMarshaler {
    type Error = Infallible;

    #[inline(always)]
    fn to_foreign(input: PathBuf) -> Result<Slice<u8>, Self::Error> {
        use std::os::unix::ffi::OsStringExt;

        let vec = input.into_os_string().into_vec();
        VecMarshaler::to_foreign(vec)
    }
}

#[cfg(unix)]
impl<E> ToForeign<Result<PathBuf, E>, Slice<u8>> for PathBufMarshaler {
    type Error = E;

    #[inline(always)]
    fn to_foreign(input: Result<PathBuf, E>) -> Result<Slice<u8>, Self::Error> {
        input.and_then(|x| Ok(PathBufMarshaler::to_foreign(x).unwrap()))
    }
}
