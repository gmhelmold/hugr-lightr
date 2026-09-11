//! BuildKit extensions — new parser + stub modules (contract C-04-NEW).
//! Frozen interface: does NOT edit `build/exec.rs`.

pub mod cache;
pub mod secrets;
pub mod ssh;

pub use cache::{parse_cache_from, CacheFrom};
pub use secrets::{parse_secret, SecretEntry};
pub use ssh::{parse_ssh, SshEntry};
