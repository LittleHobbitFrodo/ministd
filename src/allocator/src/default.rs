
//! The default allocator functionality (thanks to the `buddy_system_allocator` crate)


use core::alloc::{GlobalAlloc, Layout};
use core::mem::MaybeUninit;
use core::ptr::{NonNull, copy_nonoverlapping, drop_in_place, null_mut};
use core::cell::UnsafeCell;

use buddy_system_allocator as allocator;

use crate::MinistdAllocator;


pub struct Heap {
    heap: UnsafeCell<allocator::Heap<32>>
}



impl Heap {
    /// Gets mutable reference to the inner value
    const fn mutable(&self) -> &mut allocator::Heap<32> {
        unsafe { &mut *self.heap.get() }
    }

    pub const fn new() -> Self {
        Self {
            heap: UnsafeCell::new(allocator::Heap::empty())
        }
    }
}


unsafe impl GlobalAlloc for Heap {

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.mutable().alloc(layout) {
            Ok(p) => p.as_ptr(),
            Err(_) => null_mut(),
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        match self.mutable().alloc(layout) {
            Ok(p) => {
                unsafe { core::ptr::write_bytes(p.as_ptr(), 0, layout.size()) };
                p.as_ptr()
            },
            Err(_) => null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return
        }
        self.mutable().dealloc(unsafe { NonNull::new_unchecked(ptr) }, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };

        let new = match self.mutable().alloc(new_layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => return null_mut()
        };

        let count = core::cmp::min(new_size, layout.size());

        unsafe {
            copy_nonoverlapping(ptr, new, count);
            self.dealloc(ptr, layout);
        }

        new

    }

}

impl MinistdAllocator for Heap {
    type AddError = ();

    unsafe fn allocate<T: Sized>(&mut self, val: T) -> Result<core::ptr::NonNull<T>, T> {
        let data: NonNull<T> = match self.mutable().alloc(Layout::new::<T>()) {
            Ok(ptr) => ptr.cast(),
            Err(_) => return Err(val)
        };

        unsafe { data.write(val); }

        Ok(data)
    }

    unsafe fn allocate_uninit<T: Sized>(&mut self) -> Result<NonNull<core::mem::MaybeUninit<T>>, ()> {
        match self.mutable().alloc(Layout::new::<T>()) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(_) => Err(())
        }
    }

    unsafe fn allocate_zeroed<T: Sized>(&mut self) -> Result<NonNull<MaybeUninit<T>>, ()> {
        match self.mutable().alloc(Layout::new::<T>()) {
            Ok(ptr) => {
                let ptr = ptr.cast();
                unsafe { ptr.write_bytes(0, 1) }
                Ok(ptr)
            },
            Err(_) => Err(())
        }
    }

    unsafe fn allocate_array<T: Sized + Clone>(&mut self, size: usize, val: T) -> Result<NonNull<[T]>, ()> {

        if size == 0 { return Err(()); }

        let layout = layout_arr::<T>(size);

        let data = NonNull::slice_from_raw_parts(self.mutable().alloc(layout)?.cast(), size);

        unsafe {
            let mut ptr: NonNull<T> = data.cast();
            for _ in 0..size {
                ptr.write(val.clone());
                ptr = ptr.add(1);
            }
        }

        Ok(data)
    }

    unsafe fn allocate_array_with<T: Sized, F: FnMut() -> T>(&mut self, size: usize, f: &mut F) -> Result<NonNull<[T]>, ()> {

        if size == 0 { return Err(()); }

        let layout = layout_arr::<T>(size);

        let data = NonNull::slice_from_raw_parts(self.mutable().alloc(layout)?.cast(), size);

        unsafe {
            let mut ptr: NonNull<T> = data.cast();
            for _ in 0..size {
                ptr.write(f());
                ptr = ptr.add(1);
            }
        }

        Ok(data)
    }

    unsafe fn allocate_array_uninit<T: Sized>(&mut self, size: usize) -> Result<NonNull<[core::mem::MaybeUninit<T>]>, ()> {

        if size == 0 { return Err(()) }

        let layout = layout_arr::<T>(size);

        Ok(NonNull::slice_from_raw_parts(self.mutable().alloc(layout)?.cast::<MaybeUninit<T>>(), size))
    }

    unsafe fn allocate_array_zeroed<T: Sized>(&mut self, size: usize) -> Result<NonNull<[MaybeUninit<T>]>, ()> {
        
        if size == 0 { return Err(()) }

        let ptr = NonNull::slice_from_raw_parts(self.mutable().alloc(layout_arr::<T>(size))?.cast(), size);

        unsafe {
            ptr.cast::<MaybeUninit<T>>().write_bytes(0, size);
        }

        Ok(ptr)
    }

    unsafe fn delete<T: Sized>(&mut self, ptr: NonNull<T>) {
        unsafe { drop_in_place(ptr.as_ptr()); }
        self.mutable().dealloc(ptr.cast(), Layout::new::<T>());
    }

    unsafe fn add_to_heap(&mut self, start: NonNull<u8>, size: usize) -> Result<(), Self::AddError> {

        if size == 0 { return Err(()) }

        let start = start.as_ptr() as usize;
        unsafe { self.mutable().add_to_heap(start, start + size) }
        Ok(())
    }

    fn total_bytes(&self) -> usize { self.mutable().stats_total_bytes() }

    fn allocated_bytes(&self) -> usize { self.mutable().stats_alloc_actual() }


    unsafe fn reallocate<T: Sized + Default>(&mut self, ptr: NonNull<[T]>, size: usize) -> Result<NonNull<[T]>, ()> {

        if size == 0 { return Err(()) }
        if size == ptr.len() { return Ok(ptr) }


        let layout = layout_arr::<T>(size);
        let data: NonNull<T> = self.mutable().alloc(layout)?.cast();

        unsafe {
            core::ptr::copy(ptr.as_ptr() as *mut T, data.as_ptr(), core::cmp::min(size, ptr.len()));
            self.mutable().dealloc(ptr.cast(), layout_arr::<T>(ptr.len()));
        }

        Ok(NonNull::slice_from_raw_parts(data, size))

    }

    unsafe fn delete_array<T: Sized>(&mut self, mut ptr: NonNull<[T]>) {
        unsafe {
            drop_in_place(ptr.as_mut());
        }
        self.mutable().dealloc(ptr.cast(), layout_arr::<T>(ptr.len()));
    }


}


/// Creates `Layout` for an array of `T`
/// - does not check whether `size` is not zero
pub(crate) fn layout_arr<T: Sized>(size: usize) -> Layout {
    unsafe {
        Layout::from_size_align_unchecked(size_of::<T>() * size, align_of::<T>())
    }
}