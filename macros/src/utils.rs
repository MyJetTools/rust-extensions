use std::str::FromStr;

pub fn extract_type(input_as_string: &str) -> (proc_macro2::TokenStream, &str) {
    let open_index = input_as_string.find("(").unwrap();
    let close_index = input_as_string.find(")").unwrap();

    let type_str = &input_as_string[open_index + 1..close_index];

    let type_tokens = proc_macro2::TokenStream::from_str(type_str);
    match type_tokens {
        Ok(tokens) => (tokens, type_str),
        Err(_) => {
            panic!("Invalid type: {}", type_str);
        }
    }
}
