use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
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
    let params: Params = syn::parse2(params.clone())?;
    let function: syn::Item = syn::parse2(raw_function.clone())?;
    let fn_item = match function {
        syn::Item::Fn(f) => f,
        _ => {
            return Err(syn::Error::new_spanned(
                &raw_function,
                "cthulhu macro only supported on functions right now",
            ))
        }
    };

    let syn::ItemFn { ident: ref name, ref decl, .. } = fn_item.clone();
    let syn::FnDecl { inputs: ref params, output: ref return_type, .. } = *decl.clone();

    let mut c_params: Punctuated<FnArg, syn::Token![,]> = Punctuated::new();
    for param in params {
        c_params.extend(to_c_param(param)?);
    }

    let rust_fn = &raw_function;
    let body: syn::Block = syn::parse2(quote! {{
        #rust_fn

        unimplemented!()
    }})?;

    Ok(syn::ItemFn {
        abi: Some(syn::parse2(quote!(extern "C"))?),
        decl: Box::new(syn::FnDecl {
            inputs: c_params,
            generics: syn::parse2(quote!())?,
            ..*decl.clone()
        }),
        block: Box::new(body),
        ..fn_item
    }
    .into_token_stream())
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
    /// This is an example for using doc comment attributes
    pub static TYPE_MAPPING: std::collections::HashMap<Type, Vec<Type>> = map_types![
        bool => [::std::os::raw::c_char],
        u8 => [::std::os::raw::c_uchar],
        i8 => [::std::os::raw::c_char],
        i16 => [::std::os::raw::c_short],
        u16 => [::std::os::raw::c_ushort],
        i32 => [::std::os::raw::c_int],
        u32 => [::std::os::raw::c_uint],
        i64 => [::std::os::raw::c_long],
        u64 => [::std::os::raw::c_ulong],
        &'a CStr => [*const ::std::os::raw::c_char],
        CString => [*mut ::std::os::raw::c_char],
        Arc<str> => [*const ::std::os::raw::c_char, ::libc::size_t],
    ];
}

pub struct Params {}

impl Parse for Params {
    fn parse(_input: ParseStream) -> Result<Self, syn::Error> {
        Ok(Params {})
    }
}
