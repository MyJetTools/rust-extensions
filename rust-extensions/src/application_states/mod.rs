mod application_states;
pub use application_states::*;
// `AppStates` reacts to SIGTERM/SIGINT through `signal-hook` - there are no POSIX signals in a
// browser, so only the trait above crosses over to wasm.
#[cfg(not(target_arch = "wasm32"))]
mod app_states;
#[cfg(not(target_arch = "wasm32"))]
pub use app_states::*;
