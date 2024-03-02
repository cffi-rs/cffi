use heck::ToSnakeCase as _;
use log::debug;
use proc_macro2::TokenStream;
use quote::quote;

use super::{function::Function, function::InnerFn, return_type::ReturnType};
use crate::attr::marshal::MarshalAttr;
use crate::attr::SignatureExt;

pub(crate) fn call_with_impl(
    prefix: Option<String>,
    mut item: syn::ItemImpl,
) -> Result<TokenStream, syn::Error> {
    debug!("{}", {
        let mut item = item.clone();
        item.items = vec![];
        quote! { #item }
    });

    if let Some(defaultness) = item.defaultness {
        return Err(syn::Error::new_spanned(
            &defaultness,
            "Does not support specialised impls",
        ));
    }

    if let Some(unsafety) = item.unsafety {
        return Err(syn::Error::new_spanned(
            &unsafety,
            "Does not support unsafe impls",
        ));
    }

    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "Does not support generic impls",
        ));
    }

    if let Some(trait_) = item.trait_ {
        return Err(syn::Error::new_spanned(
            &trait_.1,
            "Does not support trait impls",
        ));
    }

    let self_ty = &*item.self_ty;
    let invoke_prefix = prefix.unwrap_or_else(|| "".into());
    let prefix = format!("{}_{}", invoke_prefix, quote! { #self_ty }).to_snake_case();
    let pub_methods = item
        .items
        .iter_mut()
        .filter_map(|impl_item| match impl_item {
            syn::ImplItem::Fn(method) => match (
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

    let foreign_methods = pub_methods
        .map(|x| {
            let ident = &x.sig.ident;
            let fn_path: syn::Path = syn::parse2(quote! { #self_ty::#ident })?;
            let c_ident: syn::Ident =
                syn::parse_str(&format!("{}_{}", prefix, &ident).to_snake_case()).unwrap();

            let mappings = x.sig.drain_mappings(Some(&*self_ty))?;

            debug!("impl fn {}", quote! { #fn_path });
            debug!("impl fn def: {}", quote! { #x });

            let mut attrs = vec![];
            std::mem::swap(&mut attrs, &mut x.attrs);

            let mut idents = attrs
                .into_iter()
                .filter_map(|item| {
                    debug!("attr {}", quote! { #item });
                    match MarshalAttr::from_attribute(item.clone()) {
                        Ok(None) => {
                            x.attrs.push(item);
                            return None;
                        }
                        Ok(Some(v)) => Some(Ok(v)),
                        Err(e) => return Some(Err(e)),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            let attr = idents.pop();

            let syn::Signature {
                inputs: params,
                output: local_return_type,
                ..
            } = x.sig.clone();

            let fn_marshal_attr = match attr.map(|x| x.path) {
                Some(p) => MarshalAttr::from_path(p)?,
                None => MarshalAttr::from_defaults_by_return_type(&local_return_type),
            };

            let return_type = ReturnType::new(fn_marshal_attr.as_ref(), local_return_type)?;
            let function = Function::new(
                c_ident,
                params,
                &mappings,
                return_type,
                InnerFn::FunctionCall(fn_path),
                fn_marshal_attr,
                false,
            )?;

            debug!("{:#?}", &function);

            function.to_token_stream()
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(quote! {
        #item

        #(#foreign_methods)*
    })
}
