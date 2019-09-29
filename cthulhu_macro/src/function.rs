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

    pub fn to_token_stream(&self) -> Result<TokenStream, syn::Error> {
        let Self {
            name,
            foreign_params,
            foreign_args,
            return_type,
            from_foreigns,
            inner_fn,
            fn_marshal_attr,
        } = self;

        let call_name: syn::Path = match inner_fn {
            InnerFn::FunctionCall(path) => path.clone(),
            _ => syn::parse2(quote! { #name }).unwrap(),
        };

        let original_fn = match inner_fn {
            InnerFn::FunctionBody(body) => Some(body),
            _ => None,
        };

        match &return_type.local {
            syn::ReturnType::Default => Ok(quote! {
                #[no_mangle]
                pub extern "C" fn #name(#foreign_params) {
                    #from_foreigns
                    #original_fn
                    #call_name(#foreign_args);
                }
            }),
            syn::ReturnType::Type(_, ty) => {
                let foreign_return = &return_type.foreign;

                if crate::is_passthrough_type(&ty) {
                    return Ok(quote! {
                        #[no_mangle]
                        extern "C" fn #name(#foreign_params) #foreign_return {
                            #from_foreigns
                            #original_fn
                            #call_name(#foreign_args)
                        }
                    });
                }

                let return_marshaler = match fn_marshal_attr.as_ref() {
                    Some(v) => &v.path,
                    None => {
                        return Err(syn::Error::new_spanned(
                            &ty,
                            "no marshaler found for return type",
                        ))
                    }
                };

                let err = match return_type.foreign_ptr_type() {
                    None => match return_type.foreign_type() {
                        Some(ret) => quote! { ::cursed::throw!(e, __exception, <#ret>::default()) },
                        None => quote! { ::cursed::throw!(e, __exception) },
                    },
                    Some(PtrType::Const) => {
                        quote! { ::cursed::throw!(e, __exception, std::ptr::null()) }
                    }
                    Some(PtrType::Mut) => {
                        quote! { ::cursed::throw!(e, __exception, std::ptr::null_mut()) }
                    }
                };

                let return_to_foreign = quote! {
                    match #return_marshaler::to_foreign(result) {
                        Ok(v) => v,
                        Err(e) => #err
                    }
                };

                Ok(quote! {
                    #[no_mangle]
                    pub extern "C" fn #name(#foreign_params) #foreign_return {
                        #from_foreigns
                        #original_fn
                        let result = #call_name(#foreign_args);
                        #return_to_foreign
                    }
                })
            }
        }
    }
}
