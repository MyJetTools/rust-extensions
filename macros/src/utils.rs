use std::str::FromStr;


pub struct ExtractedType<'s> {
    pub tp: &'s str,
}


impl<'s> ExtractedType<'s> {

    pub fn get_type_and_wrapper(&'s self)->(&'s str, bool){

        if let Some(start_index) = self.tp.find('<'){
            let Some(end_index) = self.tp.find(">") else{
                return (self.tp, false);
            };

            let type_str = &self.tp[start_index + 1..end_index].trim();

            return (type_str, true);
        }
        

        (self.tp, false)
    }

    pub fn get_as_value_fn(&self)->proc_macro2::TokenStream{

        let (tp, ref_count) = self.get_type_and_wrapper();

        if ref_count{
            if tp.eq_ignore_ascii_case("string"){
                return   quote::quote! {
                    pub fn as_str(&self) -> &str {
                        self.0.as_str()
                    }
                }
            }
        }

        let value = format!("as_{}", self.tp);

        let fn_name =  proc_macro2::TokenStream::from_str(value.as_str()).unwrap();


        quote::quote! {
            pub fn #fn_name(&self) -> #tp {
                self.0
             }
        }
    }
}

pub fn extract_type<'s>(input_as_string: &'s str) -> (proc_macro2::TokenStream, ExtractedType<'s>) {
    let open_index = input_as_string.find("(").unwrap();
    let close_index = input_as_string.find(")").unwrap();

    let type_str = &input_as_string[open_index + 1..close_index].trim();

    let type_tokens = proc_macro2::TokenStream::from_str(type_str);
    match type_tokens {
        Ok(tokens) => (tokens, ExtractedType { tp: type_str }),
        Err(_) => {
            panic!("Invalid type: {}", type_str);
        }
    }
}
