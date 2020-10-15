use darling::FromMeta;

#[derive(Debug, FromMeta, Default)]
pub struct InvokeParams {
    #[darling(default)]
    pub return_marshaler: Option<syn::Path>,
    #[darling(default)]
    pub prefix: Option<String>,
    #[darling(default)]
    pub callback: bool,
}
