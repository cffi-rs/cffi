use darling::{ast::NestedMeta, FromAttributes, FromMeta};
use quote::ToTokens;

#[derive(Debug, FromMeta, Default)]
pub struct InvokeParams {
    #[darling(default)]
    pub return_marshaler: Option<syn::Path>,
    #[darling(default)]
    pub prefix: Option<String>,
    #[darling(default)]
    pub callback: bool,
}

impl FromAttributes for InvokeParams {
    fn from_attributes(attrs: &[syn::Attribute]) -> darling::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("marshal") {
                continue;
            }

            let metas = match &attr.meta {
                syn::Meta::List(ml) => NestedMeta::parse_meta_list(ml.tokens.clone())?,
                x @ syn::Meta::Path(_) => vec![NestedMeta::Meta(x.clone())],
                x @ syn::Meta::NameValue(_) => vec![NestedMeta::Meta(x.clone())],
            };

            for meta in metas {
                match meta {
                    NestedMeta::Meta(meta) => match meta {
                        syn::Meta::Path(path) => {
                            if path.is_ident("return_marshaler") {
                                result.return_marshaler = Some(path.clone());
                            } else if path.is_ident("prefix") {
                                result.prefix = Some(path.to_token_stream().to_string());
                            } else if path.is_ident("callback") {
                                result.callback = true;
                            }
                        }
                        syn::Meta::NameValue(nv) => {
                            if nv.path.is_ident("return_marshaler") {
                                result.return_marshaler = Some(match nv.value {
                                    syn::Expr::Path(p) => p.path,
                                    _ => panic!("Must be path, got: {:#?}", nv.value),
                                });
                            } else if nv.path.is_ident("prefix") {
                                result.prefix = Some(match nv.value {
                                    syn::Expr::Lit(x) => match x.lit {
                                        syn::Lit::Str(x) => quote::quote! { #x }.to_string(),
                                        _ => todo!(),
                                    },
                                    _ => todo!(),
                                });
                            } else if nv.path.is_ident("callback") {
                                result.callback = true;
                            }
                        }
                        syn::Meta::List(x) => unreachable!("A list?? HOW"),
                    },
                    NestedMeta::Lit(_) => unreachable!("A LIT??"),
                }
            }
        }

        Ok(result)
    }
}

impl InvokeParams {
    // pub fn from_attributes(attr: syn::Attribute) -> Result<Option<Self>, syn::Error> {
    //     if !attr.meta.path().is_ident("marshal") {
    //         return Ok(None);
    //     }

    //     if let Ok(list) = attr.meta.require_list() {
    //         let marshal_ty: syn::Type = match syn::parse2(list.tokens.clone()) {
    //             Ok(v) => v,
    //             Err(e) => return Err(e),
    //         };

    //         match marshal_ty {
    //             syn::Type::Path(path) => Self::from_path(path.path),
    //             syn::Type::BareFn(bare_fn) => Self::from_bare_fn(bare_fn),
    //             e => {
    //                 return Err(syn::Error::new_spanned(e, "Must be a path"));
    //             }
    //         }
    //     } else {
    //         unreachable!("Shouldn't be here")
    //     }
    // }
}
