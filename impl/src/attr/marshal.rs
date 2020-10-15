use quote::quote;
use std::fmt::{self, Debug};

/// Holds onto the inside of #[marshal(...)]
///
/// Example:
///    place::Foo::<Bar>
///
///    MarshalAttr { path: place::Foo, types: [Bar] }
pub struct MarshalAttr {
    pub path: syn::Path,
    pub types: Vec<syn::Type>,
}

impl Debug for MarshalAttr {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let MarshalAttr { path, types } = self;

        fmt.debug_struct("MarshalAttr")
            .field("path", &quote! { #path }.to_string())
            .field(
                "types",
                &types
                    .iter()
                    .map(|x| quote! { #x }.to_string())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl MarshalAttr {
    pub fn self_type() -> Option<MarshalAttr> {
        Some(MarshalAttr {
            path: syn::parse2(quote! { ::cursed::BoxMarshaler }).unwrap(),
            types: vec![],
        })
    }

    pub fn first_type(&self) -> Option<syn::Type> {
        self.types.first().cloned()
    }

    pub fn return_type(&self) -> Option<syn::ReturnType> {
        self.first_type()
            .map(|ty| syn::ReturnType::Type(<syn::Token![->]>::default(), Box::new(ty)))
    }

    pub fn from_defaults_by_type(ty: &syn::Type) -> Option<MarshalAttr> {
        crate::default_marshaler(&ty).map(|x| MarshalAttr {
            path: x.clone(),
            types: vec![],
        })
    }

    pub fn from_defaults_by_return_type(ty: &syn::ReturnType) -> Option<MarshalAttr> {
        match ty {
            syn::ReturnType::Type(_, ty) => Self::from_defaults_by_type(&*ty),
            _ => None,
        }
    }

    pub fn from_path(path: syn::Path) -> Result<Option<MarshalAttr>, syn::Error> {
        let types = path
            .segments
            .iter()
            .filter_map(|s| match &s.arguments {
                syn::PathArguments::AngleBracketed(args) => match args.args.first() {
                    Some(syn::GenericArgument::Type(ty)) => Some(ty.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>();

        Ok(Some(MarshalAttr { path, types }))
    }

    fn from_bare_fn(bare_fn: syn::TypeBareFn) -> Result<Option<MarshalAttr>, syn::Error> {
        if bare_fn.lifetimes.is_some() {
            return Err(syn::Error::new_spanned(
                bare_fn,
                "Marshal fn may not have lifetimes",
            ));
        }

        if bare_fn.unsafety.is_some() {
            return Err(syn::Error::new_spanned(
                bare_fn,
                "Marshal fn may not have unsafety",
            ));
        }

        if bare_fn.abi.is_some() {
            return Err(syn::Error::new_spanned(
                bare_fn,
                "Marshal fn may not have abi",
            ));
        }

        if bare_fn.variadic.is_some() {
            return Err(syn::Error::new_spanned(
                bare_fn,
                "Marshal fn may not have variadic",
            ));
        }

        // TODO: not this, not here.

        Ok(None)
    }

    pub fn from_attribute(attr: syn::Attribute) -> Result<Option<MarshalAttr>, syn::Error> {
        match attr.path.segments.first() {
            Some(v) => {
                if v.ident.to_string() != "marshal" {
                    return Ok(None);
                }
            }
            None => {
                return Ok(None);
            }
        }

        let marshal_ty: syn::Type = match syn::parse2(attr.tokens) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        match marshal_ty {
            syn::Type::Paren(path) => match *path.elem {
                syn::Type::Path(path) => Self::from_path(path.path),
                syn::Type::BareFn(bare_fn) => Self::from_bare_fn(bare_fn),
                e => {
                    return Err(syn::Error::new_spanned(e, "Must be a path"));
                }
            },
            e => {
                return Err(syn::Error::new_spanned(e, "Must be a path"));
            }
        }
    }
}
