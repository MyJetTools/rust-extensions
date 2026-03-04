use std::str::FromStr;

pub struct ExtractedType<'s> {
    pub tp: &'s str,
}


impl<'s> ExtractedType<'s> {
    pub fn new(tp: &'s str) -> Self {
        Self { tp }
    }

    pub fn is_arc(&self)->bool{
        self.tp.starts_with("Arc<")
    }


    pub fn get_as_value_fn(&self)->proc_macro2::TokenStream{
        let value = format!("as_{}", self.tp);

        return proc_macro2::TokenStream::from_str(value.as_str()).unwrap();
    }
}

pub fn extract_type<'s>(input_as_string: &'s str) -> (proc_macro2::TokenStream, ExtractedType<'s>) {
    let open_index = input_as_string.find("(").unwrap();
    let close_index = input_as_string.find(")").unwrap();

    let type_str = &input_as_string[open_index + 1..close_index];

    let type_tokens = proc_macro2::TokenStream::from_str(type_str);
    match type_tokens {
        Ok(tokens) => (tokens, ExtractedType { tp: type_str }),
        Err(_) => {
            panic!("Invalid type: {}", type_str);
        }
    }
}
