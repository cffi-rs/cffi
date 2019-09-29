use ctor::ctor;
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::Display;

mod attr;
mod call_fn;
mod call_impl;
mod function;
mod ptr_type;
mod return_type;

pub use attr::invoke::InvokeParams;

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

pub(crate) trait ForeignArgExt {
    fn to_foreign_param(&self) -> Result<syn::PatType, syn::Error>;
    fn to_foreign_arg(&self) -> Result<syn::Pat, syn::Error>;
}

impl ForeignArgExt for syn::PatType {
    fn to_foreign_param(&self) -> Result<syn::PatType, syn::Error> {
        Ok(syn::PatType { ty: Box::new(self.ty.to_foreign_type()?), ..self.clone() }.into())
    }

    fn to_foreign_arg(&self) -> Result<syn::Pat, syn::Error> {
        Ok(*self.pat.clone())
    }
}

impl ForeignArgExt for syn::Receiver {
    fn to_foreign_param(&self) -> Result<syn::PatType, syn::Error> {
        Ok(syn::PatType {
            attrs: vec![],
            pat: syn::parse2(quote! { __handle }).unwrap(),
            colon_token: <syn::Token![:]>::default(),
            ty: Box::new(syn::parse2(quote! { *const ::libc::c_void }).unwrap()),
        })
    }

    fn to_foreign_arg(&self) -> Result<syn::Pat, syn::Error> {
        Ok(syn::parse2(quote! { __handle }).unwrap())
    }
}

impl ForeignArgExt for syn::FnArg {
    fn to_foreign_param(&self) -> Result<syn::PatType, syn::Error> {
        match self {
            syn::FnArg::Typed(arg) => arg.to_foreign_param(),
            syn::FnArg::Receiver(receiver) => receiver.to_foreign_param(),
        }
    }

    fn to_foreign_arg(&self) -> Result<syn::Pat, syn::Error> {
        match self {
            syn::FnArg::Typed(arg) => arg.to_foreign_arg(),
            syn::FnArg::Receiver(receiver) => receiver.to_foreign_arg(),
        }
    }
}

pub(crate) trait ForeignTypeExt {
    fn to_foreign_type(&self) -> Result<syn::Type, syn::Error>;
}

impl ForeignTypeExt for syn::Type {
    fn to_foreign_type(&self) -> Result<syn::Type, syn::Error> {
        match &self {
            syn::Type::Path(..) | syn::Type::Reference(..) => {}
            syn::Type::Tuple(..) => {
                return Err(syn::Error::new_spanned(self, "Tuple parameters not supported"))
            }
            _ => return Err(syn::Error::new_spanned(self, "Unknown parameters not supported")),
        }

        match is_passthrough_type(self) {
            true => Ok(self.clone()),
            false => {
                let c_ty: syn::Type = syn::parse2(quote! { *const ::libc::c_void }).unwrap();
                Ok(c_ty.clone())
            }
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

trait ErrorExt<T> {
    fn context(self, msg: impl Display) -> Result<T, syn::Error>;
}

impl<T> ErrorExt<T> for Result<T, syn::Error> {
    fn context(self, msg: impl Display) -> Self {
        match self {
            Err(err) => Err(syn::Error::new(err.span(), format!("{}: {}", msg, err.to_string()))),
            x => x,
        }
    }
}
