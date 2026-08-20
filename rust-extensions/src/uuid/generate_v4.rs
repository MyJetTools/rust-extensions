/// Generates a random uuid-v4 in its lowercase hyphenated form.
///
/// On `wasm32` the `uuid` crate is not used: its v4 generator needs `getrandom`, which has no
/// backend on `wasm32-unknown-unknown` without extra build flags. The browser gives us the same
/// thing for free through `crypto.randomUUID()`.
#[cfg(target_arch = "wasm32")]
pub fn generate_v4() -> String {
    let result = js_sys::eval("crypto.randomUUID()").expect("Failed to eval crypto.randomUUID()");

    result
        .as_string()
        .expect("crypto.randomUUID() did not return a string")
}

/// Generates a random uuid-v4 in its lowercase hyphenated form.
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_v4() -> String {
    // leading `::` - inside `mod uuid` the bare `uuid::` would be ambiguous with this module
    ::uuid::Uuid::new_v4().to_string()
}
