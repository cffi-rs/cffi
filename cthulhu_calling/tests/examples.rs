use cthulhu_calling::call_with;

#[test]
fn test_bool() {
    let res = call_with(
        quote::quote! {},
        quote::quote! {
            fn foo(yes: bool) {}
        },
    )
    .unwrap();
    let expected = quote::quote! {
        extern "C" fn foo(yes: ::std::os::raw::c_char) {
            fn foo(yes: bool) {}
            unimplemented!()
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}

#[test]
fn test_u32() {
    let res = call_with(
        quote::quote! {},
        quote::quote! {
            fn foo(num: u32) {}
        },
    )
    .unwrap();
    let expected = quote::quote! {
        extern "C" fn foo(num: ::std::os::raw::c_uint) {
            fn foo(num: u32) {}
            unimplemented!()
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}

#[test]
fn cstr() {
    let res = call_with(
        quote::quote! {},
        quote::quote! {
            fn foo<'a>(input: &'a CStr) {}
        },
    )
    .unwrap();
    let expected = quote::quote! {
        extern "C" fn foo(input: *const ::std::os::raw::c_char) {
            fn foo<'a>(input: &'a CStr) {}
            unimplemented!()
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}

#[test]
fn arc_str() {
    let res = call_with(
        quote::quote! {},
        quote::quote! {
            fn foo(input: Arc<str>) {}
        },
    )
    .unwrap();
    let expected = quote::quote! {
        extern "C" fn foo(input: *const ::std::os::raw::c_char, input_len: ::libc::size_t) {
            fn foo(input: Arc<str>) {}
            unimplemented!()
        }
    };
    assert_eq!(res.to_string(), expected.to_string());
}
