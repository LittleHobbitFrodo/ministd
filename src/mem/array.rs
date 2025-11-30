//	mem/array/mod.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build


//! The `Box` for arrays and slices - the `ministd::Box` cannot yet allocate arrays and slices

use core::alloc::Layout;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::{drop_in_place, NonNull};
use core::alloc::GlobalAlloc;
use crate::mem::alloc::ALLOCATOR;
use crate::TryClone;
use core::ops::{Bound::*, Index, IndexMut, RangeBounds};
use core::slice;
use core::ops::{Deref, DerefMut};

#[cfg(all(feature="allocator", feature="vector"))]
use crate::Vec;



pub const fn uninit<T: Sized, const LEN: usize>() -> [T; LEN] {
    unsafe { MaybeUninit::uninit().assume_init() }
}

/// Array is Box-like structure used to allocate arrays
#[repr(C)]
pub struct Array<T: Sized> {
    data: NonNull<T>,
    size: usize,
}

impl<T: Sized> Array<T> {

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

    /// Describes layout for Array
    pub const fn layout(size: usize) -> Layout {
        unsafe { Layout::from_size_align_unchecked(size_of::<T>() * size, align_of::<T>()) }
    }

    /// Allocates array on the heap and sets all values to `f()`
    /// - **panics** if allocation fails
    pub fn new_with<F>(f: F, size: usize) -> Self
    where F: Fn(usize) -> T {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(size))
        } as *mut T;

        assert!(!data.is_null(), "failed to allocate memory for Array");

        for (i, item) in unsafe { slice::from_raw_parts_mut(data, size) }.iter_mut().enumerate() {
            *item = f(i);
        }

        Self {
            data: unsafe { NonNull::new_unchecked(data) },
            size: size,
        }
    }

    /// Allocates array on the heap and sets all values to `f()`
    /// - returns `Err` if allocation fails
    pub fn try_new_with<F>(f: F, size: usize) -> Result<Self, ()>
    where F: Fn(usize) -> T {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(size))
        } as *mut T;

        if data.is_null() {
            return Err(());
        }

        for (i, item) in unsafe { slice::from_raw_parts_mut(data, size) }.iter_mut().enumerate() {
            *item = f(i);
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(data) },
            size,
        })
    }


    /// Allocates array on the heap and checks for values returned by `f()`
    /// - **panics** if allocation or `f()` fails
    pub fn new_with_checked<F, E>(f: F, size: usize) -> Self
    where F: Fn(usize) -> Result<T, E> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(size))
        } as *mut T;

        assert!(!data.is_null(), "failed to allocate data for Array");

        let slice = unsafe { slice::from_raw_parts_mut(data, size) };

        for (i, item) in slice.iter_mut().enumerate() {
            *item = match f(i) {
                Ok(i) => i,
                Err(_) => panic!("failed to create instance of T"),
            };
        }

        Self {
            data: unsafe { NonNull::new_unchecked(data) },
            size,
        }
    }


    /// Tries to allocate array on the heap and checks for values returned by `f()`
    /// - return `Err` if allocation or `f()` fails
    pub fn try_new_with_checked<F, E: Default>(f: F, size: usize) -> Result<Self, E>
    where F: Fn(usize) -> Result<T, E> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(size))
        } as *mut T;

        if data.is_null() {
            return Err(E::default());
        }

        let slice = unsafe { slice::from_raw_parts_mut(data, size) };

        for (i, item) in slice.iter_mut().enumerate() {
            *item = f(i)?;
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(data) },
            size,
        })

    }


    /// Allocates array on heap, returning it unitialized
    /// - **panics** if allocation fails
    pub fn new_uninit(size: usize) -> Array<MaybeUninit<T>> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(size))
        } as *mut MaybeUninit<T>;

        assert!(!data.is_null(), "failed to allocate memory for Array");

        Array {
            data: unsafe { NonNull::new_unchecked(data) },
            size,
        }
    }

    /// Tries to allocate array on heap while returning it uninitialized
    /// - returns `Err` if allocation fails
    pub fn try_new_uninit(size: usize) -> Result<Array<MaybeUninit<T>>, ()> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(size))
        } as *mut MaybeUninit<T>;

        if data.is_null() {
            return Err(());
        }

        Ok(Array {
            data: unsafe { NonNull::new_unchecked(data) },
            size,
        })
    }

    /// Allocates array on heap while forcing all bytes to 0
    /// - **panics** if allocation fails
    pub fn new_zeroed(size: usize) -> Array<MaybeUninit<T>> {
        let data = unsafe {
            ALLOCATOR.alloc_zeroed(Self::layout(size))
        } as *mut MaybeUninit<T>;

        assert!(!data.is_null(), "failed to allocate memory for Array");

        Array {
            data: unsafe { NonNull::new_unchecked(data) },
            size,
        }

    }

    /// Tries to allocate array on heap while forcing all bytes to 0
    /// - returns `Err` if allocation fails
    pub fn try_new_zeroed(size: usize) -> Result<Array<MaybeUninit<T>>, ()> {
        let data = unsafe {
            ALLOCATOR.alloc_zeroed(Self::layout(size))
        } as *mut MaybeUninit<T>;

        if data.is_null() {
            return Err(());
        }

        Ok(Array {
            data: unsafe { NonNull::new_unchecked(data) },
            size,
        })
    }

    /// Returns iterator to Array
    #[inline(always)]
    pub fn iter<'l>(&'l self) -> core::slice::Iter<'l, T> {
        self.as_slice().iter()
    }

    /// Returns mutable iterator to Array
    #[inline(always)]
    pub fn iter_mut<'l>(&'l mut self) -> core::slice::IterMut<'l, T> {
        self.as_mut_slice().iter_mut()
    }


}

impl<T: Sized> Array<MaybeUninit<T>> {

    /// Tells the compiler that data are initialized
    #[inline]
    pub unsafe fn assume_init(self) -> Array<T> {
        let m = ManuallyDrop::new(self);
        Array {
            data: unsafe { NonNull::new_unchecked(m.data.as_ptr() as *mut T) },
            size: m.size,
        }
    }

}

impl<T: Sized> Array<T> {

    /// Returns number of Elements in the array
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.size
    }

    /// Returns element at some index or `None`
    pub fn at(&self, index: usize) -> Option<&T> {
        if index < self.len() {
            Some(unsafe { self.data.add(index).as_ref() })
        } else {
            None
        }
    }

    /// Returns element at some index as mutable reference or `None`
    pub fn at_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len() {
            Some(unsafe { self.data.add(index).as_mut() })
        } else {
            None
        }
    }

    /// Returns array as slice of `T`
    #[inline]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.size) }
    }

    /// Returns array as mutable slice of `T`
    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.size) }
    }

    /// Returns subslice of the array
    pub fn get<R>(&self, range: R) -> Option<&[T]>
    where R: RangeBounds<usize> {
        let (start, end) = self.handle_bounds(&range);

        if start > self.len() || end > self.len() {
            return None;
        }

        Some(unsafe { slice::from_raw_parts(self.data.add(start).as_ptr(), end - start) })

    }

    /// Returna mutable sublslice of the array
    pub fn get_mut<R>(&mut self, range: R) -> Option<&mut [T]>
    where R: RangeBounds<usize> {
        let (start, end) = self.handle_bounds(&range);

        if start > self.len() || end > self.len() {
            return None;
        }

        Some(unsafe { slice::from_raw_parts_mut(self.data.add(start).as_ptr(), end - start) })
    }

    /// Leaks the array while returning reference to its data
    pub unsafe fn leak<'l>(self) -> &'l mut [T] {
        let m = ManuallyDrop::new(self);
        unsafe {
            slice::from_raw_parts_mut(m.data.as_ptr(), m.len())
        }
    }

    /// Returns pointer to allocated array
    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Returns mutable pointer to allocated array
    pub const fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr()
    }

    /// Constructs `Vec<T>` from this `Array`
    #[cfg(all(feature="allocator", feature="vector"))]
    pub fn into_vec<const STEP: usize>(self) -> Vec<T, STEP> {
        let m = ManuallyDrop::new(self);
        unsafe { Vec::from_parts(m.data, m.len(), m.len()) }
    }

}

impl<T: Sized + Clone> Array<T> {

    /// Allocates array on the heap while copying all elements from the slice
    /// - **panics** if allocation or `T::clone()` fails
    pub fn from_slice(slice: &[T]) -> Self {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(slice.len()))
        } as *mut T;

        assert!(!data.is_null(), "failed to allocate memory for Array");

        let s = unsafe { slice::from_raw_parts_mut(data, slice.len()) };

        for (i, item) in s.iter_mut().enumerate() {
            *item = slice[i].clone();
        }

        Self {
            data: unsafe { NonNull::new_unchecked(data) },
            size: slice.len(),
        }

    }
}


impl<T: Sized + TryClone> Array<T>
where T::Error: Default {
    /// Tries to allocate array on the heap while copying ell elements from the slice
    /// - returns `Err` if allocation or `T::try_clone()` fails
    ///   - drops all already copied values and deallocates buffer
    pub fn try_from_slice(slice: &[T]) -> Result<Self, T::Error> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(slice.len()))
        } as *mut T;

        if data.is_null() {
            return Err(T::Error::default());
        }

        let s = unsafe { slice::from_raw_parts_mut(data, slice.len()) };

        for (i, item) in s.iter_mut().enumerate() {
            *item = slice[i].try_clone()?;
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(data) },
            size: slice.len(),
        })
    }
}










impl<T: Sized + Clone> Clone for Array<T> {
    fn clone(&self) -> Self {

        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(self.len()))
        } as *mut T;

        assert!(!data.is_null(), "failed to allocate memoyr for Array");

        let new = unsafe { slice::from_raw_parts_mut(data, self.len()) };
        let slice = self.as_slice();

        for (i, item) in new.iter_mut().enumerate() {
            *item = slice[i].clone();
        }

        Array {
            data: unsafe { NonNull::new_unchecked(data) },
            size: self.size
        }

    }
}


impl<T: Sized + TryClone> TryClone for Array<T>
where T::Error: Default {

    type Error = T::Error;

    fn try_clone(&self) -> Result<Self, Self::Error>
        where Self: Sized, Self::Error: Default {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout(self.size))
        } as *mut T;

        if data.is_null() {
            return Err(T::Error::default());
        }

        let new = unsafe { slice::from_raw_parts_mut(data, self.len()) };
        let slice = self.as_slice();

        for (i, item) in new.iter_mut().enumerate() {
            *item = match slice[i].try_clone() {
                Ok(i) => i,
                Err(_) => unsafe {
                    //  deallocate memory
                    drop_in_place(slice::from_raw_parts_mut(self.data.as_ptr(), i));
                    ALLOCATOR.dealloc(self.data.as_ptr() as *mut u8, Self::layout(self.len()));
                    return Err(T::Error::default());
                },
            };
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(data) },
            size: self.len(),
        })

    }
}










impl<T: Sized> Drop for Array<T> {
    fn drop(&mut self) {
        unsafe {
            drop_in_place(self.as_mut_slice());
            ALLOCATOR.dealloc(self.data.as_ptr() as *mut u8, Self::layout(self.size));
        }
    }
}

impl<T: Sized> Deref for Array<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.size) }
    }
}

impl<T: Sized> DerefMut for Array<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.size) }
    }
}

impl<T: Sized> Index<usize> for Array<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        match self.at(index) {
            Some(r) => r,
            None => panic!("Array[index]: index is out of bounds"),
        }
    }
}

impl<T: Sized> IndexMut<usize> for Array<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self.at_mut(index) {
            Some(m) => m,
            None => panic!("Array[index]: index is out of bounds"),
        }
    }
}


impl<'l, T> From<&'l [T]> for Array<T>
    where T: Sized + Clone {
    fn from(value: &'l [T]) -> Self {
        let arr = Array::new_uninit(value.len());
        let mut this = arr.as_ptr() as *mut T;

        for i in 0..value.len() {
            unsafe {
                this.write(value[i].clone());
                this = this.add(1);
            }
        }

        unsafe {
            Array::assume_init(arr)
        }

    }
}

impl<'l, T, const N: usize> From<&'l [T; N]> for Array<T>
    where T: Sized + Clone {
    fn from(value: &'l [T; N]) -> Self {
        let arr = Array::new_uninit(N);
        let mut this = arr.as_ptr() as *mut T;

        for i in 0..N {
            unsafe {
                this.write(value[i].clone());
                this = this.add(1);
            }
        }

        unsafe {
            Array::assume_init(arr)
        }
    }
}

impl<T, const N: usize> From<[T; N]> for Array<T>
    where T: Sized + Clone {
    fn from(value: [T; N]) -> Self {
        let arr = Array::new_uninit(N);
        let mut this = arr.as_ptr() as *mut T;

        for i in 0..N {
            unsafe {
                this.write(value[i].clone());
                this = this.add(1);
            }
        }

        unsafe {
            Array::assume_init(arr)
        }
    }
}
