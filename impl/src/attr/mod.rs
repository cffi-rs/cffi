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
            e => return Err(syn::Error::new_spanned(&e, "not a valid self type path")),
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
            marshaler: MarshalAttr::self_type(),
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
                return None;
            }
            Ok(Some(v)) => Some(Ok(v)),
            Err(e) => return Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>();

    let mut idents = match idents {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    Ok(idents.pop())
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
            .filter_map(|mut input| {
                // Check if we're a self-type, and short-circuit
                // TODO: this should use marshal_attr like normal typed fields
                let input = match &mut input {
                    syn::FnArg::Receiver(receiver) => {
                        if let Some(parent_type) = parent_type {
                            return Some(Mapping::self_type(receiver, parent_type));
                        } else {
                            return Some(Err(syn::Error::new_spanned(
                                &receiver,
                                "no self type found; using invoke wrong?",
                            )));
                        }
                    }
                    syn::FnArg::Typed(t) => t,
                };

                let marshaler = match input.drain_marshal_attrs() {
                    Ok(v) => v.or_else(|| MarshalAttr::from_defaults_by_type(&input.ty)),
                    Err(e) => return Some(Err(e)),
                };

                Some(Ok(Mapping {
                    output_type: *input.ty.clone(),
                    marshaler,
                }))
            })
            .collect::<Result<Vec<_>, _>>()
    }
}
