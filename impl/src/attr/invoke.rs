use darling::{FromAttributes, FromMeta, ast::NestedMeta};
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
                        syn::Meta::List(_) => unreachable!("A list?? HOW"),
                    },
                    NestedMeta::Lit(_) => unreachable!("A LIT??"),
                }
            }
        }

        Ok(result)
    }
}
