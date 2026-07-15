//! Владение TDJSON transport, ordered state и Telegram workflows.

#![deny(unsafe_code)]

pub mod schema;
pub mod transport;

#[allow(unsafe_code)]
mod tdjson_native;

pub use tdjson_native::NativeTdJson;
