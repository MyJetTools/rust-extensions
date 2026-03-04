use std::str::FromStr;

use quote::quote;

pub fn generate(
    ast: &syn::DeriveInput,
    input_as_string: String,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let struct_name = &ast.ident;

    let (tp, tp_as_str) = crate::utils::extract_type(input_as_string.as_str());

    let as_tp_fn_name = format!("as_{}", tp_as_str);

    let as_tp_fn_name = proc_macro2::TokenStream::from_str(as_tp_fn_name.as_str()).unwrap();

    let result = quote! {

        impl #struct_name{
              pub fn new(value: #tp) -> Self {
                 Self(value)
              }

              pub fn #as_tp_fn_name(&self) -> #tp {
                self.0
              }
        }

        impl Into<#struct_name> for #tp {
            fn into(self) -> #struct_name {
                #struct_name::new(self)
        }
}

    };

    Ok(result.into())
}
