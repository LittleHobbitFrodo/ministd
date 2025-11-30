//	mem/rc_inner.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

use core::{cell::Cell, mem::MaybeUninit, ops::Deref};
use core::hint::assert_unchecked;





/// Value allocated by the `Rc` type
#[repr(C)]
pub struct RcInner<T: Sized> {
    strong: Cell<u32>,
    weak: Cell<u32>,
    data: T,
}

impl<T> RcInner<T> {

    pub const fn new(val: T) -> Self {
        Self {
            strong: Cell::new(1),
            weak: Cell::new(1),
            data: val,
        }
    }

    pub const fn new_uninit() -> RcInner<MaybeUninit<T>> {
        RcInner {
            strong: Cell::new(1),
            weak: Cell::new(1),
            data: MaybeUninit::uninit(),
        }
    }

    /// Returns the status of the strong reference counter
    #[inline(always)]
    pub(crate) fn strong(&self) -> u32 {
        self.strong.get()
    }

    /// Decrements the strong reference counter
    #[inline(always)]
    pub(crate) fn dec_strong(&self) {
        self.strong.set(self.strong() - 1)
    }

    /// Returns the status of the weak reference counter
    #[inline(always)]
    pub fn weak(&self) -> u32 {
        self.weak.get()
    }

    /// Decrements the weak reference counder
    #[inline(always)]
    pub(crate) fn dec_weak(&self) {
        let weak = self.weak();
        unsafe { assert_unchecked(weak != 0); }
        self.weak.set(weak - 1)
    }

    /// Increments the strong reference counter
    #[inline(always)]
    pub(crate) fn inc_strong(&self) {
        let strong = self.strong();
        unsafe { assert_unchecked(strong != 0);}
        self.strong.set(strong + 1)
    }

    /// Increments the weak reference counter
    #[inline(always)]
    pub(crate) fn inc_weak(&self) {
        self.weak.set(self.weak() + 1)
    }

    /// Returns pointer to stored data
    #[inline(always)]
    pub fn data_as_ptr(&self) -> *mut T {
        core::ptr::addr_of!(self.data) as *mut T
    }

    /// Returns pointer to `self`
    #[inline]
    pub const fn as_ptr(&self) -> *const Self {
        self
    }

    /// Returns mutable pointer to `self`
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut Self {
        self
    }

    pub fn set_strong(&mut self, val: u32) {
        *self.strong.get_mut() = val;
    }

    pub fn set_weak(&mut self, val: u32) {
        *self.weak.get_mut() = val;
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

}

impl<T> Deref for RcInner<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}