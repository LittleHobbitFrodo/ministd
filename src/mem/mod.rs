//	mem/mod.rs (ministd crate)
//	this file originally belonged to the baseOS project
//		an OS template on which to build


//! Provides memory related functionalities
//! 
//! such as:
//! 1. Commonly used sizes for memory - `KB` for kilobyte, `MB` for megabyte
//!     - uses the `1024` convention
//! 2. Page sizes and counts for each supported architecture (`PAGE_SIZE`)
//! 3. Rust std-like structures, collections and smart pointers
//!     1. `Box<T>` - Allocates memory on the heap
//!         - Does not support slice and array allocation yet (use `Array<T>` for that)
//!     2. `String` - Special way to store text
//!         - Uses generics to give you control over overallocation
//!     3. `Vec<T>` - Modified version of the `std::Vec` giving control over overallocation and data align
//!     4. `Rc` - Classic reference counter
//!         - The `Weak` pointer is not yet implemented
//! 4. `Region` struct - used by the allocator to mark used memory areas


/// Standard size of one **kilobyte** (1024 bytes)
pub const KB: usize = 1024;
/// Standard size of one **megabyte** (1024 kilobytes)
pub const MB: usize = 1024 * 1024;
/// Standatd size of one **gigabyte** (1024 megabytes)
pub const GB: usize = 1024 * 1024 * 1024;
/// Standard size of one **terabyte** (1024 gigabytes)
pub const TB: usize = 1024 * 1024 * 1024 * 1024;

/// Constant that shows the size of one page (target specific)
/// - in bytes
pub const PAGE_SIZE: usize = 4096;

/// Constant that shows the align of one page (target specific)
/// - in bytes
pub const PAGE_ALIGN: usize = 4096;


pub use core::mem::needs_drop;

mod readonly;
pub use readonly::ReadOnly;

pub mod kernel;

#[cfg(all(feature="allocator", feature="spin"))]
pub mod alloc;

#[cfg(all(feature="box", feature="allocator", feature="spin"))]
pub mod boxed;
#[cfg(all(feature="box", feature="allocator", feature="spin"))]
pub mod array;
#[cfg(all(feature="allocator", feature="spin"))]
mod dynamic_buffer;

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
pub mod string;
#[cfg(all(feature="vector", feature="allocator", feature="spin"))]
pub mod vec;
#[cfg(all(feature="rc", feature="allocator", feature="spin"))]
pub mod rc;

#[cfg(all(feature="allocator", feature="spin"))]
pub use dynamic_buffer::DynamicBuffer;

pub use crate::convert::Align;
pub use core::mem::{ManuallyDrop, MaybeUninit};








/// Region represents memory region
/// 
/// It holds virtual address, physical address and size of the memory region
/// 
/// Generic parameter: ALIGN
/// - **forces the align** of addresses and size
///   - the `ministd::mem::Align` trait is used to align values
#[derive(Copy, Clone)]
pub struct Region<const ALIGN: usize = PAGE_ALIGN> {
    virt: usize,    //  usize is only inner representation
    phys: usize,
    size: usize,
}

const fn const_align_usize(val: usize, align: usize) -> usize {
    (val + align-1) & !(align-1)
}


impl<const ALIGN: usize> Region<ALIGN> {

    

    /// Constructs Region with given information
    /// - `virt: usize` is used because of `*const u8` cannot be aligned in `const fn`
    pub const fn new(virt: usize, phys: usize, size: usize) -> Self {
        Self {
            virt: const_align_usize(virt, ALIGN),
            phys: const_align_usize(phys, ALIGN),
            size: const_align_usize(size, ALIGN),
        }
    }

    /// Constructs Region with given information, does not align values
    /// - `virt: usize` is used because of `*const u8` cannot be aligned in `const fn`
    pub const unsafe fn new_unchecked(virt: usize, phys: usize, size: usize) -> Self {
        Self {
            virt: virt,
            phys,
            size,
        }
    }

    /// Constructs empty Region
    /// - addresses are set to `null`
    /// - `size = 0`
    pub const fn empty() -> Self {
        Self {
            virt: 0,
            phys: 0,
            size: 0,
        }
    }

    /// Moves the virtual address to specified value
    /// - the new address is aligned to `ALIGN`
    #[inline(always)]
    pub fn move_to(&mut self, virt: *const u8) {
        self.virt = const_align_usize(virt.addr(), ALIGN);
    }

    /// Moves the virtual address to specified value
    /// - does not align the new address
    #[inline(always)]
    pub unsafe fn move_to_unchecked(&mut self, virt: *const u8) {
        self.virt = virt as usize;
    }

    /// Moves the virtual address by specified amount of bytes
    /// - returns `Err` on overflow
    /// - the address is calculated and then aligned to `ALIGN`
    #[inline]
    pub fn move_by(&mut self, by: isize) -> Result<(), ()> {

        self.virt = (  self.virt.checked_add_signed(by).ok_or(())?  ).align(ALIGN);

        Ok(())

    }

    /// Moves the virtual address by specified amount of bytes
    /// - does not check for overflow
    /// - the address is calculated and then aligned to `ALIGN`
    #[inline]
    pub fn move_by_unchecked(&mut self, by: isize) {
        self.virt = unsafe { (self.virt as *const u8).offset(by) as usize }.align(ALIGN);
    }

    /// Moves the virtual address by specified amount of bytes
    /// - does not check for overflow
    /// - does not align thw calculated address
    pub unsafe fn move_by_unckecked_unaligned(&mut self, by: isize) {
        self.virt = unsafe { (self.virt as *const u8).offset(by) } as usize
    }

    /// Moves the physical address to specified place
    /// - the physical address is aligned to `ALIGN`
    pub const fn reallocate(&mut self, phys: usize) {
        self.phys = const_align_usize(phys, ALIGN);
    }

    /// Moves the physical address to specified place
    /// - does not align the address
    pub const unsafe fn reallocate_unchecked(&mut self, phys: usize) {
        self.phys = phys;
    }

    /// Resizes the region
    pub const fn resize(&mut self, size: usize) {
        self.size = const_align_usize(size, ALIGN);
    }

    /// Adds some contignous memory to the Region
    /// - the calculated size is the aligned to `ALIGN`
    pub const fn enlarge(&mut self, by: usize) {
        self.size = const_align_usize(self.size + by, ALIGN);
    }

    /// Adds some contignous memory to the region
    /// - does not align the size
    pub const fn enlarge_unckecked(&mut self, by: usize) {
        self.size += by;
    }

    /// Shrinks the region by specified value
    /// - returns `Err` if resulted size is less than or equal to zero
    #[inline]
    pub fn shrink(&mut self, by: usize) -> Result<(), ()> {

        self.size = (self.size.checked_sub(by).ok_or(())?).align(ALIGN);

        Ok(())

    }

    /// Shrinks the region by specified value
    /// - does not check for overflow
    /// - the calculated size is aligned to `ALIGN`
    #[inline]
    pub unsafe fn shrink_unchecked(&mut self, by: usize) {
        self.size = (self.size - by).align(ALIGN);
    }

    /// Shrinks the region by specified value
    /// - does not check for overflow
    /// - does not align the size
    pub unsafe fn shrink_unchecked_unaligned(&mut self, by: usize) {
        self.size -= by;
    }

    /// Returns the starting virtual address
    pub const fn virt(&self) -> *const u8 {
        self.virt as *const u8
    }

    /// Returns the starting physical address
    pub const fn phys(&self) -> usize {
        self.phys
    }

    /// Returns the size of the region
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Returns the forced align for this instance
    pub const fn align(&self) -> usize {
        ALIGN
    }

}



