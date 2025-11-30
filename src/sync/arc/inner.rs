//  mem/sync/arc/inner.rs (ministd crate)
//  this file originally belonged to baseOS project
//      an OS template on which to build


use core::{alloc::Layout, sync::atomic::{AtomicUsize, Ordering}};


#[repr(C)]
pub(crate) struct ArcInner<T: Sized> {
    pub strong: AtomicUsize,

    /// Does not 
    pub weak: AtomicUsize,

    pub data: T,
}

unsafe impl<T: Sized + Sync + Send> Send for ArcInner<T> {}
unsafe impl<T: Sized + Sync + Send> Sync for ArcInner<T> {}

impl<T: Sized> ArcInner<T> {

    /// Creates layout for specific value
    pub(crate) fn layout_for_value(layout: Layout) -> Layout {
        Layout::new::<ArcInner<()>>().extend(layout).unwrap().0.pad_to_align()
    }

    /// Creates layout for `ArcInner<T>`
    pub(crate) fn layout() -> Layout
    where T: Sized {
        Layout::new::<Self>()
    }

    /// Returns strong reference count
    pub fn strong(&self) -> usize {
        self.strong.load(Ordering::Relaxed)
    }

    /// Returns weak reference count
    pub fn weak(&self) -> usize {
        self.weak.load(Ordering::Relaxed)
    }

    /// Returns reference to the `data` field
    pub(crate) const fn get_ref(&self) -> &T {
        &self.data
    }

    /// Returns mutable reference to the `data` field
    pub(crate) const fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// Returns pointer to `self`
    pub(crate) const fn as_ptr(&self) -> *const Self {
        self
    }

    pub(crate) const fn as_mut_ptr(&self) -> *mut Self {
        self.as_ptr() as *mut Self
    }

    pub(crate) const fn data_as_ptr(&self) -> *const T {
        &self.data
    }

    pub(crate) const fn data_as_mut_ptr(&self) -> *mut T {
        self.data_as_ptr() as *mut T
    }


    pub(super) const fn new(value: T) -> Self {
        Self {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            data: value,
        }
    }

}