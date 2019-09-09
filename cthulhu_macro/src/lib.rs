use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::Display;
use syn::{punctuated::Punctuated, spanned::Spanned, FnArg, Pat, PatType, Type};

fn collect_mappings_from_sig(
    sig: &mut syn::Signature,
) -> Result<Vec<(PatType, Option<syn::Path>)>, syn::Error> {
    if let Some(syn::FnArg::Receiver(item)) = sig.inputs.first() {
        return Err(syn::Error::new(item.span(), "Cannot support self"));
    }

    let attrs = sig
        .inputs
        .iter_mut()
        .filter_map(|x| {
            println!("XXX: {:#?}", &x);
            match x {
                syn::FnArg::Typed(t) => Some(t),
                _ => None,
            }
        })
        .map(|input| {
            let mut unhandled_attrs = vec![];
            println!("IN: {:?}", &input);

            std::mem::swap(&mut input.attrs, &mut unhandled_attrs);

            let mut idents = unhandled_attrs
                .into_iter()
                .filter_map(|item| {
                    // Try to get the item as a Meta
                    let meta = match item.parse_meta() {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Meta yolo: {:?}: {:?}", &item, e);
                            input.attrs.push(item);
                            return None;
                        }
                    };

                    let list = match meta {
                        syn::Meta::List(list) => list,
                        _ => {
                            input.attrs.push(item);
                            return None;
                        }
                    };

                    if list.nested.len() > 1 {
                        // TODO: throw proper error
                        input.attrs.push(item);
                        return None;
                    }

                    let marshaler = match list.nested.first() {
                        Some(syn::NestedMeta::Meta(syn::Meta::Path(v))) => v,
                        _ => {
                            // TODO: throw proper error
                            input.attrs.push(item);
                            return None;
                        }
                    };

                    Some(marshaler.to_owned())
                })
                .collect::<Vec<_>>();

            if idents.len() > 1 {
                // TODO: have a very strong, negative opinion.
            }

            println!("{:?} {:?}", &input, &idents);
            (input.to_owned(), idents.pop())
        })
        .map(|x| {
            let default_marshaler = default_marshaler(&x.0.ty).map(|x| x.clone());
            (x.0, x.1.or_else(|| default_marshaler))
        })
        .collect::<Vec<_>>();

    Ok(attrs)
}

fn process_function(
    mut func: syn::ItemFn,
) -> Result<(syn::ItemFn, Vec<(PatType, Option<syn::Path>)>), syn::Error> {
    // Dig into inputs
    let marshal_attrs = collect_mappings_from_sig(&mut func.sig)?;

    Ok((func, marshal_attrs))
}

#[derive(Debug, FromMeta, Default)]
pub struct InvokeParams {
    #[darling(default)]
    pub return_marshaler: Option<syn::Path>,
    #[darling(default)]
    pub prefix: Option<String>,
}

enum ReturnPtrTy {
    Mut,
    Const,
}

impl ReturnPtrTy {
    fn from(ty: Option<&syn::Type>) -> Option<ReturnPtrTy> {
        match ty {
            Some(syn::Type::Ptr(ptr)) => {
                if ptr.const_token.is_some() {
                    Some(ReturnPtrTy::Const)
                } else {
                    Some(ReturnPtrTy::Mut)
                }
            }
            _ => None,
        }
    }
}

pub fn call_with(
    invoke_params: InvokeParams,
    item: TokenStream,
) -> Result<TokenStream, syn::Error> {
    let item: syn::Item = syn::parse2(item.clone()).context("error parsing function body")?;
    match item {
        syn::Item::Fn(item) => call_with_function(invoke_params, item),
        syn::Item::Impl(item) => call_with_impl(invoke_params, item),
        _ => Err(syn::Error::new_spanned(&item, "Only supported on functions and impls")),
    }
}

fn call_with_impl(
    invoke_params: InvokeParams,
    item: syn::ItemImpl,
) -> Result<TokenStream, syn::Error> {
    if let Some(defaultness) = item.defaultness {
        return Err(syn::Error::new_spanned(&defaultness, "Does not support specialised impls"));
    }

    if let Some(unsafety) = item.unsafety {
        return Err(syn::Error::new_spanned(&unsafety, "Does not support unsafe impls"));
    }

    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&item.generics, "Does not support generic impls"));
    }

    if let Some(trait_) = item.trait_ {
        return Err(syn::Error::new_spanned(&trait_.1, "Does not support trait impls"));
    }

    use heck::SnakeCase;

    let self_ty = item.self_ty;
    let invoke_prefix = invoke_params.prefix.unwrap_or_else(|| "".into());
    let prefix = format!("{}_{}", invoke_prefix, quote! { #self_ty }).to_snake_case();
    let pub_methods = item.items.iter().filter_map(|impl_item| match impl_item {
        syn::ImplItem::Method(method) => match (
            &method.vis,
            method.sig.asyncness,
            method.sig.unsafety,
            &method.sig.abi,
            &method.sig.generics.params.is_empty(),
        ) {
            (syn::Visibility::Public(_), None, None, None, true) => Some(method),
            _ => None,
        },
        _ => None,
    });

    let pub_method_names = pub_methods
        .map(|x| {
            let ident = &x.sig.ident;
            let c_ident: syn::Ident = syn::parse_str(&format!("{}_{}", prefix, &ident).to_snake_case()).unwrap();

            quote! {
                #[no_mangle]
                pub extern "C" fn #c_ident() {
                    #self_ty::#ident();
                }
            }
        })
        .collect::<Vec<_>>();

    let free_ident: syn::Ident = syn::parse_str(&format!("{}_free", prefix).to_snake_case()).unwrap();

    println!("{:?}, {:?}", prefix, pub_method_names);
    Ok(quote! {
        #[no_mangle]
        pub extern "C" fn #free_ident(
            __handle: *mut ::libc::c_void,
            __exception: ::cursed::ErrCallback,
        ) {
            ::cursed::try_not_null!(
                ::cursed::BoxMarshaler::from_foreign_as_owned(__handle),
                __exception,
            );
            log::debug!("ex_pref_something_free has consumed this handle; do not reuse it!");
            unsafe {
                *__handle = std::ptr::null_mut();
            }
        }

        #(#pub_method_names)*
    })
}

fn call_with_function(
    invoke_params: InvokeParams,
    item: syn::ItemFn,
) -> Result<TokenStream, syn::Error> {
    let (fn_item, marshalers) = process_function(item.clone())?;

    let syn::Signature { ident: ref name, inputs: ref params, output: ref return_type, .. } =
        fn_item.sig.clone();

    let c_return_type = match return_type {
        syn::ReturnType::Type(x, ty) => {
            syn::ReturnType::Type(x.clone(), Box::new(to_c_type(&*ty)?))
        }
        x => x.clone(),
    };

    let return_type_ty = match &return_type {
        syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
        _ => None,
    };

    let c_return_type_ty = match &c_return_type {
        syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
        _ => None,
    };

    let c_return_is_ptr = ReturnPtrTy::from(c_return_type_ty.as_ref());

    let mut from_foreigns = TokenStream::new();
    let mut c_params: Punctuated<PatType, syn::Token![,]> = Punctuated::new();
    let mut c_args: Punctuated<Pat, syn::Token![,]> = Punctuated::new();

    let mut needs_exception_param = false;

    for (i, param) in params.iter().enumerate() {
        let (out_type, marshaler) = &marshalers[i];
        let name = to_c_arg(param).context("failed to convert Rust type to FFI type")?;
        let in_type = to_c_param(param).context("failed to convert Rust type to FFI type")?;

        if let Some(marshaler) = marshaler {
            let foreign = gen_foreign(&marshaler, &name, c_return_type_ty.as_ref());
            from_foreigns.extend(foreign);
            needs_exception_param = true;
        } else if !is_passthrough_type(&*out_type.ty) {
            println!("OUT TY: {:?}", &*out_type.ty);
            return Err(syn::Error::new_spanned(&param, "no marshaler found for type"));
        }

        c_params.push(in_type);
        c_args.push(name);
    }

    let passthrough_return = match return_type {
        syn::ReturnType::Type(_, ty) => is_passthrough_type(&ty),
        _ => true,
    };

    if needs_exception_param || !passthrough_return {
        c_params.push(PatType {
            attrs: vec![],
            pat: Box::new(Pat::Verbatim(quote! { __exception })),
            colon_token: <syn::Token![:]>::default(),
            ty: Box::new(Type::Verbatim(quote! { ::cursed::ErrCallback })),
        });
    }

    let rust_fn = &item;

    match return_type {
        syn::ReturnType::Default => Ok(quote! {
            #[no_mangle]
            extern "C" fn #name(#c_params) {
                #from_foreigns
                #rust_fn
                #name(#c_args);
            }
        }),
        syn::ReturnType::Type(_, ty) => {
            if is_passthrough_type(&ty) {
                return Ok(quote! {
                    #[no_mangle]
                    extern "C" fn #name(#c_params) #c_return_type {
                        #from_foreigns
                        #rust_fn
                        #name(#c_args)
                    }
                });
            }

            let return_marshaler =
                invoke_params.return_marshaler.map(|x| Ok(x)).unwrap_or_else(|| {
                    default_marshaler(&return_type_ty.clone().unwrap())
                        .map(|x| x.clone())
                        .ok_or_else(|| {
                            println!("ret: {:?}", &return_type_ty);
                            syn::Error::new_spanned(
                                &return_type,
                                "no marshaler found for return type",
                            )
                        })
                })?;

            let err = match c_return_is_ptr {
                None => quote! { ::cursed::throw!(e, __exception, <#c_return_type_ty>::default()) },
                Some(ReturnPtrTy::Const) => {
                    quote! { ::cursed::throw!(e, __exception, std::ptr::null()) }
                }
                Some(ReturnPtrTy::Mut) => {
                    quote! { ::cursed::throw!(e, __exception, std::ptr::null_mut()) }
                }
            };

            let return_to_foreign = quote! {
                match #return_marshaler::to_foreign(result) {
                    Ok(v) => v,
                    Err(e) => #err
                }
            };

            Ok(quote! {
                #[no_mangle]
                extern "C" fn #name(#c_params) #c_return_type {
                    #from_foreigns
                    #rust_fn
                    let result = #name(#c_args);
                    #return_to_foreign
                }
            })
        }
    }
}

fn to_c_param(arg: &FnArg) -> Result<PatType, syn::Error> {
    match arg {
        FnArg::Typed(arg) => {
            Ok(PatType { ty: Box::new(to_c_type(&*arg.ty)?), ..arg.clone() }.into())
        }
        _ => Err(syn::Error::new(arg.span(), "cannot")),
    }
}

fn to_c_arg(arg: &FnArg) -> Result<Pat, syn::Error> {
    match arg {
        FnArg::Typed(arg) => Ok(*arg.pat.clone()),
        _ => Err(syn::Error::new(arg.span(), "cannot")),
    }
}

fn gen_foreign(marshaler: &syn::Path, name: &syn::Pat, ty: Option<&syn::Type>) -> TokenStream {
    if let Some(ty) = ty {
        let fallback = match ReturnPtrTy::from(Some(ty)) {
            None => quote! { <#ty>::default() },
            Some(ReturnPtrTy::Const) => quote! { std::ptr::null() },
            Some(ReturnPtrTy::Mut) => quote! { std::ptr::null_mut() },
        };
        quote! {
            let #name = ::cursed::try_not_null!(
                #marshaler::from_foreign(#name),
                __exception,
                #fallback
            );
        }
    } else {
        quote! {
            let #name = ::cursed::try_not_null!(
                #marshaler::from_foreign(#name),
                __exception
            );
        }
    }
}

fn to_c_type(ty: &Type) -> Result<Type, syn::Error> {
    match &*ty {
        syn::Type::Path(..) | syn::Type::Reference(..) => {}
        syn::Type::Tuple(..) => {
            return Err(syn::Error::new(ty.span(), "Tuple parameters not supported"))
        }
        _ => return Err(syn::Error::new(ty.span(), "Unknown parameters not supported")),
    }

    let v: Option<syn::Type> =
        TYPE_MAPPING.get(&*quote! { #ty }.to_string()).and_then(|x| syn::parse_str(x).ok());

    match v {
        Some(ty) => Ok(ty),
        None => {
            let c_ty: Type = syn::parse2(quote! { *const ::libc::c_void }).unwrap();
            Ok(c_ty.clone())
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

fn default_marshaler(ty: &syn::Type) -> Option<syn::Path> {
    DEFAULT_MARSHALERS.get(&*quote! { #ty }.to_string()).and_then(|x| syn::parse_str(x).ok())
}

fn is_passthrough_type(ty: &syn::Type) -> bool {
    PASSTHROUGH_TYPES.contains(&&*quote! { #ty }.to_string())
}

trait ErrorExt<T> {
    fn context(self, msg: impl Display) -> Result<T, syn::Error>;
}

impl<T> ErrorExt<T> for Result<T, syn::Error> {
    fn context(self, msg: impl Display) -> Self {
        match self {
            Err(err) => Err(syn::Error::new(err.span(), format!("{}: {}", msg, err.to_string()))),
            x => x,
        }
    }
}
