use ctor::ctor;
use proc_macro2::TokenStream;
use quote::quote;

mod attr;
mod call_fn;
mod call_impl;
mod ext;
mod function;
mod ptr_type;
mod return_type;

pub use attr::invoke::InvokeParams;
use ext::*;

#[ctor]
fn init() {
    pretty_env_logger::init();
}

pub fn call_with(
    invoke_params: InvokeParams,
    item: TokenStream,
) -> Result<TokenStream, syn::Error> {
    let item: syn::Item = syn::parse2(item.clone()).context("error parsing function body")?;
    match item {
        syn::Item::Fn(item) => {
            call_fn::call_with_function(invoke_params.return_marshaler, item, None)
        }
        syn::Item::Impl(item) => call_impl::call_with_impl(invoke_params.prefix, item),
        item => {
            log::error!("{:?}", &item);
            Err(syn::Error::new_spanned(&item, "Only supported on functions and impls"))
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub(crate) fn default_marshaler(ty: &syn::Type) -> Option<syn::Path> {
    DEFAULT_MARSHALERS.get(&*quote! { #ty }.to_string()).and_then(|x| syn::parse_str(x).ok())
}

pub(crate) fn is_passthrough_type(ty: &syn::Type) -> bool {
    PASSTHROUGH_TYPES.contains(&&*quote! { #ty }.to_string())
}
