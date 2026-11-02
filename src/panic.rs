use std::any::Any;
use std::error::Error;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A panic caught at the foreign boundary.
///
/// A function `#[cffi::marshal]` makes runs its work inside `catch_unwind`, so a panic never
/// unwinds into foreign code, which would abort the process. The panic is reported through the
/// exception callback as this error, formatted with `{:?}` like any other, and the function
/// returns the zero value of its return type.
///
/// Catching a panic needs the crate built with `panic = "unwind"`, the default. With
/// `panic = "abort"` there is nothing to catch and the process aborts as usual.
#[derive(Debug)]
pub struct Panic(String);

impl Panic {
    /// The message the panic was given, or `Box<dyn Any>` when it was given a value that is not
    /// a string, as the standard panic hook writes it.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl From<Box<dyn Any + Send>> for Panic {
    fn from(payload: Box<dyn Any + Send>) -> Self {
        let message = if let Some(message) = payload.downcast_ref::<&'static str>() {
            (*message).to_owned()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            "Box<dyn Any>".to_owned()
        };

        Panic(message)
    }
}

impl fmt::Display for Panic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "panicked: {}", self.0)
    }
}

impl Error for Panic {}

/// Drops `value`, catching a panic in its destructor so that it goes no further than here.
/// The panic hook has already reported it by the time it is caught.
pub(crate) fn drop_caught<T>(value: T) {
    let _ = catch_unwind(AssertUnwindSafe(move || drop(value)));
}
