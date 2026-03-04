pub trait IntoBase64 {
    fn into_base64(&self) -> String;
}

pub trait FromBase64 {
    fn from_base64(&self) -> Result<Vec<u8>, String>;
}

impl<'s> IntoBase64 for &'s Vec<u8> {
    fn into_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self)
    }
}

impl IntoBase64 for Vec<u8> {
    fn into_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self)
    }
}

impl<'s> IntoBase64 for &'s [u8] {
    fn into_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self)
    }
}

impl FromBase64 for String {
    fn from_base64(&self) -> Result<Vec<u8>, String> {
        use base64::Engine;
        let result = base64::engine::general_purpose::STANDARD.decode(self);
        match result {
            Ok(data) => Ok(data),
            Err(err) => Err(format!("Can not decode base64: {}", err)),
        }
    }
}

impl<'s> FromBase64 for &'s String {
    fn from_base64(&self) -> Result<Vec<u8>, String> {
        use base64::Engine;
        let result = base64::engine::general_purpose::STANDARD.decode(self);
        match result {
            Ok(data) => Ok(data),
            Err(err) => Err(format!("Can not decode base64: {}", err)),
        }
    }
}

impl<'s> FromBase64 for &'s str {
    fn from_base64(&self) -> Result<Vec<u8>, String> {
        use base64::Engine;
        let result = base64::engine::general_purpose::STANDARD.decode(self);
        match result {
            Ok(data) => Ok(data),
            Err(err) => Err(format!("Can not decode base64: {}", err)),
        }
    }
}
