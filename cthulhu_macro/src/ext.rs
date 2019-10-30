use std::fmt::Display;

use quote::quote;

pub trait ForeignArgExt {
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
            ty: Box::new(syn::parse2(quote! { *const ::std::ffi::c_void }).unwrap()),
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

pub trait ForeignTypeExt {
    fn to_foreign_type(&self) -> Result<syn::Type, syn::Error>;
}

impl ForeignTypeExt for syn::Type {
    fn to_foreign_type(&self) -> Result<syn::Type, syn::Error> {
        match &self {
            syn::Type::Path(..) | syn::Type::Reference(..) => {}
            syn::Type::BareFn(bare_fn) => {
                if bare_fn.abi.is_some() {
                    // This one is safe to pass through.
                    return Ok(self.clone());
                }

                return Err(syn::Error::new_spanned(
                    self,
                    "Non-extern-C fn parameters not supported",
                ));
            }
            syn::Type::Tuple(..) => {
                return Err(syn::Error::new_spanned(self, "Tuple parameters not supported"))
            }
            _ => return Err(syn::Error::new_spanned(self, "Unknown parameters not supported")),
        }

        // Special case for bool
        let bool_ty: syn::Type = syn::parse_str("bool").unwrap();
        if self == &bool_ty {
            return Ok(syn::parse2(quote! { /* bool */ u8 }).unwrap());
        }

        match crate::is_passthrough_type(self) {
            true => Ok(self.clone()),
            false => {
                let c_ty: syn::Type = syn::parse2(quote! { *const ::std::ffi::c_void }).unwrap();
                Ok(c_ty)
            }
        }
    }
}

pub trait ErrorExt<T> {
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
