use cthulhu_calling::call_with;

#[test]
fn test_bool() {
    let res = call_with(quote::quote! {}, quote::quote! {
        fn some_shit(yes: bool) {}
    }).unwrap();
    let expected = quote::quote! {
        extern "C" fn some_shit(yes: ::std::os::raw::c_char) {
            fn some_shit(yes: bool) {}
            unimplemented!()
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}

#[test]
fn test_u32() {
    let res = call_with(quote::quote! {}, quote::quote! {
        fn some_shit(num: u32) {}
    }).unwrap();
    let expected = quote::quote! {
        extern "C" fn some_shit(num: ::std::os::raw::c_uint) {
            fn some_shit(num: u32) {}
            unimplemented!()
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}

#[test]
fn cstr() {
    let res = call_with(quote::quote! {}, quote::quote! {
        fn some_shit<'a>(string_maybe: &'a CStr) {}
    }).unwrap();
    let expected = quote::quote! {
        extern "C" fn some_shit(le: *const ::std::os::raw::c_char, len: ::libc::size_t) {
            fn some_shit<'a>(string_maybe: &'a CStr) {}
            let (le, args) = oh_god(..);
            some_shit(le, args)
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}

#[test]
fn arc_str() {
    let res = call_with(quote::quote! {}, quote::quote! {
        fn some_shit(string_maybe: Arc<str>) {}
    }).unwrap();
    let expected = quote::quote! {
        extern "C" fn some_shit(le: *const ::std::os::raw::c_char, len: ::libc::size_t) {
            fn some_shit(string_maybe: Arc<str>) {}
            let (le, args) = oh_god(...);
            some_shit(le, args)
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}
