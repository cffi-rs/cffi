use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;

use crate::attr::{marshal::MarshalAttr, Mapping};
use crate::ext::*;
// use crate::ptr_type::PtrType;
use crate::return_type::ReturnType;

fn gen_throw(fallback: Option<TokenStream>) -> TokenStream {
    quote! {
        {
            if let Some(callback) = __exception {
                let err = format!("{:?}", e);
                let s = std::ffi::CString::new(err)
                    .unwrap_or_else(|_| std::ffi::CString::new("<unknown>".to_string()).unwrap());
                callback(s.as_ptr().cast());
            }
            return #fallback;
        }
    }
}

fn gen_try_not_null(path: TokenStream, fallback: Option<TokenStream>) -> TokenStream {
    let throw = gen_throw(fallback);

    quote! {
        match #path {
            Ok(v) => v,
            Err(e) => #throw
        }
    }
}

fn gen_foreign(
    marshaler: &syn::Path,
    name: &syn::Pat,
    out_ty: &syn::Type,
    out_marshaler: Option<&syn::Path>,
    ret_ty: Option<&syn::Type>,
) -> TokenStream {
    let block = gen_try_not_null(
        quote! { #marshaler::from_foreign(#name) },
        ret_ty.map(|ty| {
            if crate::is_passthrough_type(ty) {
                quote! { <#ty>::default() }
            } else {
                quote! { <#out_marshaler as ::cursed::ReturnType>::foreign_default() }
            }
        }),
    );

    quote! { let #name: #out_ty = #block; }
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
    return_marshaler: Option<syn::Path>,
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
    ) -> Option<&'a syn::Path>;
}

impl TypeMarshalExt for syn::ReturnType {
    fn resolve_marshaler<'a>(
        &self,
        marshaler_attr: Option<&'a MarshalAttr>,
    ) -> Option<&'a syn::Path> {
        match &self {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) if crate::is_passthrough_type(&ty) => None,
            syn::ReturnType::Type(_, ty) => ty.resolve_marshaler(marshaler_attr),
        }
    }
}

impl TypeMarshalExt for syn::Type {
    fn resolve_marshaler<'a>(
        &self,
        marshaler_attr: Option<&'a MarshalAttr>,
    ) -> Option<&'a syn::Path> {
        marshaler_attr.map(|a| &a.path)
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
        let return_marshaler = return_type.local.resolve_marshaler(fn_marshal_attr.as_ref());

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
                    return_marshaler,
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
                    return_marshaler,
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
            return_marshaler: return_marshaler.map(|x| x.clone()),
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
            let ret = if crate::is_passthrough_type(&ty) {
                quote! { -> #ty }
            } else {
                let return_marshaler = match ty.resolve_marshaler(self.fn_marshal_attr.as_ref()) {
                    Some(v) => v,
                    None => {
                        return Err(syn::Error::new_spanned(
                            ty,
                            format!(
                                "no marshaler found for return type {}",
                                quote! { #ty }.to_string()
                            ),
                        ))
                    }
                };
                quote! { -> <#return_marshaler as ::cursed::ReturnType>::Foreign }
            };

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
                let return_marshaler = match ty.resolve_marshaler(self.fn_marshal_attr.as_ref()) {
                    Some(v) => v,
                    None => {
                        return Err(syn::Error::new_spanned(
                            ty,
                            format!(
                                "no marshaler found for return type {}",
                                quote! { #ty }.to_string()
                            ),
                        ))
                    }
                };

                let throw = gen_throw(Some(quote! {
                    <#return_marshaler as ::cursed::ReturnType>::foreign_default()
                }));
                let return_to_foreign = quote! {
                    match #return_marshaler::to_foreign(result) {
                        Ok(v) => v,
                        Err(e) => #throw
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
            #sig {
                #inner_block
            }
        })
    }
}
