//  mem/vec/mod.rs (ministd crate)
//  this file originally belonged to baseOS project
//      on OS template on which to build

//! Even though `ministd::Vec` is not an exact copy of `std::Vec`, you will like it: it allows you to tweak how data is stored in memory!

use core::marker::PhantomData;

use core::alloc::Layout;
use core::borrow::{Borrow, BorrowMut};
use core::ffi::CStr;
use core::fmt::Debug;
use core::hash::Hash;
use core::hint::unreachable_unchecked;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::{copy_nonoverlapping, drop_in_place, NonNull};
use core::slice::{self, from_raw_parts, from_raw_parts_mut};
use core::ops::{Bound::*, Deref, DerefMut, Index, IndexMut, Range, RangeBounds};
use core::cmp::Ordering::*;

use crate::mem::DynamicBuffer;
use crate::{println, Cow, ToOwned, TryClone};

#[cfg(all(feature="allocator", feature="spin", feature="box"))]
use crate::Box;

#[cfg(all(feature="allocator", feature="spin", feature="string"))]
use crate::panic_fmt;




/// A contiguous growable array type, written as `Vec<T>`, short for ‘vector’
/// - This implementation will also allow you to **tweak memory management** using generic parameters
/// - This vector is not an exact representation of the `std::Vec`, all important functions are preserved, some functions are added
///   - To access the `chunks`, `windows` and other functions, use the `as_slice` function as follows: `self.as_slice().chunks()`
/// 
/// ### Generic parameters
/// 1. `T`: datatype of each element
/// 2. `STEP`: indicates how much will vector grow
///     - geometrical growth is used by default
/// 3. `ALIGN` - defines custom alignment of the data
///     - set to 0 to use `align_of::<T>()`
#[repr(transparent)]
pub struct Vec<T: Sized, const STEP: usize = 0, const ALIGN: usize = 0> {
    data: DynamicBuffer<T, STEP, ALIGN>,
}

impl<T: Sized> Vec<T> {
    /// Constructs new `Vec<T>` with `n` elements
    pub fn from_elem<const S: usize>(value: T, n: usize) -> Vec<T, S>
        where T: Clone {
        
        let mut vec = Vec::with_capacity(n);
        unsafe { vec.set_len(n) };
    
        let mut ptr = vec.as_non_null();

        for _ in 0..n {
            unsafe {
                ptr.write(value.clone());
                ptr = ptr.add(1);
            }
        }

        vec

    }

    /// Constructs new `Vec<T>`
    pub fn vec_new() -> Vec<T> {
        Vec { data: DynamicBuffer::empty() }
    }

    /// Constructs new `Vec<T>` with certain `STEP`
    pub fn vec_new_with_step<const STEP: usize>() -> Vec<T, STEP> {
        Vec { data: DynamicBuffer::empty() }
    }


}

impl<T: Sized> Vec<T> {
    /// Constructs new empty `Vec<T>` with certain `STEP`
    /// - does not allocate memory
    pub const fn new_with_step<const S: usize>() -> Vec<T, S> {
        Vec {
            data: DynamicBuffer::<T, S>::empty(),
        }
    }

    /// Constructs new empty `Vec<T>` with certain `STEP` and `ALIGN`
    /// - does not allocate eny memory
    pub const fn new_with_step_align<const S: usize, const A: usize>() -> Vec<T, S, A> {
        Vec {
            data: DynamicBuffer::<T, S, A>::empty(),
        }
    }

    /// Construcs new empty `Vec<T>` with certain `ALIGN`
    /// - does not allocate any memory
    pub const fn new_with_align<const A: usize>() -> Vec<T, 0, A> {
        Vec {
            data: DynamicBuffer::<T, 0, A>::empty(),
        }
    }

}


impl<T: Sized, const STEP: usize, const ALIGN: usize> Vec<T, STEP, ALIGN> {

    /// Describes memory layout for `Vec<T>` with certain `capacity`
    /// - is aligned to `STEP`
    pub const fn layout_for(capacity: usize) -> Layout {
        DynamicBuffer::<T, STEP, ALIGN>::layout_for(capacity)
    }

    /// Describes memory layout for some capacity without aligning to `STEP``
    pub const fn layout_for_exact(capacity: usize) -> Layout {
        DynamicBuffer::<T, STEP, ALIGN>::layout_for_exact(capacity)
    }


    /// Expands the `capacity` of the vector by `STEP`
    /// - this function always reallocates memory
    /// - **panics** if allocation fails
    #[inline(always)]
    pub fn expand(&mut self) {
        self.data.expand();
    }

    /// Tries to expand the `capacity` of the vector by `STEP`
    /// - this function always reallocated memory
    /// - returns `Err` if allocation fails
    #[inline(always)]
    pub fn try_expand(&mut self) -> Result<(), ()> {
        self.data.try_expand()
    }

    /// Constructs new empty `Vec<T>`
    /// - does not allocate any memory
    pub const fn new() -> Self {
        Self {
            data: DynamicBuffer::empty(),
        }
    }


    /// Constructs new empty `Vec` with at least the specified capacity allocated
    /// - the vector will be able to hold at least `capacity` elements without reallocating
    /// - **panics** if allocation fails
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: DynamicBuffer::with_capacity(capacity),
        }
    }


    /// Tries to construct new empty `Vec<T>` with at least the specified capacity allocated
    /// - the vector will be able to hold at least `capacity` elements without reallocating
    /// - returns `Err` if allocation fails
    #[inline]
    pub fn try_with_capacity(capacity: usize) -> Result<Self, ()> {
        Ok(Self {
            data: DynamicBuffer::try_with_capacity(capacity)?,
        })
    }

    /// Resizes the vector to certain size
    /// - **panics** if reallocation fails
    /// 
    /// If `new_len` is greater than `len`, the `Vec` is extended by the difference, with each additional slot filled with `value`. If `new_len` is less than `len`, the `Vec` is simply truncated
    pub fn resize(&mut self, size: usize, value: T)
    where T: Clone {

        match self.len().cmp(&size) {
            Equal => return,

            Less => {
                //  Append the vector
                let len = size - self.len();
                self.reserve(len);

                unsafe {
                    let mut ptr = self.data.data().add(self.len());

                    for _ in 0..len {
                        ptr.write(value.clone());
                        ptr = ptr.add(1);
                    }

                }
            },

            Greater => {
                //  Shrink the vector
                unsafe { self.truncate_unchecked(size); }
            }
        }

        self.data.size = size as u32;

    }

    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`
    /// - **panics** if allocation fails
    /// 
    /// If `new_len` is greater than `len`, the `Vec` is extended by the difference, with each additional slot filled with the result of calling the closure `f`. The return values from `f` will end up in the `Vec` in the order they have been generated
    /// 
    /// If `new_len` is less than `len`, the `Vec` is simply truncated
    pub fn resize_with<F>(&mut self, new_len: usize, mut f: F)
        where F: FnMut() -> T {
        
        match self.len().cmp(&new_len) {
            Equal => return,
            Less => {
                //  Append to the vector
                let len = new_len - self.len();
                self.reserve(len);

                unsafe {
                    let mut ptr = self.data.data().add(self.len());

                    for _ in 0..len {
                        ptr.write(f());

                        ptr = ptr.add(1);
                    }
                }
            },
            Greater => {
                //  Shrink the vector
                unsafe { self.truncate_unchecked(new_len) };
            }
        }

        self.data.size = new_len as u32;

    }

    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`
    /// - returns `Err` if allocation fails
    /// 
    /// If `new_len` is greater than `len`, the `Vec` is extended by the difference, with each additional slot filled with the result of calling the closure `f`. The return values from `f` will end up in the `Vec` in the order they have been generated
    /// 
    /// If `new_len` is less than `len`, the `Vec` is simply truncated
    pub fn try_resize_with<F>(&mut self, new_len: usize, mut f: F) -> Result<(), ()>
        where F: FnMut() -> T {
        
        match self.len().cmp(&new_len) {
            Equal => return Ok(()),
            Less => {
                //  Append to the vector
                let len = new_len - self.len();
                self.try_reserve(len)?;

                unsafe {
                    let mut ptr = self.data.data().add(self.len());

                    for _ in 0..len {
                        ptr.write(f());

                        ptr = ptr.add(1);
                    }
                }
            },
            Greater => {
                //  Shrink the vector
                unsafe { self.truncate_unchecked(new_len) };
            }
        }

        self.data.size = new_len as u32;

        Ok(())

    }

    /// Tries to resize the vector to certain size
    /// - returns `Err` if allocation fails
    /// 
    /// If `new_len` is greater than `len`, the `Vec` is extended by the difference, with each additional slot filled with `value`. If `new_len` is less than `len`, the `Vec` is simply truncated
    pub fn try_resize(&mut self, size: usize, value: T) -> Result<(), ()>
    where T: Clone {

        match self.len().cmp(&size) {

            Equal => return Ok(()),

            Less => {
                //  Append the vector
                let len = size - self.len();
                self.try_reserve(len)?;

                let slice = unsafe { slice::from_raw_parts_mut(self.as_mut_ptr().add(self.len()), len) };

                for i in slice {
                    *i = value.clone();
                }
            },

            Greater => {
                //  Shrink the vector
                unsafe { self.truncate_unchecked(size); }
            }
        }

        self.data.size = size as u32;
        
        Ok(())

    }

    /// Clones and appends all elements in a slice
    /// - **panics** if allocation fails
    pub fn extend_from_slice(&mut self, other: &[T])
    where T: Clone {

        self.reserve(other.len());

        let slice = unsafe {
            slice::from_raw_parts_mut(self.as_mut_ptr().add(self.len()), other.len())
        };

        for (i, item) in slice.iter_mut().enumerate() {
            *item = other[i].clone();
        }

        self.data.size += other.len() as u32;

    }

    /// Tries to clone and append all elements in a slice
    /// - return `Err` if allocation fails
    pub fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), ()>
    where T: Clone {

        self.try_reserve(other.len())?;

        let slice = unsafe { slice::from_raw_parts_mut(self.as_mut_ptr().add(self.len()), other.len()) };

        for (i, item) in slice.iter_mut().enumerate() {
            *item = other[i].clone();
        }

        self.data.size += other.len() as u32;

        Ok(())

    }

    /// Given a range `src`, clones a slice of elements in that range and appends it to the end
    /// - `src` must be a range that can form a valid subslice of the `Vec`
    /// - **panics** if range is out of bounds or allocation fails
    pub fn extend_from_within<R>(&mut self, src: R)
        where T: Clone, R: RangeBounds<usize> {
        
        let (start, end) = self.handle_bounds(&src);

        if start > self.len() || end > self.len() {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("slice {start}..{end} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("slice is out of bounds");
        }

        let len = end - start;

        self.reserve(len);

        unsafe {
            let slice = from_raw_parts(self.as_ptr().add(start), len);

            let mut ptr = self.data.data().add(self.len());

            for i in slice {
                ptr.write(i.clone());

                ptr = ptr.add(1);
            }
        }

        self.data.size += len as u32;

    }

    /// Given a range `src`, clones a slice of elements in that range and appends it to the end
    /// - `src` must be a range that can form a valid subslice of the `Vec`
    /// - **panics** if range is out of bounds
    /// - returns `Err` if allocation fails
    pub fn try_extend_from_within<R>(&mut self, src: R) -> Result<(), ()>
        where T: Clone, R: RangeBounds<usize> {
        
        let (start, end) = self.handle_bounds(&src);

        if start > self.len() || end > self.len() {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("slice {start}..{end} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("slice is out of bounds");
        }

        let len = end - start;

        self.try_reserve(len)?;

        unsafe {
            let slice = from_raw_parts(self.as_ptr().add(start), len);

            let mut ptr = self.data.data().add(self.len());

            for i in slice {
                ptr.write(i.clone());

                ptr = ptr.add(1);
            }
        }

        self.data.size += len as u32;

        Ok(())

    }


    /// Reserves capacity for at least `additional` more elements
    /// - **panics** if allocation fails
    /// - `capacity` will be greater than or equal to `self.len() + additional` 
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let min = self.len() + additional;
        if self.capacity() < min {
            self.data.resize(min);
        }
    }

    /// Tries to reserve capacity for at least `additional` more elements
    /// - returns `Err` if allocation fails
    /// - `capacity` will be greater than or equal to `self.len() + additional` 
    #[inline(always)]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), ()> {
        let min = self.len() + additional;
        if self.capacity() < min {
            self.data.try_resize(min)
        } else {
            Ok(())
        }
    }

    /// Reserves the minimum capacity for at least `additional` more elements
    /// - unlike `reserve`, this does not overallocate memory
    /// - **panics** if allocation fails
    /// - `capacity` will be greater than or equal to `self.len() + additional` 
    #[inline(always)]
    pub fn reserve_exact(&mut self, additional: usize) {
        let min = self.len() + additional;
        if self.capacity() < min {
            self.data.resize_exact(min);
        }
    }


    /// Tries to reserve the minimum capacity for at least `additional` more elements
    /// - unlike `try_reserve`, this does not overallocate memory
    /// - returns `Err` if allocation fails
    /// - `capacity` will be greater than or equal to `self.len() + additional` 
    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), ()> {
        let min = self.len() + additional;
        if self.capacity() < min {
            self.data.try_resize_exact(min)
        } else {
            Ok(())
        }
    }

    /// Appends one element at the end of the vector
    /// - **panics** if allocation fails
    pub fn push(&mut self, val: T) {
        if self.len() == self.capacity() {
            self.data.expand();
        }
        
        unsafe {
            self.data.as_ptr().add(self.len()).write(val);
        }
        self.data.size += 1;
    }

    /// Tries to append one element at the end of the vector
    /// - returns `Err` if allocation fails
    ///     - in this case returns the ownership of `val`
    pub fn try_push(&mut self, val: T) -> Result<(), T> {
        if self.len() == self.capacity() {
            if let Err(_) = self.data.try_expand() {
                return Err(val);
            }
        }
        unsafe {
            self.data.as_ptr().add(self.len()).write(val);
        }

        self.data.size += 1;

        Ok(())

    }

    /// Appends one element to the vector and returns mutable reference to the pushed element
    /// - **panics** if allocation fails
    pub fn push_mut(&mut self, value: T) -> &mut T {
        if self.len() == self.capacity() {
            self.expand();
        }
        unsafe {
            let mut ptr = self.data.as_non_null().add(self.len());
            ptr.write(value);
            self.data.size += 1;
            ptr.as_mut()
        }
    }

    /// Tries to append one element to the vector and returns mutable reference to the pushed element
    /// - returns `Err` if allocation fails
    pub fn try_push_mut(&mut self, value: T) -> Result<&mut T, T> {
        if self.len() == self.capacity() {
            if let Err(_) = self.try_expand() {
                return Err(value);
            }
        }
        unsafe {
            let mut ptr = self.data.as_non_null().add(self.len());
            ptr.write(value);
            self.data.size += 1;
            Ok(ptr.as_mut())
        }
    }



    /// Appends the vector if there is enough spare space in allocated memory
    pub fn push_within_capacity(&mut self, val: T) -> Result<(), T> {
        if self.len() == self.capacity() {
            return Err(val);
        } else {
            unsafe {
                self.data.as_ptr().add(self.len()).write(val);
            }
            self.data.size += 1;
            Ok(())
        }
    }

    /// Appends the vector
    /// - does not check for `capacity`
    /// - use only if you are sure that `capacity` will not be exceeded
    #[inline]
    pub unsafe fn push_within_capacity_unchecked(&mut self, val: T) {
        unsafe {
            self.as_mut_ptr().add(self.len()).write(val);
        }
        self.data.size += 1;
    }

    /// Appens the vector if there is enough spare space in allocated memory and returns mutable reference to the pushed element
    pub fn push_mut_within_capacity(&mut self, value: T) -> Result<&mut T, T> {
        if self.len() == self.capacity() {
            return Err(value);
        } else {
            unsafe {
                let mut ptr = self.data.as_non_null().add(self.len());
                ptr.write(value);
                self.data.size += 1;
                Ok(ptr.as_mut())
            }
        }
    }

    /// Shrinks the capacity of the vector as much as possible
    /// - **panics** if allocation fails
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.data.resize_exact(self.len());
    }

    /// Shrinks the capacity of the vector as much as possible
    /// - returns `Err` if allocation fails
    #[inline]
    pub fn try_shrink_to_fit(&mut self) -> Result<(), ()> {
        self.data.try_resize_exact(self.len())
    }

    /// Shrinks the vector to some size while dropping all elements that will not be preserved
    /// - **panics** if allocation fails
    pub fn shrink_to(&mut self, size: usize) {
        if size == 0 { return; }
        let wanted = core::cmp::max(size, self.len());
        if self.capacity() > wanted {
            self.data.resize(wanted);
        }
    }

    /// Tries to shrink the vector to some size while dropping all elements that will not be preserved
    /// - returns `Err` if allocation fails
    pub fn try_shrink_to(&mut self, size: usize) -> Result<(), ()> {
        if self.capacity() > size {
            if size < self.len() {
                unsafe {
                    drop_in_place(from_raw_parts_mut(self.data.as_ptr()
                    .add(size), self.len()));
                }
                self.data.size = size as u32;
            }

            self.data.try_resize_exact(size)
        } else {
            Ok(())
        }
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest
    /// 
    /// If `len` is greater or equal to the vector’s current length, this has no effect
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            unsafe {
                if core::mem::needs_drop::<T>() {
                    let slice = from_raw_parts_mut(self.as_mut_ptr().add(len), self.len() - len).as_mut_ptr();
                    drop_in_place(slice);
                }
            }
            self.data.size = len as u32;
        }
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest
    /// - does not check if the length of the vector is greater than `len`
    /// - please use only if you are sure that `len < self.len()`
    pub unsafe fn truncate_unchecked(&mut self, len: usize) {
        unsafe {
            if core::mem::needs_drop::<T>() {
                let slice = from_raw_parts_mut(self.as_mut_ptr(), self.len() - len).as_mut_ptr();
                drop_in_place(slice);
            }
        }
        self.data.size -= len as u32;
    }

    /// Removes and drops the element at `index`
    /// - **panics** if `index > self.len()`
    /// - **no-op** if `self.is_empty()`
    /// - preserves ordering of the vector
    ///   - this is `O(n)` operation
    pub fn remove_drop(&mut self, index: usize) {
        if self.capacity() > 0 {
            if index >= self.len() {
                #[cfg(all(feature="allocator", feature="spin", feature="string"))]
                panic_fmt!("index {index} out of bounds 0..{}", self.len());
                #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
                panic!("index is out of bounds");
            }

            unsafe {

                let ptr = self.data.data().add(index).as_ptr();

                drop_in_place(ptr);

                core::ptr::copy(ptr.add(1), ptr, self.len() - index - 1);

            }

            self.data.size -= 1;
        }
    }

    /// Returns a NonNull pointer to the vector’s buffer, **or a dangling NonNull** pointer valid for zero sized reads if the vector didn’t allocate
    pub const fn as_non_null(&self) -> NonNull<T> {
        self.data.data()
    }

    /// Removes and returns the element at `index`
    /// - **panics** if `index > self.len() || self.is_empty()`
    /// - preserves ordering of the vector
    ///   - this is `O(n)` operation
    pub fn remove(&mut self, index: usize) -> T {
        if self.capacity() > 0 {
            if index >= self.len() {
                #[cfg(all(feature="allocator", feature="spin", feature="string"))]
                panic_fmt!("index {index} out of bounds 0..{}", self.len());
                #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
                panic!("index is out of bounds");
            }

            let ret;

            self.data.size -= 1;

            unsafe {

                let ptr = self.data.data().add(index).as_ptr();

                ret = ptr.read();

                core::ptr::copy(ptr.add(1), ptr, self.len() - index);

            }

            ret

        } else {
            panic!("vector does not contain any data");
        }
    }


    /// Inserts `val` into the vector at `index` index
    /// - shifts all elements - this is `O(n)` operation
    /// - **panics** if allocation fails or `index > self.len()`
    /// - if `index == self.len()`, pushes instead
    pub fn insert(&mut self, index: usize, val: T) {

        let len = self.len();

        if index > len {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("index {index} out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("index is out of bounds");
        } else if index == len {
            self.push(val);
            return;
        }

        if len == self.capacity() {
            self.expand();
        }

        unsafe {
            let ptr = self.data.as_ptr().add(index);

            core::ptr::copy(ptr, ptr.add(1), len - index);

            ptr.write(val);
        }

        self.data.size += 1;

    }

    /// Inserts element into the vector and returns mutable reference to the inserted element
    /// - **panics** if allocation fails or if `index` is out of bounds
    pub fn insert_mut(&mut self, index: usize, val: T) -> &mut T {
        let len = self.len();

        if index > len {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("index {index} out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("index is out of bounds");
        } else if index == len {
            self.push(val);
            return unsafe { self.last_mut_unchecked() };
        }

        if len == self.capacity() {
            self.expand();
        }

        unsafe {
            let ptr = self.data.as_ptr().add(index);

            core::ptr::copy(ptr, ptr.add(1), len - index);

            self.data.size += 1;

            ptr.write(val);

            ptr.as_mut().unwrap_unchecked()
        }
    }

    /// Tries to insert `val` into the vector at `index` index
    /// - shifts all elements - this is `O(n)` operation
    /// - returns `val` if allocation fails or `index > self.len()`
    /// - if `index == self.len()`, pushes instead
    pub fn try_insert(&mut self, index: usize, val: T) -> Result<(), T> {

        let len = self.len();

        if index > len {
            return Err(val);
        } else if index == len {
            return self.try_push(val);
        }

        if len == self.capacity() {
            if let Err(_) = self.try_expand() {
                return Err(val);
            }
        }

        unsafe {
            let ptr = self.data.data().add(index).as_ptr();

            core::ptr::copy(ptr, ptr.add(1), len - index);

            ptr.write(val);
        }

        self.data.size += 1;

        Ok(())

    }


    /// Retains only the elements specified by the predicate.
    /// 
    /// In other words, remove all elements `e` for which `f(&e)` returns `false`. This method operates in place, visiting each element exactly once in the original order, and preserves the order of the retained elements
    /// - this is an `O(n)` operation
    pub fn retain<F: Fn(&T) -> bool>(&mut self, f: F) {
        let mut ptr = self.data.data();
        let mut i = 0;

        loop {

            if !f(unsafe { ptr.as_ref() }) {
                self.remove_drop(i)
            }

            if i >= self.len() {
                break;
            }

            unsafe { ptr = ptr.add(1) };
            i += 1;
        }

    }

    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        let mut ptr = self.data.data();
        let mut i = 0;

        loop {

            if !f(unsafe { ptr.as_mut() }) {
                self.remove_drop(i)
            }

            if i >= self.len() {
                break;
            }

            unsafe { ptr = ptr.add(1) };
            i += 1;
        }

    }

    /// Constructs new `Vec<T>` from slice of `T`
    /// - **panics** if allocation fails
    pub fn from_slice(slice: &[T]) -> Self
        where T: Sized + Clone {

        let mut db = DynamicBuffer::<T, STEP, ALIGN>::with_capacity(slice.len());
        db.size = slice.len() as u32;

        let mut this = db.data();

        unsafe {
            for i in slice.iter() {
                this.write(i.clone());
                this = this.add(1);
            }
        }

        Self { data: db }
    }

    /// Tries to construct `Vec<T>` from slice of `T`
    /// - returns `Err` if allocation fails of `slice[i].try_clone()` returns `Err`
    pub fn try_from_slice(slice: &[T]) -> Result<Self, ()>
        where T: Sized + TryClone {
        
        let mut db = DynamicBuffer::<T, STEP, ALIGN>::try_with_capacity(slice.len())?;
        db.size = slice.len() as u32;

        let mut this = db.data();

        unsafe {
            for (i, item) in slice.iter().enumerate() {
                let e = match item.try_clone() {
                    Ok(e) => e,
                    Err(_) => {
                        let slice = from_raw_parts_mut(db.data().as_ptr(), i).as_mut_ptr();
                        drop_in_place(slice);
                        return Err(());
                    }
                };
                this.write(e);

                this = this.add(1);
            }
        }

        Ok(Self { data: db })

    }

    /// Constructs new `Vec<T>` from slice of `U`
    /// - **panics** if allocation fails
    /// - `T` must implement `From<&U>`
    pub fn from_different_slice<'l, U>(slice: &'l [U]) -> Self
        where T: From<&'l U>, U: Sized{

        let mut db = DynamicBuffer::<T, STEP, ALIGN>::with_capacity(slice.len());
        db.size = slice.len() as u32;

        let mut this = db.data();

        unsafe {
            for i in slice.iter() {
                this.write(T::from(i));

                this = this.add(1);
            }
        }

        Self { data: db }

    }

    /// Drops the last element of the vector
    /// - **no-op** if `self.is_empty()`
    /// - does not affect `capacity`
    pub fn pop_drop(&mut self) {

        if self.len() > 0 {
            unsafe {
                drop_in_place(self.data.data().add(self.len() - 1).as_ptr());
            }
            self.data.size -= 1;
        }
    }

    /// Removes and returns the last element of the vector
    /// - **no-op** if `self.is_empty()`
    /// - does not affect `capacity`
    pub fn pop(&mut self) -> Option<T> {

        if self.len() > 0 {
            self.data.size -= 1;
            Some(unsafe { self.data.data().add(self.len()).read() })
        } else {
            None
        }
    }

    /// Removes (drops) last `n` elements of the vector
    /// - does not affect `capacity`
    /// - if `n` is greater than or equal to `self.len()`, all elements are dropped
    pub fn pop_n(&mut self, mut n: usize) {
        if self.len() > 0 && n > 0 {

            n = core::cmp::min(n, self.len());

            unsafe {
                let start = self.len() - n;

                drop_in_place(self.get_unchecked_mut(start..self.len()).as_mut_ptr());
                self.data.size -= n as u32;


                //drop_in_place();
            }
        }

        /*if self.len() > 0 {

            if n <= self.len() {
                
                unsafe {
                    drop_in_place(self.as_mut_slice_unchecked());
                }
                
                self.data.size = 0;

            } else {

                self.data.size -= n as u32;

                unsafe {
                    let ptr = self.data.as_ptr().add(self.len());
                    drop_in_place(slice::from_raw_parts_mut(ptr, n));
                }

            }
            
        }*/
    }

    /// Drops the last element if the vector if `f` returns `true`
    pub fn pop_drop_if<F>(&mut self, predicate: impl FnOnce(&mut T) -> bool)
    where F: FnOnce(&mut T) -> bool {
        let last = match self.last_mut() {
            Some(l) => l,
            None => return,
        };



        if predicate(last) {
            
            self.data.size -= 1;

            unsafe {
                drop_in_place(self.data.data().add(self.len()).as_ptr());
            }
        }
    }

    /// Removes and returns the last element if the vector if `f` returns `true`
    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let last = match self.last_mut() {
            Some(l) => l,
            None => return None,
        };



        if predicate(last) {
            
            self.data.size -= 1;

            unsafe {
                Some(self.data.data().add(self.len()).read())
            }
        } else {
            None
        }
    }

    /// Moves all elements from `other` into `self`, leaving `other` empty
    /// - **panics** if allocation fails
    pub fn append(&mut self, other: &mut Vec<T>) {
        if other.is_empty() {
            return;
        }

        self.reserve(other.len());

        unsafe {
            let mut ptr = other.data.data();
            for _ in 0..other.len() {
                self.push(ptr.read());
                ptr = ptr.add(1);
            }
            other.set_len(0);
        }
    }

    /// Append all elements from `other` to `self`, leaving `other` empty
    /// - returns `Err` if allocation fails
    pub fn try_append(&mut self, other: &mut Vec<T>) -> Result<(), ()> {
        if other.is_empty() {
            return Ok(());
        }

        self.try_reserve(other.len())?;

        unsafe {
            let mut ptr = other.data.data();
            for _ in 0..other.len() {
                self.push(ptr.read());
                ptr = ptr.add(1);
            }
            other.set_len(0);
        }

        Ok(())

    }




    /// Forces the length of the vector to `new_len`
    /// - this will not construct and/or modify `capacity`
    /// - this function does not check for any boundaries (including `capacity`)
    #[inline(always)]
    pub unsafe fn set_len(&mut self, len: usize) {
        self.data.size = len as u32;
    }

    /// Removes an element from vector and returns it
    /// - the removed element is replaced by the last element of the vector
    /// 
    /// **panics** if index is out of bounds or `self.len() == 1`
    pub fn swap_remove(&mut self, index: usize) -> T {

        if index < self.len() && self.len() > 1 {
            
            self.data.size -= 1;

            unsafe {
                let e = self.data.data().add(index);
                let removed = e.read();

                drop_in_place(e.as_ptr());

                e.write(self.data.data().add(self.len()).read());

                //e.read()
                removed

            }

        } else {
            if index >= self.len() {
                #[cfg(all(feature="allocator", feature="spin", feature="string"))]
                panic_fmt!("index {index} is out of bounds 0..{}", self.len());
                #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
                panic!("index is out of bounds");
            } else {
                panic!("calling `Vec::swap_remove` on vector with length equal to 1 could end up with undefined behaviour");
            }
        }

    }

    /// Removes an element from vector and returns it
    /// - the removed element is replaced by the last element of the vector
    /// - **does not check bounds or state of the vector**
    ///   - use only if you are sure that `index` is in bounds
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) -> T {

        self.data.size -= 1;

        unsafe {
            let e = self.data.data().add(index);

            drop_in_place(e.as_ptr());

            e.write(self.data.data().add(self.len()).read());
            e.read()
        }

    }

    /// Removes an element from vector and drops it
    /// - the removed element is replaces by `T::default()`
    /// 
    /// - **panics** if index is out of bounds
    pub fn drop_remove(&mut self, index: usize)
        where T: Default {

        if index < self.len() {
            self.data.size -= 1;
        
            unsafe {
                let e = self.data.data().add(index);

                drop_in_place(e.as_ptr());

                e.write(T::default());
            }
        } else {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("index {index} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("index is out of bounds");
        }
    }


    /// Removes an element from vector and drops it
    /// - the removed element is replaces by `T::default()`
    /// - **does not check bounds or state of the vector**
    ///   - use only if you are sure that `index` is in bounds
    pub unsafe fn drop_remove_unchecked(&mut self, index: usize)
        where T: Default {
        
        self.data.size -= 1;

        unsafe {
            let e = self.data.data().add(index);

            drop_in_place(e.as_ptr());

            e.write(T::default());
        }

    }

    /// Clears the vector, removing all values.
    /// - note that this method has no effect on the allocated `capacity`
    #[inline]
    pub fn clear(&mut self) {
        if core::mem::needs_drop::<T>() && self.len() > 0 {
            unsafe {
                drop_in_place(self.as_mut_slice_unchecked().as_mut_ptr())
            }
        }
        self.data.size = 0;
    }

    /// Consumes and leaks the `Vec`, returning mutable reference to its data
    /// - **panics** if has no data
    /// - does not shrink the `capacity`
    /// - deciding to not drop the returned reference may result in memory leak
    pub fn leak<'l>(self) -> &'l mut [T] {

        if self.capacity() > 0 {
            panic!("Vec does not contain any data");
        }

        let m = ManuallyDrop::new(self);

        unsafe { from_raw_parts_mut(m.data.data().as_ptr(), m.len()) }

    }

    /// Returns the remaining spare capacity of the vector as a slice of MaybeUninit<T>
    pub fn spare_capacity_mut(&mut self) -> Option<&mut [MaybeUninit<T>]> {
        if self.capacity() - self.len() > 0 {
            Some(unsafe { from_raw_parts_mut(self.data.data().add(self.len()).as_ptr() as *mut MaybeUninit<T>, self.capacity() - self.len()) })
        } else {
            None
        }
    }














    /// Returns number of elements in the vector
    pub const fn len(&self) -> usize { self.data.size as usize }

    /// Returns number of elements allocated by the vector
    pub const fn capacity(&self) -> usize { self.data.capacity() }

    /// Checks whether the vector is empty (`size == 0`)
    pub const fn is_empty(&self) -> bool { self.data.size == 0 }

    /// Checks if vector has any allocated data
    pub const fn has_data(&self) -> bool { self.data.has_data() }

    /// Returns the value of the generic parameter `STEP` for this instance
    pub const fn step(&self) -> usize { STEP }

    /// Returns the value of the generic parameter `ALIGN` for this instance
    pub const fn align(&self) -> usize { ALIGN }



    /// returns contents of the vector as slice
    /// - or `None` if vector does not have any contents
    pub const fn as_slice(&self) -> Option<&[T]> {
        if self.capacity() > 0 {
            Some(unsafe { from_raw_parts(self.data.data().as_ptr(), self.len()) })
        } else {
            None
        }
    }

    /// returns contents of the vector as mutable slice
    /// - or `None` if vector does not have any contents
    pub const fn as_mut_slice(&self) -> Option<&mut [T]> {
        if self.capacity() > 0 {
            Some(unsafe { from_raw_parts_mut(self.data.data().as_ptr(), self.len()) })
        } else {
            None
        }
    }


    /// returns contents of the vector as slice without checking for NULL
    /// - safety: use only if you are 1000% sure it will not be a disaster ._.
    pub const unsafe fn as_slice_unchecked(&self) -> &[T] {
        unsafe { from_raw_parts(self.data.data().as_ptr(), self.len())}
    }

    /// returns contents of the vector as mutable slice without checking for NULL
    /// - safety: use only if you are 1000% sure it will not be a disaster ._.
    pub const unsafe fn as_mut_slice_unchecked(&mut self) -> &mut [T] {
        unsafe { from_raw_parts_mut(self.data.data().as_ptr(), self.len())}
    }


    /// Checks RangeBound for this vector
    #[inline]
    fn handle_bounds<R>(&self, range: &R) -> (usize, usize)
    where R: RangeBounds<usize> {

        (match range.start_bound() {
            Excluded(&val) => val + 1,
            Included(&val) => val,
            Unbounded => 0,
        },
        match range.end_bound() {
            Included(&val) => val + 1,
            Excluded(&val) => val,
            Unbounded => self.len(),
        })
    }


}

impl<T, const STEP: usize, const N: usize, const ALIGN: usize> Vec<[T; N], STEP, ALIGN> {
    pub fn into_flattened(self) -> Vec<T, STEP> {

        let this = ManuallyDrop::new(self);

        let ptr = unsafe { NonNull::new_unchecked(this.data.as_ptr() as *mut T) };
        let cap = (this.capacity() * N) as u32;
        let size = (this.len() * N) as u32;

        Vec { data: DynamicBuffer::from_raw(ptr, cap, size) }
    }
}


impl<T: Sized, const STEP: usize, const ALIGN: usize> Vec<T, STEP, ALIGN> {

    //  Deref<[T]>

        /// Returns first element of the vector (if there are any elements)
    pub fn first(&self) -> Option<&T> {
        if self.len() > 0 {
            Some(unsafe { self.data.data().as_ref() })
        } else {
            None
        }
    }

    /// Returns firs element without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `first()` as safe alternative
    #[inline(always)]
    pub unsafe fn first_unchecked(&self) -> &T {
        unsafe { self.data.data().as_ref() }
    }

    /// Returns first element as mutable reference
    pub fn first_mut(&mut self) -> Option<&mut T> {
        if self.len() > 0 {
            Some(unsafe { self.data.data().as_mut() })
        } else {
            None
        }
    }

    /// Returns first element as mutable reference without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `first_mut()` as safe alternative
    #[inline]
    pub unsafe fn first_mut_unchecked(&mut self) -> &mut T {
        unsafe { self.data.data().as_mut() }
    }

    /// Returns the first and all the rest of the elements of the vector, or None if it is empty
    pub fn split_first(&self) -> Option<(&T, &[T])> {
        if self.len() > 0 {
            let data = self.data.data();
            Some(unsafe { (data.as_ref(), slice::from_raw_parts(data.add(1).as_ptr(), self.len() - 1))})
        } else {
            None
        }
    }

    /// Returns the first and all the rest of the elements of the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `split_first()` as safe alternative
    #[inline]
    pub unsafe fn split_first_unchecked(&self) -> (&T, &[T]) {
        unsafe {
            ( self.data.data().as_ref(),
            slice::from_raw_parts(self.data.data().add(1).as_ptr(), self.len() - 1) )
        }
    }

    /// Returns the first and all the rest of the elements of the vector, or None if it is empty
    pub fn split_first_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        if self.len() > 0 {
            let mut data = self.data.data();
            Some(unsafe { (data.as_mut(), slice::from_raw_parts_mut(data.add(1).as_ptr(), self.len() - 1))})
        } else {
            None
        }
    }

    /// Returns the first and all the rest of the elements of the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `split_first_mut()` as safe alternative
    #[inline]
    pub unsafe fn split_first_mut_unchecked(&mut self) -> (&mut T, &mut [T]) {
        unsafe {
            ( self.data.data().as_mut(),
            slice::from_raw_parts_mut(self.data.data().add(1).as_ptr(), self.len() - 1) )
        }
    }

    /// Returns the last and all the rest of te elements of the vector
    pub fn split_last(&self) -> Option<(&T, &[T])> {
        if self.len() > 0 {
            let data = self.data.data();
            Some(unsafe { (data.add(self.len() - 1).as_ref(),
                slice::from_raw_parts(data.as_ptr(), self.len() - 1)) })
        } else {
            None
        }
    }

    /// Returns the last and all the rest of te elements of the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `split_last()` as safe alternative
    #[inline]
    pub unsafe fn split_last_unchecked(&self) -> (&T, &[T]) {
        unsafe {
            let data = self.data.data();
            (data.add(self.len() - 1).as_ref(), slice::from_raw_parts(data.as_ptr(), self.len() - 1))
        }
    }

    /// Returns the last and all the rest of te elements of the vector
    pub fn split_last_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        if self.len() > 0 {
            let data = self.data.data();
            Some(unsafe { (data.add(self.len() - 1).as_mut(),
                slice::from_raw_parts_mut(data.as_ptr(), self.len() - 1)) })
        } else {
            None
        }
    }

    /// Returns the last and all the rest of te elements of the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `split_last_mut()` as safe alternative
    #[inline]
    pub unsafe fn split_last_mut_unchecked(&mut self) -> (&mut T, &mut [T]) {
        unsafe {
            let data = self.data.data();
            (data.add(self.len() - 1).as_mut(), slice::from_raw_parts_mut(data.as_ptr(), self.len() - 1))
        }
    }

    /// Returns the last element of the slice, or None if it is empty
    pub fn last(&self) -> Option<&T> {
        if self.len() > 0 {
            Some(unsafe { self.data.data().add(self.len() - 1).as_ref() })
        } else {
            None
        }
    }
    /// Returns the last element of the slice without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `last()` as safe alternative
    #[inline]
    pub unsafe fn last_unchecked(&self) -> &T {
        unsafe { self.data.data().add(self.len() - 1).as_ref() }
    }

    /// Returns the last element of the slice, or None if it is empty
    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.len() > 0 {
            Some(unsafe { self.data.data().add(self.len() - 1).as_mut() })
        } else {
            None
        }
    }

    /// Returns the last element of the slice without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `last_mut()` as safe alternative
    #[inline]
    pub unsafe fn last_mut_unchecked(&mut self) -> &mut T {
        unsafe { self.data.data().add(self.len() - 1).as_mut() }
    }


    /// Returns an array reference to the first `N` items in the vector
    pub fn first_chunk<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() >= N {
            Some(unsafe { &*(self.as_ptr().cast::<[T; N]>()) })
        } else {
            None
        }
    }

    /// Returns an array reference to the first `N` items in the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `first_chunk()` as safe alternative
    #[inline]
    pub unsafe fn first_chunk_unchecked<const N: usize>(&self) -> &[T; N] {
        unsafe { &*(self.as_ptr().cast::<[T; N]>()) }
    }

    /// Returns an array reference to the first `N` items in the vector
    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]> {
        if self.len() >= N {
            Some(unsafe { &mut *(self.as_mut_ptr().cast::<[T; N]>()) })
        } else {
            None
        }
    }

    /// Returns an array reference to the first `N` items in the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `first_chunk()` as safe alternative
    #[inline]
    pub unsafe fn first_chunk_mut_unchecked<const N: usize>(&mut self) -> &mut [T; N] {
        unsafe { &mut *(self.as_mut_ptr().cast::<[T; N]>()) }
    }


    /// Returns an array reference to the last `N` items in the slice
    pub fn last_chunk<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() > N {
            Some(unsafe { &*(self.as_ptr().add(self.len() - N).cast::<[T; N]>()) })
        } else {
            None
        }
    }

    /// Returns an array reference to the last `N` items in the slice without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `last_chunk()` as safe alternative
    #[inline]
    pub unsafe fn last_chunk_unchecked<const N: usize>(&self) -> &[T; N] {
        unsafe { &*(self.as_ptr().add(self.len() - N).cast::<[T; N]>()) }
    }

    /// Returns an array reference to the last `N` items in the slice
    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]> {
        if self.len() > N {
            Some(unsafe { &mut *(self.as_mut_ptr().add(self.len() - N).cast::<[T; N]>()) })
        } else {
            None
        }
    }

    /// Returns an array reference to the last `N` items in the slice without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `last_chunk_mut()` as safe alternative
    #[inline]
    pub unsafe fn last_chunk_mut_unchecked<const N: usize>(&mut self) -> &mut [T; N] {
        unsafe { &mut *(self.as_mut_ptr().add(self.len() - N).cast::<[T; N]>()) }
    }

    /// Returns an subslice from the vector
    /// - or `None` if vector has no data or out of bounds
    pub fn get<R>(&self, range: R) -> Option<&[T]>
    where R: RangeBounds<usize> {
        let (start, end) = self.handle_bounds(&range);

        if start > self.len() || end > self.len() {
            return None;
        }

        Some(unsafe { from_raw_parts(self.data.data().add(start).as_ptr(), end - start) })
        
    }

    /// Returns an mutable subslice from the vector
    /// - or `None` if vector has no data or out of bounds
    pub fn get_mut<R>(&self, range: R) -> Option<&mut [T]>
    where R: RangeBounds<usize> {
        let (start, end) = self.handle_bounds(&range);

        if start > self.len() || end > self.len() {
            return None;
        }

        Some(unsafe { from_raw_parts_mut(self.data.data().add(start).as_ptr(), end - start) })
    }

    /// Returns an sublice from the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `get()` as safe alternative
    pub unsafe fn get_unchecked<R>(&self, range: R) -> &[T]
    where R: RangeBounds<usize> {

        let (start, end) = self.handle_bounds(&range);
        unsafe {
            from_raw_parts(self.data.data().add(start).as_ptr(), end - start)
        }
    }

    /// Returns an mutable sublice from the vector without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `get_mut()` as safe alternative
    pub unsafe fn get_unchecked_mut<R>(&mut self, range: R) -> &mut [T]
    where R: RangeBounds<usize> {
        let (start, end) = self.handle_bounds(&range);
        unsafe {
            from_raw_parts_mut(self.data.data().add(start).as_ptr(), end - start)
        }
    }


    /// Returns pointer to data of this vector
    /// 
    /// # IMPORTANT
    /// Returned pointer will not be null even if no data is allocated
    #[inline(always)]
    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Returns mutable pointer to data of this vector
    /// 
    /// # IMPORTANT
    /// Returned pointer will not be null even if no data is allocated
    #[inline(always)]
    pub const fn as_mut_ptr(&self) -> *mut T {
        self.data.as_ptr()
    }

    /// Returns the two raw pointers spanning the slice
    /// 
    /// The returned range is half-open, which means that the end pointer points one past the last element of the slice. This way, an empty slice is represented by two equal pointers, and the difference between the two pointers represents the size of the slice
    /// 
    /// See `as_ptr` for warnings on using these pointers. The end pointer requires extra caution, as it does not point to a valid element in the slice.
    /// 
    /// This function is useful for interacting with foreign interfaces which use two pointers to refer to a range of elements in memory, as is common in C++.
    /// 
    /// **note** - bounds are not check in this implementation
    #[inline]
    pub const fn as_ptr_range(&self) -> Range<*const T> {
        Range { start: self.data.as_ptr(), end: unsafe { self.data.as_ptr().add(self.len()) } }
    }

    /// Returns the two raw pointers spanning the slice
    /// 
    /// The returned range is half-open, which means that the end pointer points one past the last element of the slice. This way, an empty slice is represented by two equal pointers, and the difference between the two pointers represents the size of the slice
    /// 
    /// See `as_ptr` for warnings on using these pointers. The end pointer requires extra caution, as it does not point to a valid element in the slice.
    /// 
    /// This function is useful for interacting with foreign interfaces which use two pointers to refer to a range of elements in memory, as is common in C++.
    /// 
    /// **note** - bounds are not check in this implementation
    #[inline]
    pub const fn as_mut_ptr_range(&mut self) -> Range<*mut T> {
        let ptr = self.data.as_ptr();
        Range { start: ptr, end: unsafe { ptr.add(self.len()) } }
    }

    /// Gets an reference to underlying array
    pub fn as_array<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() > N {
            Some(unsafe { &*(self.as_ptr().cast()) })
        } else {
            None
        }
    }

    /// Gets an reference to underlying array without checking bounds
    /// /// - can possibly cause **address boundary errors**
    /// - please use `as_array()` as safe alternative
    #[inline]
    pub unsafe fn as_array_unchecked<const N: usize>(&self) -> &[T; N] {
        unsafe { &*(self.as_ptr().cast()) }
    }

    /// Gets an mutable reference to underlying array
    pub fn as_mut_array<const N: usize>(&self) -> Option<&mut [T; N]> {
        if self.len() > N {
            Some(unsafe { &mut *(self.as_mut_ptr().cast()) })
        } else {
            None
        }
    }

    /// Gets an mutable reference to underlying array  without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `as_mut_array()` as safe alternative
    pub unsafe fn as_mut_array_unchecked<const N: usize>(&mut self) -> &mut [T; N] {
        unsafe { &mut *(self.as_mut_ptr().cast()) }
    }

    /// Swaps elements at index `a` and `b`
    /// - **panics** if out of bounds
    pub fn swap(&mut self, a: usize, b: usize) {

        if a >= self.len() || b >= self.len() {
            //  give user an ide where is problem
            let check = (a >= self.len()) as usize | ((b >= self.len()) as usize) << 1;
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            match check {
                0b01 => panic_fmt!("argument a = {a} is out of bounds [0..{}]", self.len()),
                0b10 => panic_fmt!("argument b = {b} is out of bounds [0..{}]", self.len()),
                0b11 => panic_fmt!("arguments a = {a} and b = {b} are out of bounds [0..{}]", self.len()),
                _ => unsafe { unreachable_unchecked()},
            }
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            match check {
                0b01 => panic!("argument a is out of bounds"),
                0b10 => panic!("argument b is out of bounds"),
                0b11 => panic!("arguments a and b are out of bounds"),
                _ => unsafe { unreachable_unchecked() }
            }
        }
        if a == b { return } 

        unsafe {
            let a = self.data.data().add(a);

            let b = self.data.data().add(b);

            let tmp = a.read();
            a.write(b.read());
            b.write(tmp);
        }

    }

    /// Swaps elemetns at index `a` and `b` without checking bounds
    /// - can possibly cause **address boundary errors**
    /// - please use `swap()` as safe alternative
    pub unsafe fn swap_unchecked(&mut self, a: usize, b: usize) {
        unsafe {
            let a = self.data.data().add(a);

            let b = self.data.data().add(b);

            let tmp = a.read();
            a.write(b.read());
            b.write(tmp);
        }
    }

    /// Reverses the order of elements in the vector, in place
    /// - this is an `O(n)` operation
    pub fn reverse(&mut self) {

        if self.len() > 1 {

            let mut end = self.len() - 1;

            for i in 0..self.len()/2 {
                unsafe { self.swap_unchecked(i, end) };
                end -= 1;
            }

        }

    }

    /// Returns iterator for this vector
    /// - checks whether vector is empty or not
    #[inline]
    pub fn iter<'l>(&'l self) -> core::slice::Iter<'l, T> {
        self.as_slice().expect("Vec has no data").iter()
    }

    /// Returns iterator for this vector
    /// - does not check if vector is empty
    ///   - can possibly cause **address boundary errors**
    /// - please use `iter()` as safe alternative
    #[inline(always)]
    pub unsafe fn iter_unchecked<'l>(&'l self) -> core::slice::Iter<'l, T> {
        unsafe { self.as_slice_unchecked() }.iter()
    }

    /// Returns mutable iterator for this vector
    /// - checks whether vector is empty or not
    #[inline]
    pub fn iter_mut<'l>(&'l mut self) -> core::slice::IterMut<'l, T> {
        self.as_mut_slice().expect("Vec has no data").iter_mut()
    }

    /// Returns mutable iterator for this vector
    /// - does not check if vector is empty
    ///   - can possibly cause **address boundary errors**
    /// - please use `iter_mut()` as safe alternative
    #[inline(always)]
    pub unsafe fn iter_mut_unchecked<'l>(&'l mut self) -> core::slice::IterMut<'l, T> {
        unsafe { self.as_mut_slice_unchecked() }.iter_mut()
    }

    /// Creates a `Vec<T>` directly from a pointer, a length and a capacity
    /// 
    /// This is **highly unsafe**, due to the number of invariants that aren’t checked:
    /// - `ptr` must be allocated via the `ministd::ALLOCATOR` allocator
    ///   - with `Vec::layout_for()` or `Vec::layout_for_exact()` used for layout description
    /// - `size` must be less than or equal to `capacity`
    ///   - The first `size` values must be properly initialized values of type `T`
    /// - `capacity` needs to fit the layout size that the pointer was allocated with
    pub unsafe fn from_raw_parts(ptr: *mut T, size: usize, capacity: usize,) -> Self {
        Self {
            data: DynamicBuffer::from_raw(unsafe { NonNull::new_unchecked(ptr) }, capacity as u32, size as u32)
        }
    }

    /// Creates a `Vec<T>` directly from a pointer, a length and a capacity
    /// 
    /// This is less unsafe variant of the `from_raw_parts` function, however it is not completely safe:
    /// - `ptr` is checked to be non-null and well aligned
    ///   - must be allocated via the `ministd::ALLOCATOR` allocator
    ///   - with `Vec::layout_for()` or `Vec::layout_for_exact()` used for layout description
    /// - `size` is checked to be less than or equal to `capacity`
    /// - `capacity` needs to fit the layout size that the pointer was allocated with
    pub unsafe fn from_raw_parts_checked(ptr: *mut T, size: usize, capacity: usize) -> Result<Self, ()> {
        Ok(Self {
            data: DynamicBuffer::from_raw(NonNull::new(ptr).ok_or(())?,
            capacity as u32, if size <= capacity {
                size as u32
            } else {
                return Err(())
            })
        })
    }

    /// Creates a `Vec<T>` directly from a pointer, a length and a capacity
    /// 
    /// This is **highly unsafe**, due to the number of invariants that aren’t checked:
    /// - `ptr` must be allocated via the `ministd::ALLOCATOR` allocator
    ///   - with `Vec::layout_for()` or `Vec::layout_for_exact()` used for layout description
    /// - `size` must be less than or equal to `capacity`
    ///   - The first `size` values must be properly initialized values of type `T`
    /// - `capacity` needs to fit the layout size that the pointer was allocated with
    pub const unsafe fn from_parts(ptr: NonNull<T>, size: usize, capacity: usize) -> Self {
        Self {
            data: DynamicBuffer::from_raw(ptr, capacity as u32, size as u32)
        }
    }

    /// Creates a `Vec<T>` directly from a pointer, a length and a capacity
    /// 
    /// This is less unsafe variant of the `from_parts` function, however it is not completely safe:
    /// - `ptr` is checked to be non-null and well aligned
    ///   - must be allocated via the `ministd::ALLOCATOR` allocator
    ///   - with `Vec::layout_for()` or `Vec::layout_for_exact()` used for layout description
    /// - `size` is checked to be less than or equal to `capacity`
    /// - `capacity` needs to fit the layout size that the pointer was allocated with
    pub unsafe fn from_parts_checked(ptr: NonNull<T>, size: usize, capacity: usize) -> Result<Self, ()> {
        Ok(Self {
            data: DynamicBuffer::from_raw(ptr, capacity as u32,
            if size <= capacity {
                size as u32
            } else {
                return Err(())
            })
        })
    }

    /// Decomposes a `Vec<T>` into its raw components: `(pointer, length, capacity)`
    #[inline]
    pub unsafe fn into_raw_parts(self) -> (*mut T, usize, usize) {
        let m = ManuallyDrop::new(self);
        (m.as_mut_ptr(), m.len(), m.capacity())
    }

    /// Decomposes a `Vec<T>` into its raw components: `(NonNull pointer, length, capacity)`
    #[inline]
    pub unsafe fn into_parts(self) -> (NonNull<T>, usize, usize) {
        let m = ManuallyDrop::new(self);
        (m.data.data(), m.len(), m.capacity())
    }

    
    /*pub(crate) unsafe fn into_dynamic_buffer(self) -> DynamicBuffer<T, STEP, ALIGN> {
        unsafe {
            let (ptr, size, cap) = self.into_parts();
            DynamicBuffer::from_raw(ptr, cap as u32, size as u32)
        }

    }


    pub(crate) const unsafe fn from_dynamic_buffer(db: DynamicBuffer<T, STEP, ALIGN>) -> Self {
        Self { data: db }
    }*/



}



impl<T: Sized, const STEP: usize, const ALIGN: usize> AsRef<[T]> for Vec<T, STEP, ALIGN> {
    /// **panics** if has no data
    fn as_ref(&self) -> &[T] {
        self.as_slice().expect("Vec has no data")
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> AsMut<[T]> for Vec<T, STEP, ALIGN> {
    /// **panics** if has no data
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice().expect("Vec has no data")
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> AsRef<Vec<T, STEP, ALIGN>> for Vec<T, STEP, ALIGN> {
    fn as_ref(&self) -> &Vec<T, STEP, ALIGN> {
        &self
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> AsMut<Vec<T, STEP, ALIGN>> for Vec<T, STEP, ALIGN> {
    fn as_mut(&mut self) -> &mut Vec<T, STEP, ALIGN> {
        self
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> Borrow<[T]> for Vec<T, STEP, ALIGN> {
    /// **panics** if has no data
    fn borrow(&self) -> &[T] {
        self.as_slice().expect("Vec has no data")
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> BorrowMut<[T]> for Vec<T, STEP, ALIGN> {
    /// **panics** if has no data
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice().expect("Vec has no data")
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> Drop for Vec<T, STEP, ALIGN> {
    fn drop(&mut self) {
        if self.capacity() > 0 {
            unsafe {
                drop_in_place(from_raw_parts_mut(self.data.data().as_ptr(), self.len()));
            }
        }
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> Index<usize> for Vec<T, STEP, ALIGN> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        if index < self.len() {
            unsafe {
                return self.data.data().add(index).as_ref();
            }
        }

        if self.len() == 0 {
            panic!("vector has no data");
        } else {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("Vec[]: index {index} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("Vec[]: index is out of bounds");
        }
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> IndexMut<usize> for Vec<T, STEP, ALIGN> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < self.len() {
            unsafe {
                return self.data.data().add(index).as_mut();
            }
        }

        if self.len() == 0 {
            panic!("vector has no data");
        } else {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("Vec[]: index {index} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("Vec[]: index is out of bounds");
        }
    }
}

impl<T: Sized + Clone, const STEP: usize, const ALIGN: usize> Clone for Vec<T, STEP, ALIGN> {
    fn clone(&self) -> Self {

        let db = self.data.clone();

        let new_slice = unsafe { from_raw_parts_mut(db.as_ptr(), self.len()) };
        let old_slice = unsafe { from_raw_parts(self.data.as_ptr(), self.len()) };

        for (i, item) in new_slice.iter_mut().enumerate() {
            *item = old_slice[i].clone();
        }

        Self {
            data: db,
        }

    }
}

impl<T: Sized + TryClone, const STEP: usize, const ALIGN: usize> TryClone for Vec<T, STEP, ALIGN> {
    type Error = ();

    fn try_clone(&self) -> Result<Self, Self::Error>
    where Self: Sized, Self::Error: Default {
        
        let db = self.data.try_clone()?;

        let new_slice = unsafe { from_raw_parts_mut(db.as_ptr(), self.len()) };
        let old_slice = unsafe { from_raw_parts(self.data.as_ptr(), self.len()) };

        //  copy all elements (DynamicBuffer does not do that)
        for (i, item) in new_slice.iter_mut().enumerate() {
            *item = match old_slice[i].try_clone() {
                Ok(i) => i,
                Err(_) => {
                    unsafe { drop_in_place(new_slice[0..i].as_mut_ptr()) }
                    return Err(());
                },
            };
        }

        Ok(Self {
            data: db,
        })

    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> Deref for Vec<T, STEP, ALIGN> {
    type Target = [T];
    #[inline]
    /// Does not check for null at all
    fn deref(&self) -> &Self::Target {
        unsafe { self.as_slice_unchecked() }
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> DerefMut for Vec<T, STEP, ALIGN> {
    #[inline]
    /// Does not check for null at all
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.as_mut_slice_unchecked() }
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> Default for Vec<T, STEP, ALIGN> {
    #[inline(always)]
    /// Equivalent of `Vec::new()`
    fn default() -> Self {
        Self::new()
    }
}


impl<T: Sized + Debug, const STEP: usize, const ALIGN: usize> Debug for Vec<T, STEP, ALIGN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {

        if f.alternate() {
            write!(f, "Vec( ptr: {:p}, len: {}, capacity: {} )", self.as_ptr(), self.len(), self.capacity())
        } else {
            write!(f, "{:?}", if let Some(slc) = self.as_slice() { slc } else { &[] })
        }
    }
}



impl<T, U, const SSTEP: usize, const OSTEP: usize, const SALIGN: usize, const OALIGN: usize>
    PartialEq<Vec::<U, OSTEP, OALIGN>> for Vec<T, SSTEP, SALIGN>
    where T: Sized + PartialEq<U> {

    fn eq(&self, other: &Vec::<U, OSTEP, OALIGN>) -> bool {

        match self.is_empty() as usize | ((other.is_empty() as usize) << 1) {
            0b00 => {   //  both have any data
                unsafe { self.as_slice_unchecked() == other.as_slice_unchecked() }
            },
            0b01 => {   //  only `self` has data
                false
            },
            0b10 => {   //  only `other` has data
                false
            },
            0b11 => {   //  `self` and `other` has no data
                true
            },
            _ => unsafe {   //  only 2 bits are used
                unreachable_unchecked()
            }
        }
    }

    fn ne(&self, other: &Vec::<U, OSTEP, OALIGN>) -> bool {
        
        match self.is_empty() as usize | ((other.is_empty() as usize) << 1) {
            0b00 => {   //  both have any data
                unsafe { self.as_slice_unchecked() != other.as_slice_unchecked() }
            },
            0b01 => {   //  only `self` has data
                true
            },
            0b10 => {   //  only `other` has data
                true
            },
            0b11 => {   //  `self` and `other` has no data
                false
            },
            _ => unsafe {   //  only 2 bits are used
                unreachable_unchecked()
            }
        }

    }

}

impl<T, U, const STEP: usize, const ALIGN: usize> PartialEq<[U]> for Vec<T, STEP, ALIGN>
    where T: Sized + PartialEq<U>, U: Sized {

    fn eq(&self, other: &[U]) -> bool {

        if self.len() == other.len() {
            if let Some(slice) = self.as_slice() {
                slice == other
            } else {
                self.len() == 0
            }
        } else {
            false
        }

    }

    fn ne(&self, other: &[U]) -> bool {
        
        if self.len() == other.len() && self.len() != 0 {
            unsafe { self.as_slice_unchecked() != other }
        } else {
            true
        }

    }
}

impl<T, U, const STEP: usize, const N: usize, const ALIGN: usize> PartialEq<[U; N]> for Vec<T, STEP, ALIGN>
    where T: Sized + PartialEq<U>, U: Sized {

    fn eq(&self, other: &[U; N]) -> bool {
        if self.len() == N {
            unsafe { self.as_array_unchecked::<N>() == other }
        } else {
            false
        }
    }

    fn ne(&self, other: &[U; N]) -> bool {
        if self.len() == N {
            unsafe { self.as_array_unchecked() != other }
        } else {
            true
        }
    }
}

impl<'l, T, const STEP: usize, const ALIGN: usize> From<&'l [T]> for Vec<T, STEP, ALIGN>
    where T: Sized + Clone {
    fn from(value: &'l [T]) -> Self {

        let mut db = DynamicBuffer::<T, STEP, ALIGN>::with_capacity(value.len());
        db.size = value.len() as u32;

        let mut this = db.data();

        unsafe {
            for i in value.iter() {
                this.write(i.clone());

                this = this.add(1);
            }
        }

        Vec { data: db }
    }
}

impl<'l, T, const STEP: usize, const N: usize, const ALIGN: usize> From<&'l [T; N]> for Vec<T, STEP, ALIGN>
    where T: Sized + Clone {
    fn from(value: &'l [T; N]) -> Self {
        let mut db = DynamicBuffer::<T, STEP, ALIGN>::with_capacity(N);
        db.size = N as u32;

        let mut this = db.data();

        unsafe {
            for i in value.iter() {
                this.write(i.clone());

                this = this.add(1);
            }
        }

        Vec { data: db }
    }
}

impl<T: Sized, const STEP: usize, const N: usize, const ALIGN: usize> From<[T; N]> for Vec<T, STEP, ALIGN>
    where T: Sized + Clone {
    fn from(value: [T; N]) -> Self {
        let mut db = DynamicBuffer::<T, STEP, ALIGN>::with_capacity(N);
        db.size = N as u32;

        let mut this = db.data();

        unsafe {
            for i in value.iter() {
                this.write(i.clone());

                this = this.add(1);
            }
        }

        Vec { data: db }
    }
}

impl<const STEP: usize, const ALIGN: usize> From<&str> for Vec<u8, STEP, ALIGN> {
    fn from(value: &str) -> Self {

        let mut db = DynamicBuffer::<u8, STEP, ALIGN>::with_capacity(value.len());
        db.size = value.len() as u32;

        unsafe {
            copy_nonoverlapping(value.as_ptr(), db.data().as_ptr(), value.len());
        }

        Vec { data: db }

    }
}
#[cfg(all(feature="allocator", feature="spin", feature="box"))]
impl<T: Sized, const STEP: usize, const ALIGN: usize> From<Box<T>> for Vec<T, STEP, ALIGN> {
    fn from(value: Box<T>) -> Self {
        let m = ManuallyDrop::new(value);
        Self {
            data: DynamicBuffer::from_raw(m.as_non_null(), 1, 1)
        }
    }
}

impl<const STEP: usize, const ALIGN: usize> From<&CStr> for Vec<u8, STEP, ALIGN> {
    /// Copies the string content into a Vec
    fn from(value: &CStr) -> Self {
        let len = value.count_bytes();
        let mut db = DynamicBuffer::<u8, STEP, ALIGN>::with_capacity(len);
        db.size = len as u32;

        unsafe {
            copy_nonoverlapping(value.as_ptr(), db.as_ptr() as *mut i8, len);
        }

        Self { data: db }
    }
}

impl<T, const STEP: usize, const ALIGN: usize> Hash for Vec<T, STEP, ALIGN>
    where T: Sized + Hash {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        Hash::hash_slice(self.as_slice().expect("vector is empty"), state);
    }
}

//impl<T, const STEP: usize, const ALIGN>


impl<'a, T: Clone> From<&'a Vec<T>> for Cow<'a, [T]> {
    fn from(v: &'a Vec<T>) -> Cow<'a, [T]> {
        Cow::Borrowed(v.as_slice().expect("Vec is empty"))
    }
}

impl<'a, T> From<Cow<'a, [T]>> for Vec<T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    #[track_caller]
    fn from(s: Cow<'a, [T]>) -> Vec<T> {
        s.into_owned()
    }
}

impl<'a, T: Clone> From<Vec<T>> for Cow<'a, [T]> {
    fn from(v: Vec<T>) -> Cow<'a, [T]> {
        Cow::Owned(v)
    }
}



impl<'l, T: Sized + Clone, const STEP: usize, const ALIGN: usize> FromIterator<&'l T> for Vec<T, STEP, ALIGN> {
    fn from_iter<I: IntoIterator<Item = &'l T>>(iter: I) -> Self {
        let iter = iter.into_iter();

        let (lower, upper) = iter.size_hint();

        let mut vec;

        if let Some(upper) = upper {
            //  the exact element count is `upper`
            vec = Vec::with_capacity(upper);

            for item in iter {
                unsafe { vec.push_within_capacity_unchecked(item.clone()); }
            }
        } else {
            //  the exact count of elements is not known (should be at least `lower`)
            vec = Vec::with_capacity(lower);

            for item in iter {
                vec.push(item.clone());
            }
        }

        vec
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> FromIterator<T> for Vec<T, STEP, ALIGN> {
    
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();

        let (lower, upper) = iter.size_hint();

        let mut vec;

        if let Some(upper) = upper {
            //  the exact element count is `upper`
            vec = Vec::with_capacity(upper);

            for item in iter {
                unsafe { vec.push_within_capacity_unchecked(item); }
            }
        } else {
            //  the exact count of elements is not known (should be at least `lower`)
            vec = Vec::with_capacity(lower);

            for item in iter {
                vec.push(item);
            }
        }

        vec
    }
}


/// Creates a `Vec` containing the arguments
/// 
/// usage:
/// ```
/// //  empty `Vec<usize>` with the `STEP` generic set to default
/// let vec: Vec<usize> = vec!();
///
/// //  empty `Vec<usize>` with `STEP = 1`
/// let vec: Vec<usize, 1> vec!(1);
///
/// //  `Vec<usize>` with 4 elements set to `1` and `STEP` generic set to default
/// let vec = vec!(1usize; 4);
///
/// //  `Vec<usize>` with 4 elements set to `1` and `STEP = 1`
/// let vec = vec!(1; 1usize; 4);
/// 
/// //  `Vec<usize>` with 4 elements set to `0`, `1`, `2` and `3` and `STEP` generic set to default
/// let vec = vec![0usize, 1, 2, 3];
/// 
/// //  `Vec<usize>` with 4 elements set to `0`, `1`, `2` and `3` and `STEP = 1`
/// let vec = vec![1; 0usize, 1, 2, 3, 4];
/// ```
#[macro_export]
macro_rules! vec {
    () => (
        $crate::Vec::vec_new()
    );
    ($step:expr; ) => {
        $crate::Vec::vec_new_with_step::<$step>()
    };
    ($elem:expr; $n:expr) => (
        $crate::Vec::from_elem::<0>($elem, $n)
    );
    ($step:expr; $elem:expr; $n:expr) => {
        $crate::Vec::from_elem::<$step>($elem, $n)
    };
    ($($x:expr),+ $(,)?) => (
        $crate::Array::from([$($x),+]).into_vec::<0>()
    );
    [$step:expr; $($x:expr),+ $(,)?] => {
        $crate::Array::from([$($x),+]).into_vec::<$step>()
    };
    [$x:expr] => {
        $crate::Array::from([$($x),+]).into_vec::<0>()
    };
}
