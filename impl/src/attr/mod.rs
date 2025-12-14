pub mod invoke;
pub mod marshal;

use marshal::MarshalAttr;

#[derive(Debug)]
pub struct Mapping {
    pub output_type: syn::Type,
    pub marshaler: Option<MarshalAttr>,
}

impl Mapping {
    pub fn self_type(receiver: &syn::Receiver, parent: &syn::Type) -> Result<Mapping, syn::Error> {
        let syn::Receiver {
            reference,
            mutability,
            ..
        } = receiver.clone();

        let path = match parent {
            syn::Type::Path(path) => path,
            e => return Err(syn::Error::new_spanned(e, "not a valid self type path")),
        };

        let output_type = match (reference, mutability) {
            (None, _) => syn::Type::Path(path.clone()),
            (Some((and_token, lifetime)), mutability) => syn::Type::Reference(syn::TypeReference {
                and_token,
                lifetime,
                mutability,
                elem: Box::new(syn::Type::Path(path.clone())),
            }),
        };

        Ok(Mapping {
            output_type,
            marshaler: MarshalAttr::self_type(parent),
        })
    }
}

pub(crate) trait AttrExt {
    fn drain_marshal_attrs(&mut self) -> Result<Option<MarshalAttr>, syn::Error>;
}

#[inline]
fn drain_marshal_attrs(attrs: &mut Vec<syn::Attribute>) -> Result<Option<MarshalAttr>, syn::Error> {
    let mut unhandled_attrs = vec![];
    std::mem::swap(attrs, &mut unhandled_attrs);

    let idents = unhandled_attrs
        .into_iter()
        .filter_map(|item| match MarshalAttr::from_attribute(item.clone()) {
            Ok(None) => {
                attrs.push(item);
                None
            }
            Ok(Some(v)) => Some(Ok(v)),
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(idents.into_iter().last())
}

impl AttrExt for syn::PatType {
    fn drain_marshal_attrs(&mut self) -> Result<Option<MarshalAttr>, syn::Error> {
        drain_marshal_attrs(&mut self.attrs)
    }
}

impl AttrExt for syn::FnArg {
    fn drain_marshal_attrs(&mut self) -> Result<Option<MarshalAttr>, syn::Error> {
        match self {
            syn::FnArg::Receiver(receiver) => drain_marshal_attrs(&mut receiver.attrs),
            syn::FnArg::Typed(typed) => drain_marshal_attrs(&mut typed.attrs),
        }
    }
}

pub(crate) trait SignatureExt {
    fn drain_mappings(
        &mut self,
        parent_type: Option<&syn::Type>,
    ) -> Result<Vec<Mapping>, syn::Error>;
}

impl SignatureExt for syn::Signature {
    fn drain_mappings(
        &mut self,
        parent_type: Option<&syn::Type>,
    ) -> Result<Vec<Mapping>, syn::Error> {
        self.inputs
            .iter_mut()
            .map(|input| {
                // Check if we're a self-type, and short-circuit
                let input = match input {
                    syn::FnArg::Receiver(receiver) => {
                        if let Some(parent_type) = parent_type {
                            return Mapping::self_type(receiver, parent_type);
                        } else {
                            return Err(syn::Error::new_spanned(
                                receiver,
                                "no self type found; using invoke wrong?",
                            ));
                        }
                    }
                    syn::FnArg::Typed(t) => t,
                };

                let marshaler = input
                    .drain_marshal_attrs()?
                    .or_else(|| MarshalAttr::from_defaults_by_type(&input.ty));

                Ok(Mapping {
                    output_type: *input.ty.clone(),
                    marshaler,
                })
            })
            .collect()
    }
}
