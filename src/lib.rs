//! ONREZA platform CLI — dev, build, deploy

pub mod config;
pub mod detect;
pub mod emulator;

// Re-export commonly used types
pub use emulator::kv::KvStore;
pub use emulator::server::EmulatorServer;
