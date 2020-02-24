use ctor::ctor;
use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

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

fn json_output_path() -> PathBuf {
    std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("cthulhu.json")
}

fn is_exporting() -> bool {
    let target = std::env::var("CTHULHU_PKG").ok();

    if let Some(target) = target {
        let name = match std::env::var("CARGO_PKG_NAME").ok() {
            Some(v) => v,
            None => return false,
        };
        let version = match std::env::var("CARGO_PKG_VERSION").ok() {
            Some(v) => v,
            None => return false,
        };
        format!("{}-{}", name, version) == target
    } else {
        false
    }
}

// pub(crate) struct Context {
//     pkg_name: String,
//     pkg_version: String,
//     cargo_manifest_dir: PathBuf,
// }

// impl Default for Context {
//     fn default() -> Context {
//         Context {
//             pkg_name:
//         }
//         std::env::var("CARGO_PKG_NAME")
//         std::env::var("CARGO_PKG_VERSION")
//         std::env::var("CARGO_MANIFEST_DIR")
//     }
// }

pub fn call_with(
    invoke_params: InvokeParams,
    item: TokenStream,
) -> Result<TokenStream, syn::Error> {
    // if let Some(value) = invoke_params.send_help.as_ref() {
    //     log::debug!("HELP REQUESTED: {}", value);
    //     return Ok(item);
    // }

    // log::debug!("{:?} {:?} {:?}", std::env::var("CARGO_PKG_NAME"), std::env::var("CARGO_PKG_VERSION"), std::env::var("CARGO_MANIFEST_DIR"));

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
            log::error!("{:?}", &item);
            Err(syn::Error::new_spanned(&item, "Only supported on functions and impls"))
        }
    };

    if result.is_err() {
        log::debug!("macro finished with error");
    } else {
        log::debug!("macro finished successfully");
    }

    result
}

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub(crate) fn default_marshaler(ty: &syn::Type) -> Option<syn::Path> {
    DEFAULT_MARSHALERS.get(&*quote! { #ty }.to_string()).and_then(|x| syn::parse_str(x).ok())
}

pub(crate) fn is_passthrough_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::BareFn(bare_fn) => bare_fn.abi.is_some(),
        _ => PASSTHROUGH_TYPES.contains(&&*quote! { #ty }.to_string()),
    }
}
