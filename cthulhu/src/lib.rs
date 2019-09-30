extern crate proc_macro;

use darling::FromMeta;
use proc_macro::TokenStream;
use syn::AttributeArgs;

#[proc_macro_attribute]
pub fn invoke(params: TokenStream, function: TokenStream) -> TokenStream {
    let params = syn::parse_macro_input!(params as AttributeArgs);

    let params = match cthulhu_macro::InvokeParams::from_list(&params) {
        Ok(v) => v,
        Err(err) => return err.write_errors().into(),
    };

    match cthulhu_macro::call_with(params, function.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => {
            TokenStream::from(syn::Error::new(err.span(), err.to_string()).to_compile_error())
        }
    }
}
