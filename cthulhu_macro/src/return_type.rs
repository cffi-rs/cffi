use quote::quote;
use std::fmt::{self, Debug};

use crate::attr::marshal::MarshalAttr;
use crate::ext::*;
use crate::ptr_type::PtrType;

pub struct ReturnType {
    pub local: syn::ReturnType,
    pub foreign: syn::ReturnType,
}

impl Debug for ReturnType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let ReturnType { local, foreign } = &self;

        fmt.debug_struct("ReturnType")
            .field("local", &format!("{}", quote! { #local }))
            .field("foreign", &format!("{}", quote! { #foreign }))
            .finish()
    }
}

impl ReturnType {
    pub fn new(
        marshal_attr: Option<&MarshalAttr>,
        local: syn::ReturnType,
    ) -> Result<ReturnType, syn::Error> {
        let foreign = match (marshal_attr.and_then(|x| x.return_type()), &local) {
            (Some(ty), _) => ty,
            (_, syn::ReturnType::Type(x, ty)) => {
                syn::ReturnType::Type(x.clone(), Box::new(ty.to_foreign_type()?))
            }
            (_, x) => x.clone(),
        };

        Ok(ReturnType { foreign, local })
    }

    pub fn local_type(&self) -> Option<syn::Type> {
        match &self.local {
            syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
            _ => None,
        }
    }

    pub fn foreign_type(&self) -> Option<syn::Type> {
        match &self.foreign {
            syn::ReturnType::Type(_, ty) => Some(*ty.clone()),
            _ => None,
        }
    }

    pub fn foreign_ptr_type(&self) -> Option<PtrType> {
        PtrType::from(self.foreign_type().as_ref())
    }
}
