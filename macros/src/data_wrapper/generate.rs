use quote::quote;

pub fn generate(
    ast: &syn::DeriveInput,
    input_as_string: String,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let struct_name = &ast.ident;

    let extracted_tp = crate::utils::extract_type(input_as_string.as_str());


    
    let as_value_fn = extracted_tp.get_as_value_fn();

    let tp = &extracted_tp.tp;

    let result = quote! {

        impl #struct_name{
              pub fn new(value: #tp) -> Self {
                 Self(value)
              }

              #as_value_fn
        }

        impl Into<#struct_name> for #tp {
            fn into(self) -> #struct_name {
                #struct_name::new(self)
        }
}

    };

    Ok(result.into())
}
