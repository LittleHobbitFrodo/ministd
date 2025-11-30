//! Defines the `MinistdAllocator` trait that is implemented on the exported allocator

use core::alloc::{GlobalAlloc, Layout};
use core::mem::MaybeUninit;
use core::ptr::NonNull;

pub trait MinistdAllocator where Self: Sized + GlobalAlloc {

    /// Error type returned by the `add_to_heap()` function
    type AddError;

    /// Allocates data of tyle `T` with proper alignment
    /// - Gives back ownership upon failure
    unsafe fn allocate<T: Sized>(&mut self, val: T) -> Result<NonNull<T>, T>;

    /// Allocates uninitialized data for an instance of `T`
    unsafe fn allocate_uninit<T: Sized>(&mut self) -> Result<NonNull<MaybeUninit<T>>, ()>;


    /// Allocates data for an instance of `T` and sets all bytes to zero
    unsafe fn allocate_zeroed<T: Sized>(&mut self) -> Result<NonNull<MaybeUninit<T>>, ()>;

    /// Allocates an array of type `T`
    /// - returns `Err` if `size` is zero
    unsafe fn allocate_array<T: Sized + Clone>(&mut self, size: usize, val: T) -> Result<NonNull<[T]>, ()>;

    /// Allocates an array and uses the closure to determine the value of each element
    /// - returns `Err` if `size` is zero
    unsafe fn allocate_array_with<T: Sized, F: FnMut() -> T>(&mut self, size: usize, f: &mut F) -> Result<NonNull<[T]>, ()>;

    /// Allocates an uninitialized array
    /// - returns `Err` if `size` is zero
    unsafe fn allocate_array_uninit<T: Sized>(&mut self, size: usize) -> Result<NonNull<[MaybeUninit<T>]>, ()>;


    /// Allocates an array of type `T` and sets all bytes to zero
    /// - returns `Err` if size is zero
    unsafe fn allocate_array_zeroed<T: Sized>(&mut self, size: usize) -> Result<NonNull<[MaybeUninit<T>]>, ()>;

    /// Deallocates the pointer and `drop`s the inner value if needed
    /// - The pointer must be allocated with the `allocate()` function or has the exact memory layout as `T`
    unsafe fn delete<T: Sized>(&mut self, ptr: NonNull<T>);

    /// Deallocates the pointer and `drop`s all its elements if needed
    /// - The pointer must be allocated with the `allocate_array()` (or similar) function
    unsafe fn delete_array<T: Sized>(&mut self, ptr: NonNull<[T]>);

    /// Reallocates array into new buffer
    /// - does not drop any elements
    /// - returns `Err` if `size` is zero
    unsafe fn reallocate<T: Sized + Default>(&mut self, ptr: NonNull<[T]>, size: usize) -> Result<NonNull<[T]>, ()>;



    /// Adds range of addresses to the heap
    /// - only virtual addresses should be used
    unsafe fn add_to_heap(&mut self, start: NonNull<u8>, size: usize) -> Result<(), Self::AddError>;

    /// Returns actual number of bytes in the heap
    fn total_bytes(&self) -> usize;

    /// Returns number of bytes that are allocated
    fn allocated_bytes(&self) -> usize;

}