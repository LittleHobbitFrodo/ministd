#![no_std]


//! The `allocator` provides modular/swappable heap functionality
//! - modular means that you can swap the default allocator for another if you want
//! This module should not be modified
//! 
//! # Exports to `ministd`
//! The only thing this crate should export is the `Heap` type with `MinistdAllocator`
//! - The allocator also has to expose the `const fn new() -> Self` method to construct it


//  use the default allocator
#[cfg(feature = "default")]
mod default;

#[cfg(feature = "default")]
pub(crate) use default as alloc;



//  use the custom allocator
#[cfg(not(feature = "default"))]
pub(crate) mod custom;

#[cfg(not(feature = "default"))]
pub(crate) use custom as alloc;


pub(crate) mod ministd_allocator_trait;
pub use ministd_allocator_trait::MinistdAllocator;

pub use alloc::Heap;
