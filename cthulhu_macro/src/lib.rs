extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn invoke(params: TokenStream, function: TokenStream) -> TokenStream {
    match cthulhu_calling::call_with(params.into(), function.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => TokenStream::from(
            syn::Error::new(err.span(), format!("failed to invoke cthulhu: {}", err.to_string()))
                .to_compile_error(),
        ),
    }
}
