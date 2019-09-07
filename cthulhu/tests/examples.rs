use cthulhu_macro::{call_with, InvokeParams};
use quote::quote;
use assert_tokens_eq::assert_tokens_eq;

#[test]
fn test_bool() {
    let res = call_with(
        InvokeParams::default(),
        quote! {
            fn foo(yes: bool) {}
        },
    )
    .unwrap();
    let expected = quote! {
        #[no_mangle]
        extern "C" fn foo(yes: u8, __exception: ::cursed::ErrCallback) {
            let yes = ::cursed::try_not_null!(::cursed::BoolMarshaler::from_foreign(yes), __exception);
            fn foo(yes: bool) {}
            foo(yes);
        }
    };
    assert_tokens_eq!(res, expected);
}

#[test]
fn test_u32() {
    let res = call_with(
        InvokeParams::default(),
        quote! {
            fn foo(num: u32) {}
        },
    )
    .unwrap();
    let expected = quote! {
        #[no_mangle]
        extern "C" fn foo(num: ::libc::c_uint) {
            fn foo(num: u32) {}
            foo(num);
        }
    };
    assert_tokens_eq!(res, expected);
}

#[test]
fn test_u32_return() {
    let res = call_with(
        InvokeParams::default(),
        quote! {
            fn foo(num: u32) -> u32 {
                num + 42
            }
        },
    )
    .unwrap();
    let expected = quote! {
        #[no_mangle]
        extern "C" fn foo(num: ::libc::c_uint) -> ::libc::c_uint {
            fn foo(num: u32) -> u32 {
                num + 42
            }
            foo(num)
        }
    };
    assert_tokens_eq!(res, expected);
}

#[test]
fn arc_str() {
    let res = call_with(
        InvokeParams::default(),
        quote! {
            fn foo(input: Arc<str>) {}
        },
    )
    .unwrap();
    let expected = quote! {
        #[no_mangle]
        extern "C" fn foo(input: *const ::libc::c_char, __exception: ::cursed::ErrCallback) {
            let input = ::cursed::try_not_null!(::cursed::ArcMarshaler<str>::from_foreign(input), __exception);
            fn foo(input: Arc<str>) {}
            foo(input);
        }
    };
    assert_tokens_eq!(res, expected);
}

#[test]
fn custom_json() {
    struct TestStruct {
        hello: u32,
    }

    let res = call_with(
        InvokeParams::default(),
        quote! {
            fn foo(
                #[marshal(CustomJsonMarshaler)]
                input: &TestStruct,

                #[marshal(CustomOtherMarshaler)]
                input2: &TestStruct
            ) -> bool {
                input == input2
            }
        },
    )
    .unwrap();
    let expected = quote! {
        #[no_mangle]
        extern "C" fn foo(
            input: *const ::libc::c_void,
            input2: *const ::libc::c_void,
            __exception: ::cursed::ErrCallback,
        ) -> u8 {
            let input = ::cursed::try_not_null!(CustomJsonMarshaler::from_foreign(input), __exception, u8);
            let input2 = ::cursed::try_not_null!(CustomOtherMarshaler::from_foreign(input2), __exception, u8);
            fn foo(input: &TestStruct, input2: &TestStruct) -> bool {
                input == input2
            }
            let result = foo(input, input2);
            match ::cursed::BoolMarshaler::to_foreign(result) {
                Ok(v) => v,
                Err(e) => ::cursed::throw!(e, u8),
            }
        }
    };
    assert_tokens_eq!(res, expected);
}

#[test]
fn return_marshaler() {
    let res = call_with(
        InvokeParams {
            return_marshaler: Some(
                syn::parse2(quote! { ::cursed::BoolMarshaler }).unwrap()
            )
        },
        quote! {
            fn foo(input: Cow<str>, input2: Cow<str>) -> bool {
                input == input2
            }
        },
    ).unwrap();

    let expected = quote! {
        #[no_mangle]
        extern "C" fn foo(
            input: *const ::libc::c_char,
            input2: *const ::libc::c_char,
            __exception: ::cursed::ErrCallback,
        ) -> u8 {
            let input = ::cursed::try_not_null!(
                ::cursed::StrMarshaler::from_foreign(input),
                __exception,
                u8
            );
            let input2 = ::cursed::try_not_null!(
                ::cursed::StrMarshaler::from_foreign(input2),
                __exception,
                u8
            );
            fn foo(input: Cow<str>, input2: Cow<str>) -> bool {
                input == input2
            }
            let result = foo(input, input2);
            match ::cursed::BoolMarshaler::to_foreign(result) {
                Ok(v) => v,
                Err(e) => ::cursed::throw!(e, u8)
            }
        }
    };

    assert_tokens_eq!(res, expected);
}
