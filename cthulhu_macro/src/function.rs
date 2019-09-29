use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;

use crate::attr::{marshal::MarshalAttr, Mapping};
use crate::ext::*;
use crate::ptr_type::PtrType;
use crate::return_type::ReturnType;

fn gen_foreign(
    marshaler: &syn::Path,
    name: &syn::Pat,
    out_ty: &syn::Type,
    ret_ty: Option<&syn::Type>,
) -> TokenStream {
    if let Some(ty) = ret_ty {
        let fallback = match PtrType::from(Some(ty)) {
            None => quote! { <#ty>::default() },
            Some(PtrType::Const) => quote! { std::ptr::null() },
            Some(PtrType::Mut) => quote! { std::ptr::null_mut() },
        };
        quote! {
            let #name: #out_ty = ::cursed::try_not_null!(
                #marshaler::from_foreign(#name),
                __exception,
                #fallback
            );
        }
    } else {
        quote! {
            let #name: #out_ty = ::cursed::try_not_null!(
                #marshaler::from_foreign(#name),
                __exception
            );
        }
    }
}

#[derive(Debug)]
pub enum InnerFn {
    FunctionBody(syn::ItemFn),
    FunctionCall(syn::Path),
}

pub struct Function {
    name: syn::Ident,
    foreign_params: Punctuated<syn::PatType, syn::Token![,]>,
    foreign_args: Punctuated<syn::Pat, syn::Token![,]>,
    return_type: ReturnType,
    from_foreigns: TokenStream,
    inner_fn: InnerFn,
    fn_marshal_attr: Option<MarshalAttr>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Function { name, foreign_params, foreign_args, from_foreigns, .. } = &self;

        fmt.debug_struct("Function")
            .field("name", &format!("{}", quote! { #name }))
            .field("foreign_params", &format!("{}", quote! { #foreign_params }))
            .field("foreign_args", &format!("{}", quote! { #foreign_args }))
            .field("return_type", &self.return_type)
            .field("from_foreigns", &format!("{}", quote! { #from_foreigns }))
            .field("inner_fn", &self.inner_fn)
            .field("fn_marshal_attr", &self.fn_marshal_attr)
            .finish()
    }
}

trait TypeMarshalExt {
    fn resolve_marshaler<'a>(
        &self,
        marshaler_attr: Option<&'a MarshalAttr>,
    ) -> Result<&'a syn::Path, syn::Error>;
}

impl TypeMarshalExt for syn::Type {
    fn resolve_marshaler<'a>(
        &self,
        marshaler_attr: Option<&'a MarshalAttr>,
    ) -> Result<&'a syn::Path, syn::Error> {
        match marshaler_attr {
            Some(v) => Ok(&v.path),
            None => {
                return Err(syn::Error::new_spanned(&self, "no marshaler found for return type"))
            }
        }
    }
}

impl Function {
    pub fn new(
        name: syn::Ident,
        params: Punctuated<syn::FnArg, syn::Token![,]>,
        mappings: &[Mapping],
        return_type: ReturnType,
        inner_fn: InnerFn,
        fn_marshal_attr: Option<MarshalAttr>,
    ) -> Result<Function, syn::Error> {
        let mut from_foreigns = TokenStream::new();
        let mut foreign_params: Punctuated<syn::PatType, syn::Token![,]> = Punctuated::new();
        let mut foreign_args: Punctuated<syn::Pat, syn::Token![,]> = Punctuated::new();

        let mut has_exceptions = false;

        for (i, param) in params.iter().enumerate() {
            let mapping = &mappings[i];
            let out_type = &mapping.output_type;
            let marshaler = &mapping.marshaler;

            let name = param.to_foreign_arg().context("failed to convert Rust type to FFI type")?;
            let mut in_type =
                param.to_foreign_param().context("failed to convert Rust type to FFI type")?;

            if let Some(in_ty_override) = marshaler.as_ref().and_then(|m| m.types.first().cloned())
            {
                in_type.ty = Box::new(in_ty_override);
            }

            if let Some(marshaler) = mapping.marshaler.as_ref() {
                let foreign = gen_foreign(
                    &marshaler.path,
                    &name,
                    &out_type,
                    return_type.foreign_type().as_ref(),
                );
                from_foreigns.extend(foreign);
                has_exceptions = true;
            } else if !crate::is_passthrough_type(&out_type) {
                let box_marshaler = syn::parse2(quote! { ::cursed::BoxMarshaler::<#out_type> })?;
                let foreign = gen_foreign(
                    &box_marshaler,
                    &name,
                    &out_type,
                    return_type.foreign_type().as_ref(),
                );
                from_foreigns.extend(foreign);
                has_exceptions = true;
            }

            foreign_params.push(in_type);
            foreign_args.push(name);
        }

        let passthrough_return =
            return_type.local_type().map(|ty| crate::is_passthrough_type(&ty)).unwrap_or(true);

        if has_exceptions || !passthrough_return {
            foreign_params.push(syn::PatType {
                attrs: vec![],
                pat: Box::new(syn::Pat::Verbatim(quote! { __exception })),
                colon_token: <syn::Token![:]>::default(),
                ty: Box::new(syn::Type::Verbatim(quote! { ::cursed::ErrCallback })),
            });
        }

        Ok(Function {
            name,
            foreign_params,
            foreign_args,
            return_type,
            from_foreigns,
            inner_fn,
            fn_marshal_attr,
        })
    }

    fn build_signature(&self) -> Result<TokenStream, syn::Error> {
        let Self { name, foreign_params, .. } = self;

        let mut sig = quote! {
            #[no_mangle]
            pub extern "C" fn #name(#foreign_params)
        };

        if let syn::ReturnType::Type(_, ty) = &self.return_type.local {
            let return_marshaler = ty.resolve_marshaler(self.fn_marshal_attr.as_ref())?;
            let ret = quote! { -> <#return_marshaler as ::cursed::ReturnType>::Foreign };
            sig.extend(ret);
        }

        Ok(sig)
    }

    fn build_inner_block(&self) -> Result<TokenStream, syn::Error> {
        let Self { name, from_foreigns, foreign_args, .. } = self;

        // If we have a function body, inject it or just ignore it
        let original_fn = match &self.inner_fn {
            InnerFn::FunctionBody(body) => Some(body),
            _ => None,
        };

        let mut inner_block = quote! {
            #from_foreigns
            #original_fn
        };

        let call_name: syn::Path = match &self.inner_fn {
            InnerFn::FunctionCall(path) => path.clone(),
            _ => syn::parse2(quote! { #name }).unwrap(),
        };

        match &self.return_type.local {
            syn::ReturnType::Default => {
                inner_block.extend(quote! { #call_name(#foreign_args); });
            }
            syn::ReturnType::Type(_, ty) if crate::is_passthrough_type(&ty) => {
                inner_block.extend(quote! { #call_name(#foreign_args) });
            }
            syn::ReturnType::Type(_, ty) => {
                let return_marshaler = ty.resolve_marshaler(self.fn_marshal_attr.as_ref())?;

                let return_to_foreign = quote! {
                    match #return_marshaler::to_foreign(result) {
                        Ok(v) => v,
                        Err(e) => {
                            ::cursed::throw!(e, __exception, <#return_marshaler as ::cursed::ReturnType>::foreign_default())
                        }
                    }
                };

                inner_block.extend(quote! {
                    let result = #call_name(#foreign_args);
                    #return_to_foreign
                });
            }
        };

        Ok(inner_block)
    }

    pub fn to_token_stream(&self) -> Result<TokenStream, syn::Error> {
        let sig = self.build_signature()?;
        let inner_block = self.build_inner_block()?;

        Ok(quote! {
            #sig { #inner_block }
        })
    }
}
