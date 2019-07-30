use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::Display;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    ArgCaptured, FnArg, Type,
};

pub fn call_with(
    params: TokenStream,
    raw_function: TokenStream,
) -> Result<TokenStream, syn::Error> {
    let _params: Params = syn::parse2(params.clone()).context("error parsing params")?;
    let function: syn::Item =
        syn::parse2(raw_function.clone()).context("error parsing function body")?;
    let fn_item = match function {
        syn::Item::Fn(f) => f,
        _ => {
            return Err(syn::Error::new_spanned(
                &raw_function,
                "only supported on functions right now",
            ))
        }
    };

    let syn::ItemFn { ident: ref name, ref decl, .. } = fn_item.clone();
    let syn::FnDecl { inputs: ref params, output: ref return_type, .. } = *decl.clone();

    let mut c_params: Punctuated<FnArg, syn::Token![,]> = Punctuated::new();
    for param in params {
        c_params.extend(to_c_param(param).context("failed to convert Rust type to FFI type")?);
    }

    let rust_fn = &raw_function;
    Ok(quote! {
        #[no_mangle]
        extern "C" fn #name(#c_params) #return_type {
            #rust_fn
            unimplemented!()
        }
    })
}

fn to_c_param(arg: &FnArg) -> Result<Vec<FnArg>, syn::Error> {
    match arg {
        FnArg::Captured(arg) => to_c_type(arg),
        x => Ok(vec![x.clone()]),
    }
}

fn to_c_type(arg: &ArgCaptured) -> Result<Vec<FnArg>, syn::Error> {
    TYPE_MAPPING.with(|map| {
        let ArgCaptured { ty, pat, .. } = arg.clone();
        match &ty {
            syn::Type::Path(..) | syn::Type::Reference(..) => {}

            syn::Type::Slice(..) => {
                return Err(syn::Error::new(pat.span(), "Slice parameters not supported"))
            }
            syn::Type::Array(..) => {
                return Err(syn::Error::new(pat.span(), "Array parameters not supported"))
            }
            syn::Type::Ptr(..) => {
                return Err(syn::Error::new(pat.span(), "Ptr parameters not supported"))
            }
            syn::Type::BareFn(..) => {
                return Err(syn::Error::new(pat.span(), "BareFn parameters not supported"))
            }
            syn::Type::Never(..) => {
                return Err(syn::Error::new(pat.span(), "Never parameters not supported"))
            }
            syn::Type::Tuple(..) => {
                return Err(syn::Error::new(pat.span(), "Tuple parameters not supported"))
            }
            syn::Type::TraitObject(..) => {
                return Err(syn::Error::new(pat.span(), "TraitObject parameters not supported"))
            }
            syn::Type::ImplTrait(..) => {
                return Err(syn::Error::new(pat.span(), "ImplTrait parameters not supported"))
            }
            syn::Type::Paren(..) => {
                return Err(syn::Error::new(pat.span(), "Paren parameters not supported"))
            }
            syn::Type::Group(..) => {
                return Err(syn::Error::new(pat.span(), "Group parameters not supported"))
            }
            syn::Type::Infer(..) => {
                return Err(syn::Error::new(pat.span(), "Infer parameters not supported"))
            }
            syn::Type::Macro(..) => {
                return Err(syn::Error::new(pat.span(), "Macro parameters not supported"))
            }
            syn::Type::Verbatim(..) => {
                return Err(syn::Error::new(pat.span(), "Verbatim parameters not supported"))
            }
        }

        match map.get(&ty).cloned() {
            Some(types) => match types.as_slice() {
                [c_ty] => Ok(vec![ArgCaptured { ty: c_ty.clone(), ..arg.clone() }.into()]),
                [c_ty, len] => {
                    let name = if let syn::Pat::Ident(syn::PatIdent { ident, .. }) = pat {
                        let mut name = ident.to_string();
                        name.push_str("_len");
                        syn::Ident::new(&name, ident.span())
                    } else {
                        return Err(syn::Error::new(
                            pat.span(),
                            "pattern as parameters not supported",
                        ));
                    };
                    Ok(vec![
                        ArgCaptured { ty: c_ty.clone(), ..arg.clone() }.into(),
                        ArgCaptured {
                            pat: syn::PatIdent {
                                ident: name,
                                by_ref: None,
                                mutability: None,
                                subpat: None,
                            }
                            .into(),
                            ty: len.clone(),
                            ..arg.clone()
                        }
                        .into(),
                    ])
                }
                _ => unreachable!(),
            },
            None => Ok(vec![arg.clone().into()]),
        }
    })
}

macro_rules! map_types {
    [$($rust:ty => [$($c:ty),*],)*] => {{
        let mut map = std::collections::HashMap::<Type, Vec<Type>>::new();
        $(map.insert(
            syn::parse2(quote!{ $rust })
                .expect(concat!("cannot parse", stringify!($rust), "as type")),
            vec![$(
                syn::parse2(quote!{ $c })
                    .expect(concat!("cannot parse", stringify!($c), "as type")),
            )*],
        );)*
        map
    }}
}

thread_local! {
    pub static TYPE_MAPPING: std::collections::HashMap<Type, Vec<Type>> = map_types![
        bool => [::libc::c_char],
        u8 => [::libc::c_uchar],
        i8 => [::libc::c_char],
        i16 => [::libc::c_short],
        u16 => [::libc::c_ushort],
        i32 => [::libc::c_int],
        u32 => [::libc::c_uint],
        i64 => [::libc::c_long],
        u64 => [::libc::c_ulong],
        &'a CStr => [*const ::libc::c_char],
        CString => [*mut ::libc::c_char],
        Arc<str> => [*const ::libc::c_char, ::libc::size_t],
    ];
}

pub struct Params {}

impl Parse for Params {
    fn parse(_input: ParseStream) -> Result<Self, syn::Error> {
        Ok(Params {})
    }
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
