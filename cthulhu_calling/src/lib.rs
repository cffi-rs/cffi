use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
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
    let c_params: Punctuated<FnArg, syn::Token![,]> = params.iter().flat_map(to_c_param).collect();

    let rust_fn = &raw_function;
    let body: syn::Block = syn::parse2(quote!{{
        #rust_fn

        unimplemented!()
    }}).unwrap();

    Ok(syn::ItemFn {
        abi: Some(syn::parse2(quote!(extern "C")).unwrap()),
        decl: Box::new(syn::FnDecl { inputs: c_params, ..*decl.clone() }),
        block: Box::new(body),
        ..fn_item
    }.into_token_stream())
}

fn to_c_param(arg: &FnArg) -> Vec<FnArg> {
    match arg {
        FnArg::Captured(arg) => to_c_type(arg),
        x => vec![x.clone()],
    }
}

fn to_c_type(arg: &ArgCaptured) -> Vec<FnArg> {
    macro_rules! map_types {
        [$($rust:path => $c:ty,)*] => {{
            let mut map = std::collections::HashMap::<Type, Type>::new();
            $(map.insert(
                syn::parse2(quote!{ $rust }).unwrap(),
                syn::parse2(quote!{ $c }).unwrap(),
            );)*
            map
        }}
    }

    let map = map_types![
        bool => ::std::os::raw::c_char,
        u8 => ::std::os::raw::c_uchar,
        i8 => ::std::os::raw::c_char,
        i16 => ::std::os::raw::c_short,
        u16 => ::std::os::raw::c_ushort,
        i32 => ::std::os::raw::c_int,
        u32 => ::std::os::raw::c_uint,
        i64 => ::std::os::raw::c_long,
        u64 => ::std::os::raw::c_ulong,
        CStr => *const ::std::os::raw::c_char,
        CString => *mut ::std::os::raw::c_char,
    ];

    let ArgCaptured { ref ty, .. } = arg;
    let c_ty = map.get(&ty).cloned().unwrap_or_else(|| ty.clone());

    vec![ArgCaptured { ty: c_ty, ..arg.clone() }.into()]
}

pub struct Params {}

impl Parse for Params {
    fn parse(_input: ParseStream) -> Result<Self, syn::Error> {
        Ok(Params {})
    }
}
