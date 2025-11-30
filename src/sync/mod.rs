//  mem/sync/mod.rs (ministd crate)
//  this file originally belonged to baseOS project
//      an OS template on which to build

//! Provides the `Arc` smart pointer (without the `Weak` pointer) and re-exports all usefult strucutres from the `spin` crate
//! - such as `Once`, `Lazy`, `Mutex` and `RwLock`

#[cfg(all(feature="allocator", feature="spin", feature="rc"))]
mod arc;
#[cfg(all(feature="allocator", feature="spin", feature="rc"))]
pub use arc::Arc;

pub use spin::{Once, Lazy, Mutex, RwLock};