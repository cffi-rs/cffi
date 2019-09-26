use assert_tokens_eq::assert_tokens_eq;
use cthulhu_macro::{call_with, InvokeParams};
use quote::quote;

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
            return_marshaler: Some(syn::parse2(quote! { ::cursed::BoolMarshaler }).unwrap()),
            prefix: None,
        },
        quote! {
            fn foo(input: Cow<str>, input2: Cow<str>) -> bool {
                input == input2
            }
        },
    )
    .unwrap();

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

#[test]
fn fuu() {
    let res = call_with(
        InvokeParams {
            return_marshaler: Some(syn::parse2(quote! { BoxFileResultMarshaler }).unwrap()),
            prefix: None,
        },
        quote! {
            pub fn box_file_open(path: Cow<str>) -> std::io::Result<BoxFile> {
                BoxFile::open(&*path)
            }
        },
    )
    .unwrap();

    let expected = quote! {
        #[no_mangle]
    };

    assert_tokens_eq!(res, expected);
}

#[test]
fn impl_life() {
    let res = call_with(
        InvokeParams { return_marshaler: None, prefix: Some("ex_pref_".to_string()) },
        quote! {
            impl Something {
                #[marshal(::cursed::BoxMarshaler)]
                pub fn new(item: Cow<str>) -> Something {
                    Something { item }
                }

                fn some_internal_function(foo: u8) {

                }
                
                pub fn value(&self, value: u8) {

                }

                pub fn value_ref(&self, value: Cow<str>) {
                    
                }

                pub fn value_ref_ret(&self, value: Cow<str>) -> u32 {
                    
                }

                #[marshal(::cursed::BoxMarshaler)]
                pub fn value_ref_ret_owned(&self, value: Cow<str>) -> Something {
                    
                }
                
                // pub fn value_ref_mut(&mut self, value: &mut u8) {
                    
                // }

                pub fn act_upon_ref(&self) {

                }

                pub fn act_upon_consume(self) {

                }

                pub fn act_upon_mut_ref(&mut self) {

                }

                pub fn do_something_static() {

                }
            }
        },
    )
    .unwrap();

    let expected = quote! {
        #[no_mangle]
        pub extern "C" fn ex_pref_something_free(
            __handle: *mut ::libc::c_void,
            __exception: ::cursed::ErrCallback,
        ) {
            let _: Box<Something> = ::cursed::try_not_null!(
                ::cursed::BoxMarshaler::from_foreign(__handle),
                __exception
            );
            log::debug!("`{}` has consumed this handle; do not reuse it!", stringify!(ex_pref_something_free));
            unsafe { *__handle = std::ptr::null_mut(); }
        }

        #[no_mangle]
        pub extern "C" fn ex_pref_something_new(
            item: *const ::libc::c_char,
            __exception: ::cursed::ErrCallback
        ) -> *mut ::libc::c_void {
            let item: Cow<str> = ::cursed::try_not_null!(
                ::cursed::StrMarshaler::from_foreign(item),
                __exception,
                std::ptr::null_mut()
            );
            let result = Something::new(item);
            match ::cursed::BoxMarshaler::to_foreign(result) {
                Ok(v) => v,
                Err(e) => ::cursed::throw!(e, __exception, std::ptr::null_mut())
            }
        }

        #[no_mangle]
        pub extern "C" fn ex_pref_something_act_upon_ref(__handle: *const ::libc::c_void, __exception: ::cursed::ErrCallback) {
            let __handle: &Something = ::cursed::try_not_null!(
                ::cursed::BoxMarshaler::from_foreign(__handle),
                __exception
            );
            Something::act_upon_ref(__handle);
        }

        #[no_mangle]
        pub extern "C" fn ex_pref_something_act_upon_consume(__handle: *mut ::libc::c_void, __exception: ::cursed::ErrCallback) {
            let __handle: Something = ::cursed::try_not_null!(
                ::cursed::BoxMarshaler::from_foreign(__handle),
                __exception
            );
            Something::act_upon_consume(__handle);
            log::debug!("act_upon has consumed this handle; do not reuse it!");
            unsafe { *__handle = std::ptr::null_mut(); }
        }

        #[no_mangle]
        pub extern "C" fn ex_pref_something_act_upon_mut_ref(__handle: *mut ::libc::c_void, __exception: ::cursed::ErrCallback) {
            let __handle: &mut Something = ::cursed::try_not_null!(
                ::cursed::BoxMarshaler::from_foreign(__handle),
                __exception
            );
            Something::act_upon_mut_ref(__handle);
        }

        #[no_mangle]
        pub extern "C" fn ex_pref_something_do_something_static() {
            Something::do_something_static();
        }
    };
    assert_tokens_eq!(expected, res);
}
