//! Provides all the initialization functions needed to make the `ministd` work


use core::ptr::NonNull;

#[cfg(all(feature="allocator", feature="spin"))]
use crate::mem::{Region, alloc, PAGE_ALIGN};
use crate::mem::kernel;
#[cfg(feature = "renderer")]
use ::renderer::MinistdRenderer;

/// initializes allocator (heap)
#[cfg(all(feature="allocator", feature="spin"))]
#[inline]
pub fn allocator(region: Region<PAGE_ALIGN>) -> Result<(), Option<&'static str>> {
    alloc::init(region)
}

/// initializes renderer
/// - needed to print text to the screen
#[cfg(all(feature="renderer", feature="spin"))]
#[inline]
pub fn renderer(fb: NonNull<u32>, width: usize, height: usize) -> Result<(), ()> {
    crate::RENDERER.lock().init(fb, width, height)
}

/// initializes metadata about kernel memory layout
/// - available in the `ministd::mem::kernel` module
pub fn memory() {
    //kernel::LAYOUT.write().init();
}

