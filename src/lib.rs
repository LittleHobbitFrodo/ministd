//	lib.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

//! # The `ministd` crate
//! The ministd crate is the core of the `BaseOS` project. It is intended to do some of the work of the rust standard library and give you (the OS developer) some useful functionalities to kickstart your OS development

#![no_std]
#![no_main]

use core::ops::Deref;


pub use core::pin::Pin;
//pub use core::intrinsics::{unlikely, likely};


//  used modules
pub mod mem;
#[cfg(feature = "renderer")]
pub mod renderer;
#[macro_use]
pub mod io;
pub mod convert;
pub mod init;
pub mod sync;


//#[macro_use]
//pub(crate) mod tests;
//pub use tests::Test;

//  modules
#[cfg(all(feature="renderer", feature="spin"))]
pub use renderer::{RENDERER, Color};

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
pub use mem::string::{self, String};
#[cfg(all(feature="string", feature="allocator", feature="spin"))]
pub use mem::string::ToString;

#[cfg(all(feature="vector", feature="allocator", feature="spin"))]
pub use mem::vec::{self, Vec};

#[cfg(all(feature="box", feature="allocator", feature="spin"))]
pub use mem::boxed::Box;
#[cfg(all(feature="box", feature="allocator", feature="spin"))]
pub use mem::array::Array;

#[cfg(all(feature="allocator", feature="spin"))]
pub use mem::alloc::{self, ALLOCATOR, Allocator};

#[cfg(all(feature="rc", feature="allocator", feature="spin"))]
pub use mem::rc::Rc;

pub mod borrow;
pub use borrow::*;

#[cfg(feature="spin")]
pub use spin;
use proc_macro;

pub use proc_macro::{/*entry, */oom, /*region_finder, testing, test_only*/};

//  remote crates
#[cfg(all(feature="allocator", feature="hashmap"))]
pub use hashbrown;

pub mod assert {
    pub use static_assertions::*;
}

#[cfg(feature="spin")]
pub use spin::{Mutex, MutexGuard,
    RwLock, RwLockReadGuard, RwLockWriteGuard, RwLockUpgradableGuard,
    Lazy, Barrier, Once};

#[cfg(all(feature="hashmap", feature="allocator", feature="spin"))]
pub use hashbrown::{HashMap, HashSet, HashTable};

use core::arch::asm;
use core::hint::spin_loop;
pub use core::convert::{Infallible, From, TryFrom, Into, TryInto};


#[cfg(feature = "allocator")]
pub type HeapRef<'l> = spin::MutexGuard<'l, allocator::Heap>;


/// disables interrupts and halts the CPU
pub fn hang() -> ! {
    loop {
        io::int::disable();
        unsafe { asm!("hlt"); }
        spin_loop();
    }
}


/// Allows cloning if failure is possible
pub trait TryClone {
    type Error;
    fn try_clone(&self) -> Result<Self, Self::Error>
    where Self: Sized;
}

/// # Nothing
/// 
/// This structure represents ..., well ... nothing  
/// 
/// Usage:
/// - No data while returning `Err` but still needs to be constructed
#[derive(Copy, Clone)]
pub struct Nothing();

impl Default for Nothing {
    #[inline(always)]
    fn default() -> Self {
        Nothing()
    }
}



/// structure used for sigle-threaded immutable data access
pub struct Immutable<T: Sized> {
    data: T,
}

impl<T: Sized> Immutable<T> {
    pub const fn new(val: T) -> Self {
        Self {
            data: val,
        }
    }
}

impl<T: Sized> Deref for Immutable<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// `Either` allows you to choose between one type or another
pub enum Either<A: Sized, B: Sized> {
    First(A),
    Second(B)
}


#[cfg(all(feature="allocator", feature="spin", feature="string"))]
pub static PANIC_FMT_MSG: RwLock<Option<&'static str>> = RwLock::new(None);

/// Makes support for formatted panic messages possible
#[cfg(all(feature="allocator", feature="spin", feature="string"))]
#[macro_export]
macro_rules! panic_fmt {
    ($($arg:tt)*) => {{

        use core::fmt::Write;
        use $crate::String;

        let mut msg: String = String::with_capacity(64);

        if let Err(_) = write!(&mut msg, $($arg)*) {
            panic!();
        }

        *$crate::PANIC_FMT_MSG.write() = Some(msg.leak());

        panic!();
        

    }};

    () => {
        panic!();
    }
}


