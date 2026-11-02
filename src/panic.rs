use std::panic::{AssertUnwindSafe, catch_unwind};

/// Drops `value`, catching a panic in its destructor so that it goes no further than here.
/// The panic hook has already reported it by the time it is caught.
pub(crate) fn drop_caught<T>(value: T) {
    let _ = catch_unwind(AssertUnwindSafe(move || drop(value)));
}
