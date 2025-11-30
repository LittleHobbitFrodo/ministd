//	io/mod.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build



//! this module provides basic IO functionalities
//! - such as [`in`] and [`out`] instructions and better text rendering
//!   - functions to enable/disable interrupts are provided too

#[cfg(feature="renderer")]
#[macro_use]
mod text;
pub use text::*;

mod hardware;
pub use hardware::*;


//  functions below are made to not work intentionally
//  - if testing is done on other architectures than the kernel is designed for, it fails

/*#[cfg(test)]
pub use tst::*;

#[cfg(test)]
mod tst {

    pub mod int {
        use core::arch::asm;
        
        #[inline(always)]
        pub fn disable() {}

        #[inline(always)]
        pub fn enable() {}
    }

    pub fn outb(port: u16, data: u8) {}

    pub fn inb(port: u16) -> u8 { 0 }

    pub fn outw(port: u16, data: u16) {}

    pub fn inw(port: u16) -> u16 { 0 }

    pub fn outd(port: u16, data: u32) {}

    pub fn ind(port: u16) -> u32 { 0 }

    pub fn outq(port: u16, data: u64) {}

    pub fn inq(port: u16) -> u64 { 0 }

    pub fn wait() {}
}*/