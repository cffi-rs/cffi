use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use cffi::{ArcMarshaler, FromForeign, Slice, ToForeign};

/// What a test's callbacks are handed, through the context pointer the test passes each call.
#[derive(Default)]
struct Calls {
    exceptions: Vec<String>,
    returned: Vec<String>,
}

impl Calls {
    fn context(&mut self) -> *mut c_void {
        (self as *mut Calls).cast()
    }
}

extern "C" fn record_exception(msg: *const u8, len: usize, context: *mut c_void) {
    let bytes = unsafe { std::slice::from_raw_parts(msg, len) };
    let calls = unsafe { &mut *context.cast::<Calls>() };
    calls
        .exceptions
        .push(std::str::from_utf8(bytes).unwrap().to_owned());
}

extern "C" fn record_return(slice: Slice<u8>, context: *mut c_void) {
    let value = std::str::from_utf8(slice.as_ref()).unwrap().to_owned();
    unsafe { cffi::ffi::cffi_string_free(slice) };
    let calls = unsafe { &mut *context.cast::<Calls>() };
    calls.returned.push(value);
}

fn str_slice(s: &str) -> Slice<u8> {
    Slice {
        data: s.as_ptr().cast_mut(),
        len: s.len(),
    }
}

fn take_string(slice: Slice<u8>) -> Option<String> {
    if slice.data.is_null() {
        return None;
    }

    let value = std::str::from_utf8(slice.as_ref()).unwrap().to_owned();
    unsafe { cffi::ffi::cffi_string_free(slice) };
    Some(value)
}

#[cffi::marshal(return_marshaler = cffi::StringMarshaler)]
pub extern "C" fn test_greet(#[marshal(cffi::StrMarshaler)] name: &str, fail: bool) -> String {
    if fail {
        panic!("no greeting for {name}");
    }

    format!("hello {name}")
}

#[cffi::marshal]
pub extern "C" fn test_panic_unit(#[marshal(cffi::StrMarshaler)] name: &str) {
    panic!("no greeting for {name}");
}

#[cffi::marshal]
pub extern "C" fn test_panic_any(#[marshal(cffi::StrMarshaler)] _name: &str) -> u32 {
    std::panic::panic_any(42u32)
}

#[cffi::marshal]
pub extern "C" fn test_halve(n: u32) -> u32 {
    if n % 2 == 1 {
        panic!("{n} is odd");
    }

    n / 2
}

#[cffi::marshal(callback, return_marshaler = cffi::StringMarshaler)]
pub extern "C" fn test_greet_by_callback(
    #[marshal(cffi::StrMarshaler)] name: &str,
    fail: bool,
) -> String {
    if fail {
        panic!("no greeting for {name}");
    }

    format!("hello {name}")
}

#[test]
fn a_panic_is_reported_and_returns_the_zero_value() {
    let mut calls = Calls::default();
    let result = test_greet(
        str_slice("Ánde"),
        1,
        Some(record_exception),
        calls.context(),
    );

    assert!(result.data.is_null());
    assert_eq!(result.len, 0);
    assert_eq!(calls.exceptions, [r#"Panic("no greeting for Ánde")"#]);
}

#[test]
fn a_call_that_does_not_panic_is_unchanged() {
    let mut calls = Calls::default();
    let result = test_greet(
        str_slice("Ánde"),
        0,
        Some(record_exception),
        calls.context(),
    );

    assert_eq!(take_string(result).as_deref(), Some("hello Ánde"));
    assert!(calls.exceptions.is_empty());
}

#[test]
fn each_call_hands_back_its_own_context() {
    let mut first = Calls::default();
    let mut second = Calls::default();

    test_panic_unit(str_slice("Ánde"), Some(record_exception), first.context());
    test_panic_unit(str_slice("Biret"), Some(record_exception), second.context());

    assert_eq!(first.exceptions, [r#"Panic("no greeting for Ánde")"#]);
    assert_eq!(second.exceptions, [r#"Panic("no greeting for Biret")"#]);
}

#[test]
fn a_panic_without_a_callback_given_returns_the_zero_value() {
    let result = test_greet(str_slice("Ánde"), 1, None, std::ptr::null_mut());

    assert!(result.data.is_null());
}

#[test]
fn a_panic_in_a_function_returning_nothing_is_reported() {
    let mut calls = Calls::default();
    test_panic_unit(str_slice("Ánde"), Some(record_exception), calls.context());

    assert_eq!(calls.exceptions, [r#"Panic("no greeting for Ánde")"#]);
}

#[test]
fn a_panic_with_a_value_that_is_not_a_string_is_reported() {
    let mut calls = Calls::default();
    let result = test_panic_any(str_slice("Ánde"), Some(record_exception), calls.context());

    assert_eq!(result, 0);
    assert_eq!(calls.exceptions, [r#"Panic("Box<dyn Any>")"#]);
}

#[test]
fn a_panic_in_a_function_without_an_exception_callback_returns_the_zero_value() {
    assert_eq!(test_halve(4), 2);
    assert_eq!(test_halve(3), 0);
}

#[test]
fn a_panic_in_a_function_with_a_return_callback_is_reported() {
    let mut calls = Calls::default();
    test_greet_by_callback(
        str_slice("Ánde"),
        0,
        Some(record_exception),
        Some(record_return),
        calls.context(),
    );
    assert_eq!(calls.returned, ["hello Ánde"]);
    assert!(calls.exceptions.is_empty());

    let mut calls = Calls::default();
    test_greet_by_callback(
        str_slice("Ánde"),
        1,
        Some(record_exception),
        Some(record_return),
        calls.context(),
    );
    assert!(calls.returned.is_empty());
    assert_eq!(calls.exceptions, [r#"Panic("no greeting for Ánde")"#]);
}

pub struct Thing {
    id: u32,
}

static DROPPED: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn dropped(id: u32) -> bool {
    DROPPED.lock().unwrap().contains(&id)
}

impl Drop for Thing {
    fn drop(&mut self) {
        DROPPED.lock().unwrap().push(self.id);
    }
}

#[cffi::marshal(return_marshaler = cffi::ArcMarshaler::<Thing>)]
pub extern "C" fn test_thing_new(id: u32) -> Arc<Thing> {
    Arc::new(Thing { id })
}

#[cffi::marshal]
pub extern "C" fn test_thing_id(
    #[marshal(cffi::ArcRefMarshaler::<Thing>)] thing: Arc<Thing>,
) -> u32 {
    thing.id
}

#[cffi::marshal]
pub extern "C" fn test_thing_take(
    #[marshal(cffi::ArcMarshaler::<Thing>)] thing: Arc<Thing>,
) -> u32 {
    thing.id
}

#[cffi::marshal]
pub extern "C" fn test_thing_panic(
    #[marshal(cffi::ArcRefMarshaler::<Thing>)] thing: Arc<Thing>,
) -> u32 {
    panic!("thing {}", thing.id);
}

/// # Safety
/// `thing` must be NULL or a handle from `test_thing_new` not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn test_thing_free(thing: *const Thing) {
    unsafe { ArcMarshaler::free(thing) }
}

#[test]
fn an_arc_handle_is_released_by_its_free_function() {
    let mut calls = Calls::default();
    let thing = test_thing_new(1, Some(record_exception), calls.context());
    assert!(!thing.is_null());

    assert_eq!(
        test_thing_id(thing, Some(record_exception), calls.context()),
        1
    );
    assert_eq!(
        test_thing_id(thing, Some(record_exception), calls.context()),
        1
    );
    assert!(!dropped(1));

    unsafe { test_thing_free(thing) };

    assert!(dropped(1));
    assert!(calls.exceptions.is_empty());
}

#[test]
fn a_null_arc_handle_is_freed_as_nothing() {
    unsafe { test_thing_free(std::ptr::null()) };
}

#[test]
fn a_null_arc_handle_argument_is_reported() {
    let mut calls = Calls::default();
    let null = std::ptr::null();

    assert_eq!(
        test_thing_id(null, Some(record_exception), calls.context()),
        0
    );
    assert_eq!(
        test_thing_take(null, Some(record_exception), calls.context()),
        0
    );

    assert_eq!(calls.exceptions.len(), 2);
    assert!(calls.exceptions.iter().all(|e| e.contains("null pointer")));
}

#[test]
fn a_panic_leaves_the_callers_handle_as_it_was() {
    let arc = Arc::new(Thing { id: 2 });
    let weak = Arc::downgrade(&arc);
    let thing = ArcMarshaler::to_foreign(arc).unwrap();

    let mut calls = Calls::default();
    assert_eq!(
        test_thing_panic(thing, Some(record_exception), calls.context()),
        0
    );
    assert_eq!(calls.exceptions, [r#"Panic("thing 2")"#]);
    assert_eq!(weak.strong_count(), 1);

    unsafe { test_thing_free(thing) };
    assert!(dropped(2));
}
