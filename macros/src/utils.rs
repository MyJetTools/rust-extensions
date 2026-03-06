use std::str::FromStr;




pub enum WrappedType<'s>{
    String,
    Other(&'s str)
}

impl<'s> WrappedType<'s>{

    pub fn from_str(str: &'s str)->Self{
        if str == "String"{
            return Self::String;
        }

        Self::Other(str)
    }

    pub fn is_string(&self)->bool{
        match self{
            WrappedType::String => true,
            WrappedType::Other(_) => false,
        }
    }

    pub fn as_str(&self)->&str{
        match self{
            WrappedType::String => "string",
            WrappedType::Other(tp) => tp,
        }
    }
}


pub struct ExtractedType<'s> {
    pub tp_str: &'s str,
    pub tp: proc_macro2::TokenStream,
}


impl<'s> ExtractedType<'s> {

    pub fn get_type_and_wrapper(&'s self)->(WrappedType<'s>, bool){

        if let Some(start_index) = self.tp_str.find('<'){
            let Some(end_index) = self.tp_str.find(">") else{
                return (WrappedType::from_str(self.tp_str) , false);
            };

            let type_str = &self.tp_str[start_index + 1..end_index].trim();

            return (WrappedType::from_str(type_str), true);
        }
        

        (WrappedType::from_str(self.tp_str), false)
    }

    pub fn get_as_value_fn(&self)->proc_macro2::TokenStream{

        let (wrapped_tp, ref_count) = self.get_type_and_wrapper();

        if ref_count{
            if wrapped_tp.is_string(){
                let tp = &self.tp;
                return   quote::quote! {
                    pub fn as_str(&self) -> &str {
                        self.0.as_str()
                    }

                    pub fn to_string(&self) -> String {
                        self.0.to_string()
                    }

                    pub fn as_ref(&self) -> &#tp{
                        &self.0
                    }
                }
            }
        }

        let value = format!("as_{}", wrapped_tp.as_str());

        let fn_name =  proc_macro2::TokenStream::from_str(value.as_str()).unwrap();

        let tp = &self.tp;

        quote::quote! {
            pub fn #fn_name(&self) -> #tp {
                self.0
            }

            pub fn as_ref(&self) -> &#tp{
                &self.0
            }
        }
    }


    pub fn get_into_fn(&self, struct_name: &syn::Ident)->  proc_macro2::TokenStream {

          let (wrapped_tp, ref_count) = self.get_type_and_wrapper();

          let tp = &self.tp;

          if ref_count{
            if wrapped_tp.is_string(){
                

                return quote::quote! {

                    impl Into<#struct_name> for #tp {
                      fn into(self) -> #struct_name {
                        #struct_name::new(self)
                      }
                    }

                    impl Into<#struct_name> for String {
                        fn into(self) -> #struct_name {
                            #struct_name::new(self.into())
                        }
                    }
                };

            }
               
          }


        quote::quote! {

          impl Into<#struct_name> for #tp {
            fn into(self) -> #struct_name {
                #struct_name::new(self)
            }
          }
        }
    }
}

pub fn extract_type<'s>(input_as_string: &'s str) -> ExtractedType<'s> {
    let open_index = input_as_string.find("(").unwrap();
    let close_index = input_as_string.find(")").unwrap();

    let type_str = &input_as_string[open_index + 1..close_index].trim();

    let type_tokens = proc_macro2::TokenStream::from_str(type_str);
    match type_tokens {
        Ok(tokens) => ExtractedType { tp_str: type_str, tp: tokens },
        Err(_) => {
            panic!("Invalid type: {}", type_str);
        }
    }
}
