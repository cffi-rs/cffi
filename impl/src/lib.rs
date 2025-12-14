extern crate proc_macro;

use darling::{Error, FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use quote::quote;

mod attr;
mod call_fn;
mod call_impl;
mod ext;
mod function;
mod return_type;

use attr::invoke::InvokeParams;
use ext::*;

#[proc_macro_attribute]
pub fn marshal(
    params: proc_macro::TokenStream,
    function: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let params = match NestedMeta::parse_meta_list(params.into()) {
        Ok(v) => v,
        Err(e) => {
            return Error::from(e).write_errors().into();
        }
    };

    let params = match InvokeParams::from_list(&params) {
        Ok(v) => v,
        Err(err) => return err.write_errors().into(),
    };

    match call_with(params, function.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => syn::Error::new(err.span(), err.to_string())
            .to_compile_error()
            .into(),
    }
}

fn call_with(invoke_params: InvokeParams, item: TokenStream) -> Result<TokenStream, syn::Error> {
    let item: syn::Item = syn::parse2(item.clone()).context("error parsing function body")?;
    let result = match item {
        syn::Item::Fn(item) => call_fn::call_with_function(
            invoke_params.return_marshaler,
            invoke_params.callback,
            item,
            None,
        ),
        syn::Item::Impl(item) => call_impl::call_with_impl(invoke_params.prefix, item),
        item => {
            tracing::error!("{:?}", &item);
            Err(syn::Error::new_spanned(
                &item,
                "Only supported on functions and impls",
            ))
        }
    };

    if result.is_err() {
        tracing::debug!("macro finished with error");
    } else {
        tracing::debug!("macro finished successfully");
    }
    result
}

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub(crate) fn default_marshaler(ty: &syn::Type) -> Option<syn::Path> {
    DEFAULT_MARSHALERS
        .get(&*quote! { #ty }.to_string())
        .and_then(|x| syn::parse_str(x).ok())
}

pub(crate) fn is_passthrough_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::BareFn(bare_fn) => bare_fn.abi.is_some(),
        _ => PASSTHROUGH_TYPES.contains(&&*quote! { #ty }.to_string()),
    }
}
