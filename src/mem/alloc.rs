//	mem/heap.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build


//  this file implements features of the buddy_system_allocator

//! This is your kernel heap module
//! - allocates/deallocates memory


/*pub use allocator::Heap;
pub use allocator::MinistdAllocator as Allocator;
use spin::Mutex;

pub type LockedHeap = spin::Mutex<Heap>;

pub static ALLOCATOR: Mutex<()> = Mutex::new(());*/

//#[global_allocator]
//pub static ALLOCATOR: LockedHeap = LockedHeap::new(empty!(););

//static ALLOCATOR: 

//pub use buddy_system_allocator as allocator;
pub use allocator::{Heap, MinistdAllocator};
pub use core::alloc::GlobalAlloc;
pub use core::alloc::Layout;
use core::mem::MaybeUninit;
use core::ptr::drop_in_place;
use core::ptr::{copy_nonoverlapping, null_mut, NonNull};
use crate::mem::*;

use crate::spin::Mutex;

//pub type Heap = allocator::Heap<32>;

/// The default Allocator type for ministd
/// - has no members on purpose to prevent taking any memory with the use of `Global` allocator in other crates
pub struct Allocator;

impl Allocator {
    pub(crate) const fn new() -> Self { Self { } }
}

//  Representation of the `MinistdAllocator` trait
impl Allocator {

    /// Tries to allocate data of type `T`
    /// - Runs the `oom` handler upon failure, then tries again
    pub unsafe fn allocate<T: Sized>(&self, val: T) -> Result<NonNull<T>, T> {
        let mut guard = HEAP.lock();
        unsafe {
            match guard.allocate(val) {
                Ok(ptr) => Ok(ptr),
                Err(val) => return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => guard.allocate(val),
                    Err(_) => Err(val),
                }
            }
        }
    }

    /// Tries to allocate uninitialized data
    /// - Runs the `oom` handler upon failure, then tries again
    pub unsafe fn allocate_uninit<T: Sized>(&self) -> Result<NonNull<MaybeUninit<T>>, ()> {
        let mut guard = HEAP.lock();
        unsafe {
            match guard.allocate_uninit() {
                Ok(ptr) => Ok(ptr),
                Err(_) => return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => guard.allocate_uninit(),
                    Err(_) => Err(())
                }
            }
        }
    }

    /// Tries to allocate data and sets all bytes to zero
    /// - runs the `oom` handler upon failure, then tries again
    pub unsafe fn allocate_zeroed<T: Sized>(&self) -> Result<NonNull<MaybeUninit<T>>, ()> {
        let mut guard = HEAP.lock();
        unsafe {
            match guard.allocate_zeroed() {
                Ok(ptr) => Ok(ptr),
                Err(_) => return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => guard.allocate_zeroed(),
                    Err(_) => Err(())
                }
            }
        }

    }

    /// Allocates an array and uses of type T, does not use the `oom` handler
    pub unsafe fn allocate_array<T: Sized + Clone>(&self, size: usize, val: T) -> Result<NonNull<[T]>, ()> {
        unsafe { HEAP.lock().allocate_array(size, val) }
    }
    
    /// Allocates and array and uses the closure to determine the value of each element
    /// - Runs the `oom` handler upon failure, then tries again
    pub unsafe fn allocate_array_with<T: Sized, F: FnMut() -> T>(&self, size: usize, f: &mut F) -> Result<NonNull<[T]>, ()> {
        let mut guard = HEAP.lock();
        unsafe {
            match guard.allocate_array_with(size, f) {
                Ok(ptr) => Ok(ptr),
                Err(_) => return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => guard.allocate_array_with(size, f),
                    Err(_) => Err(())
                }
            }
        }
    }

    /// Allocates uninitialized array of `T`
    /// - Runs the `oom` handler upon failure, then tries again
    pub unsafe fn allocate_array_uninit<T: Sized>(&self, size: usize) -> Result<NonNull<[MaybeUninit<T>]>, ()> {
        let mut guard = HEAP.lock();
        unsafe {
            match guard.allocate_array_uninit(size) {
                Ok(ptr) => Ok(ptr),
                Err(_) => return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => guard.allocate_array_uninit(size),
                    Err(_) => Err(())
                }
            }
        }
    }

    /// Allocates uninitialized array of `T` with all bytes set to `0`
    /// - Runs the `oom` handler upon failure, then tries again
    pub unsafe fn allocate_array_zeroed<T: Sized>(&self, size: usize) -> Result<NonNull<[MaybeUninit<T>]>, ()> {
        let mut guard = HEAP.lock();
        unsafe {
            match guard.allocate_array_zeroed(size) {
                Ok(ptr) => Ok(ptr),
                Err(_) => return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => guard.allocate_array_zeroed(size),
                    Err(_) => Err(())
                }
            }
        }
    }

    /// Deallocates the pointer and `drop`s the inner value if needed
    #[inline]
    pub unsafe fn delete<T: Sized>(&self, ptr: NonNull<T>) {
        unsafe { HEAP.lock().delete(ptr); }
    }

    /// Deallocates the pointer (whether it is pointing to an array or not)
    /// - `drop`s the data in the pointer
    pub unsafe fn deallocate_layout<T: Sized>(&self, mut ptr: NonNull<T>, layout: Layout) {
        unsafe {
            drop_in_place(ptr.as_mut());
            HEAP.lock().dealloc(ptr.cast().as_ptr(), layout);
        }
    }

    /// Deallocates the array and `drop`s each element if needed
    #[inline]
    pub unsafe fn delete_array<T: Sized>(&mut self, ptr: NonNull<[T]>) {
        unsafe { HEAP.lock().delete_array(ptr); }
    }

    /// Reallocates array into new buffer, running the `oom` handler upon failure, tries again if needed
    /// - Does not drop eny elements
    #[inline]
    pub unsafe fn reallocate<T: Sized + Default>(&self, ptr: NonNull<[T]>, size: usize) -> Result<NonNull<[T]>, ()> {
        let mut guard = HEAP.lock();
        unsafe {
            match guard.reallocate(ptr, size) {
                Ok(ptr) => Ok(ptr),
                Err(_) => return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => guard.reallocate(ptr, size),
                    Err(_) => Err(())
                }
            }
        }
    }


    /// Adds memory region to the allocator
    #[inline]
    pub unsafe fn add_to_heap(&self, region: Region<PAGE_ALIGN>) -> Result<(), ()> {
        unsafe { HEAP.lock().add_to_heap(NonNull::new(region.virt() as *mut u8).expect("region virtual address is NULL"), region.size()) }
    }

    /// Returns actual number of bytes in the heap
    #[inline]
    pub fn total_bytes(&self) -> usize {
        HEAP.lock().total_bytes()
    }

    /// Returns number of bytes that are allocated
    #[inline]
    pub fn allocated_bytes(&self) -> usize {
        HEAP.lock().total_bytes()
    }

}



unsafe impl GlobalAlloc for Allocator {

    /// allocates new data on the heap
    /// 
    /// if allocation fails:
    /// - runs the `oom` handler
    ///   - success: try allocation again
    ///   - failure: returns null
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {

        unsafe {
            let ptr = HEAP.lock().alloc(layout);

            if ptr.is_null() {
                let mut guard = HEAP.lock();
                return match __ministd_oom_handler(&mut guard, &self) {
                    Ok(_) => {
                        let ptr = guard.alloc(layout);
                        if ptr.is_null() {
                            return null_mut()
                        }
                        ptr
                    },
                    Err(_) => null_mut()
                }
            }

            ptr
        }
    }

    /// same as `alloc` but zeroes the allocated buffer
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {

        let data = unsafe { self.alloc(layout) };

        if data.is_null() {
            null_mut()
        } else {
            unsafe {
                core::ptr::write_bytes(data, 0, layout.size())
            }
            data
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return
        }
        unsafe { HEAP.lock().dealloc(ptr, layout); }
    }

    /// reallocates memory
    /// - does not deallocate the old buffer if allocation fails
    /// 
    /// used layout: `Layout::from_size_unchecked(new_size, layout.align())`
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        
        let new = unsafe {
            self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()))
        };

        if new.is_null() {
            return null_mut();
        }

        let count = core::cmp::min(new_size, layout.size());
        unsafe {
            copy_nonoverlapping(ptr, new, count);
            self.dealloc(ptr, layout);
        }
        new
    }

}


/// This is the global allocator for BaseOS
/// 
/// It is used by all structures that are working with heap as the default allocator
#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator::new();

/// This structure takes care of mapping all the memory regions of the heap
pub(crate) static REGIONS: Mutex<Region<PAGE_ALIGN>> = Mutex::new(Region::empty());
///// This is the Heap used by the `ALLOCATOR`
pub(crate) static HEAP: Mutex<Heap> = Mutex::new(Heap::new());
    // use Vec later


unsafe extern "Rust" {

    //  functions defined by the developer in the main crate

    //pub(crate) fn __region_finder() -> Result<Region, Option<&'static str>>;
    //pub(crate) fn out_of_memory_handler(heap: &mut MutexGuard<Heap>, allocator: &Allocator) -> Result<(), ()>;
    pub(crate) fn __ministd_oom_handler(heap: &mut crate::HeapRef, alloc: &crate::Allocator) -> Result<(), ()>;
}



//  TODO: use mutex<Vec<Region>> for heap mapping



/// Initializes heap by giving the allocator memory region
/// 
/// You can check where the heap is with the [`ministd::mem::heap::REGION`] variable
/// - please do not change it
pub(crate) fn init(region: Region<PAGE_ALIGN>) -> Result<(), Option<&'static str>> {

    let ptr = NonNull::new(region.virt() as *mut u8).expect("region virtual address is NULL");

    let mut alloc = HEAP.lock();

    unsafe { alloc.add_to_heap(ptr, region.size()) };

    Ok(())

}

pub(crate) const fn layout_arr<T: Sized>(size: usize) -> Layout {
    unsafe {
        Layout::from_size_align_unchecked(size_of::<T>() * size, align_of::<T>())
    }
}