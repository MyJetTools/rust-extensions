mod data_wrapper;
mod utils;

use proc_macro::TokenStream;

#[proc_macro_derive(DataWrapper)]
pub fn data_wrapper(input: TokenStream) -> TokenStream {
    let input_as_string = input.to_string();
    let ast = syn::parse(input).unwrap();
    let result = crate::data_wrapper::generate(&ast, input_as_string);

    match result {
        Ok(result) => result.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
