pub mod config;
pub mod crypto;
pub mod mux;
pub mod wire;

pub const PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_MAX_STREAMS: usize = 256;
