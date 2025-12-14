use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::{self, Debug};
use syn::punctuated::Punctuated;

use crate::attr::{Mapping, marshal::MarshalAttr};
use crate::ext::*;
use crate::return_type::ReturnType;

fn gen_throw(fallback: Option<TokenStream>, no_return: bool) -> TokenStream {
    let fallback = if no_return {
        None
    } else {
        Some(quote! { return #fallback; })
    };

    quote! {
        {
            if let Some(callback) = __exception {
                let err = format!("{:?}", e);
                callback(err.as_bytes().as_ptr().cast(), err.len());

            }
            #fallback
        }
    }
}

fn gen_try_not_null(
    path: TokenStream,
    fallback: Option<TokenStream>,
    no_return: bool,
) -> TokenStream {
    let throw = gen_throw(fallback, no_return);

    quote! {
        match #path {
            Ok(v) => v,
            Err(e) => #throw
        }
    }
}

fn gen_foreign(
    marshaler: &MarshalAttr,
    name: &syn::Pat,
    out_ty: &syn::Type,
    out_marshaler: Option<&syn::Path>,
    ret_ty: Option<&syn::Type>,
    has_callback: bool,
) -> TokenStream {
    let marshaler_path = &marshaler.path;
    let marshal_ty = marshaler.first_type();

    let block = gen_try_not_null(
        if marshal_ty.as_ref().map(is_trait_object).unwrap_or(false) {
            quote! { unsafe {
                let #name = std::mem::transmute::<_, *const (#marshal_ty)>(#name);
                #marshaler_path::from_foreign(#name)
            } }
        } else {
            quote! { unsafe { #marshaler_path::from_foreign(#name) } }
        },
        ret_ty.filter(|_| !has_callback).map(|ty| {
            if crate::is_passthrough_type(ty) {
                quote! { <#ty>::default() }
            } else if is_trait_object(ty) {
                quote! { <#out_marshaler as ::cffi::ReturnType>::foreign_default_trait_object() }
            } else {
                quote! { <#out_marshaler as ::cffi::ReturnType>::foreign_default() }
            }
        }),
        false,
    );

    quote! { let #name: #out_ty = #block; }
}

pub enum InnerFn {
    FunctionBody(Box<syn::ItemFn>),
    FunctionCall(syn::Path),
}

impl Debug for InnerFn {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("InnerFn")
            .field(&match self {
                InnerFn::FunctionBody(_item_fn) => "FunctionBody".to_string(),
                InnerFn::FunctionCall(_path) => "FunctionCall".to_string(),
            })
            .finish()
    }
}

#[allow(dead_code)]
pub struct Function {
    name: syn::Ident,
    original_params: Punctuated<syn::FnArg, syn::Token![,]>,
    foreign_params: Punctuated<syn::PatType, syn::Token![,]>,
    foreign_args: Punctuated<syn::Pat, syn::Token![,]>,
    return_type: ReturnType,
    return_marshaler: Option<syn::Path>,
    from_foreigns: TokenStream,
    inner_fn: InnerFn,
    fn_marshal_attr: Option<MarshalAttr>,
    has_exceptions: bool,
    has_callback: bool,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Function {
            name,
            original_params,
            foreign_params,
            foreign_args,
            from_foreigns,
            ..
        } = &self;

        fmt.debug_struct("Function")
            .field("name", &format!("{}", quote! { #name }))
            .field(
                "original_params",
                &format!("{}", quote! { #original_params }),
            )
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
            syn::ReturnType::Type(_, ty) if crate::is_passthrough_type(ty) => None,
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

fn is_trait_object(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::TraitObject(_))
}

impl Function {
    pub fn new(
        name: syn::Ident,
        params: Punctuated<syn::FnArg, syn::Token![,]>,
        mappings: &[Mapping],
        return_type: ReturnType,
        inner_fn: InnerFn,
        fn_marshal_attr: Option<MarshalAttr>,
        has_callback: bool,
    ) -> Result<Function, syn::Error> {
        let mut from_foreigns = TokenStream::new();
        let mut foreign_params: Punctuated<syn::PatType, syn::Token![,]> = Punctuated::new();
        let mut foreign_args: Punctuated<syn::Pat, syn::Token![,]> = Punctuated::new();
        let return_marshaler = return_type
            .local
            .resolve_marshaler(fn_marshal_attr.as_ref());

        let mut has_exceptions = false;

        for (i, param) in params.iter().enumerate() {
            tracing::debug!("{i} {:?}", param);
            let mapping = &mappings[i];
            let out_type = &mapping.output_type;
            let _marshaler = &mapping.marshaler;

            let name = param
                .to_foreign_arg()
                .context("failed to convert Rust type to FFI type")?;
            let mut in_type = param
                .to_foreign_param()
                .context("failed to convert Rust type to FFI type")?;

            if let Some(marshaler) = mapping.marshaler.as_ref() {
                let path = &marshaler.path;
                assert!(!path.segments.is_empty());
                let is_trait_object = marshaler
                    .first_type()
                    .map(|x| is_trait_object(&x))
                    .unwrap_or(false);

                in_type.ty = if is_trait_object {
                    Box::new(syn::Type::Verbatim(
                        quote! { <#path as ::cffi::InputType>::ForeignTraitObject },
                    ))
                } else {
                    Box::new(syn::Type::Verbatim(
                        quote! { <#path as ::cffi::InputType>::Foreign },
                    ))
                };

                let foreign = gen_foreign(
                    marshaler,
                    &name,
                    out_type,
                    return_marshaler,
                    return_type.foreign_type().as_ref(),
                    has_callback,
                );
                from_foreigns.extend(foreign);
                has_exceptions = true;
            } else if !crate::is_passthrough_type(out_type) {
                in_type.ty = Box::new(syn::Type::Verbatim(quote! {
                    <::cffi::BoxMarshaler::<#out_type> as ::cffi::InputType>::Foreign
                }));

                let box_marshaler = MarshalAttr {
                    path: syn::parse2(quote! { ::cffi::BoxMarshaler::<#out_type> })?,
                    types: vec![out_type.clone()],
                };
                let foreign = gen_foreign(
                    &box_marshaler,
                    &name,
                    out_type,
                    return_marshaler,
                    return_type.foreign_type().as_ref(),
                    has_callback,
                );
                from_foreigns.extend(foreign);
                has_exceptions = true;
            }

            foreign_params.push(in_type);
            foreign_args.push(name);
        }

        let passthrough_return = return_type
            .local_type()
            .map(|ty| crate::is_passthrough_type(&ty))
            .unwrap_or(true);

        if has_exceptions || !passthrough_return {
            foreign_params.push(syn::PatType {
                attrs: vec![],
                pat: Box::new(syn::Pat::Verbatim(quote! { __exception })),
                colon_token: <syn::Token![:]>::default(),
                ty: Box::new(syn::Type::Verbatim(quote! { ::cffi::ErrCallback })),
            });
        }

        Ok(Function {
            name,
            original_params: params,
            foreign_params,
            foreign_args,
            return_type,
            return_marshaler: return_marshaler.cloned(),
            from_foreigns,
            inner_fn,
            fn_marshal_attr,
            has_exceptions,
            has_callback,
        })
    }

    fn build_signature(&self) -> Result<TokenStream, syn::Error> {
        let Self {
            name,
            foreign_params,
            ..
        } = self;

        let mut sig = quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn #name
        };

        let ty = if let syn::ReturnType::Type(_, ty) = &self.return_type.local {
            Some(if crate::is_passthrough_type(ty) {
                quote! { #ty }
            } else {
                let return_marshaler = match ty.resolve_marshaler(self.fn_marshal_attr.as_ref()) {
                    Some(v) => v,
                    None => {
                        return Err(syn::Error::new_spanned(
                            ty,
                            format!("no marshaler found for return type {}", quote! { #ty }),
                        ));
                    }
                };

                let is_trait_object = self
                    .fn_marshal_attr
                    .as_ref()
                    .and_then(|x| x.first_type())
                    .map(|x| is_trait_object(&x))
                    .unwrap_or(false);
                if is_trait_object {
                    quote! { <#return_marshaler as ::cffi::ReturnType>::ForeignTraitObject }
                } else {
                    quote! { <#return_marshaler as ::cffi::ReturnType>::Foreign }
                }
            })
        } else {
            None
        };

        sig.extend(if let Some(ty) = ty {
            if self.has_callback {
                quote! { (#foreign_params, __return: ::cffi::RetCallback<#ty>) }
            } else {
                quote! { (#foreign_params) -> #ty }
            }
        } else {
            quote! { (#foreign_params) }
        });

        Ok(sig)
    }

    fn build_inner_block(&self) -> Result<TokenStream, syn::Error> {
        let Self {
            name,
            from_foreigns,
            foreign_args,
            ..
        } = self;

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
            syn::ReturnType::Type(_, ty) if crate::is_passthrough_type(ty) => {
                if self.has_callback {
                    inner_block.extend(quote! {
                        if let Some(__return) = __return {
                            __return(#call_name(#foreign_args));
                        }
                    });
                } else {
                    inner_block.extend(quote! { #call_name(#foreign_args) });
                }
            }
            syn::ReturnType::Type(_, ty) => {
                let return_marshaler = match ty.resolve_marshaler(self.fn_marshal_attr.as_ref()) {
                    Some(v) => v,
                    None => {
                        return Err(syn::Error::new_spanned(
                            ty,
                            format!("no marshaler found for return type {}", quote! { #ty }),
                        ));
                    }
                };

                let is_trait_object = self
                    .fn_marshal_attr
                    .as_ref()
                    .and_then(|x| x.first_type())
                    .map(|x| is_trait_object(&x))
                    .unwrap_or(false);

                let throw = gen_throw(
                    Some(if is_trait_object {
                        quote! {
                            <#return_marshaler as ::cffi::ReturnType>::foreign_default_trait_object()
                        }
                    } else {
                        quote! {
                            <#return_marshaler as ::cffi::ReturnType>::foreign_default()
                        }
                    }),
                    self.has_callback,
                );

                if self.has_callback {
                    inner_block.extend(quote! {
                        let result = #call_name(#foreign_args);
                        if let Some(__return) = __return {
                            match #return_marshaler::to_foreign(result) {
                                Ok(v) => __return(v),
                                Err(e) => #throw
                            }
                        }
                    });
                } else if is_trait_object {
                    let dyn_ty = self
                        .fn_marshal_attr
                        .as_ref()
                        .and_then(|x| x.first_type())
                        .unwrap();

                    inner_block.extend(quote! {
                        let result = #call_name(#foreign_args);
                        match #return_marshaler::to_foreign(result) {
                            Ok(v) => unsafe { cffi::trait_object!(v: (#dyn_ty)) },
                            Err(e) => #throw
                        }
                    });
                } else {
                    inner_block.extend(quote! {
                        let result = #call_name(#foreign_args);
                        match #return_marshaler::to_foreign(result) {
                            Ok(v) => v,
                            Err(e) => #throw
                        }
                    });
                }
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
