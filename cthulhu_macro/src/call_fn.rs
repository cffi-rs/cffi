use log::debug;
use proc_macro2::TokenStream;
use quote::quote;

use super::{function::Function, function::InnerFn, return_type::ReturnType};
use crate::attr::marshal::MarshalAttr;
use crate::attr::SignatureExt;

pub fn call_with_function(
    return_marshaler: Option<syn::Path>,
    mut fn_item: syn::ItemFn,
    parent_type: Option<&syn::Type>,
) -> Result<TokenStream, syn::Error> {
    debug!("fn {}", {
        let ident = &fn_item.sig.ident;
        quote! { #ident }
    });

    let mappings = fn_item.sig.drain_mappings(parent_type)?;

    // The wrapped function should not be extern any longer
    fn_item.sig.abi = None;

    let fn_marshal_attr = match return_marshaler {
        Some(p) => MarshalAttr::from_path(p)?,
        None => MarshalAttr::from_defaults_by_return_type(&fn_item.sig.output),
    };

    let return_type = ReturnType::new(fn_marshal_attr.as_ref(), fn_item.sig.output.clone())?;
    let function = Function::new(
        fn_item.sig.ident.clone(),
        fn_item.sig.inputs.clone(),
        &mappings,
        return_type,
        InnerFn::FunctionBody(fn_item),
        fn_marshal_attr,
    )?;

    function.to_token_stream()
}
