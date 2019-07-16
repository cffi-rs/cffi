extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn call_of_cthulhu(params: TokenStream, function: TokenStream) -> TokenStream {
    cthulhu_calling::call_with(params.into(), function.into()).unwrap().into()
}
