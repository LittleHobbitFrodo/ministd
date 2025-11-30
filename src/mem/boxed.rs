//	mem/box.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

//! The (almost) standard implementation of `Box<T>` structure
//! - This `Box` cannot yet allocate arrays/slices
//!   - Use the `Array` structure to do so
//! 
//! **TODO**: Try to implement array/slice allocation directly in box

use core::{alloc::{GlobalAlloc, Layout}, any::Any, fmt::{Debug, Display, Pointer}, hash::Hash, iter, mem::{ManuallyDrop, MaybeUninit}, ops::{Deref, DerefMut}, option::Iter, pin::{Pin, pin}, ptr::{NonNull, drop_in_place}};

use crate::{ALLOCATOR, String, ToOwned, ToString, TryClone, alloc::layout_arr};



/// A pointer type that uniquely owns a heap allocation of type T
///
/// ## Implementation details
/// **Please note**: The `ministd::Box` is not direct replacement for the `std::Box`
/// 
/// This implementation of `std::Box`-like structure prioritizes using **stable** language features
/// - The `Layout` of allocated data is stored within the structure to make it simpler to develop
///   - This enlarges the `ministd::Box` layout by another 8 bytes: the final layout of `ministd::Box` is the `24` bytes
#[repr(C)]
pub struct Box<T: ?Sized> {
    ptr: NonNull<T>,
    layout: Layout,
}

impl<T: ?Sized> Box<T> {
    /// Returns `NonNull` pointer to the allocated data
    /// - The pointer is **dangling** if `T` is zero sized
    pub const fn as_non_null(&self) -> NonNull<T> { self.ptr }

    /// Identifies the `Layout` used by `Box` for an instance of (sized) `T`
    pub const fn layout_sized() -> Layout
    where T: Sized {
        if size_of::<T>() == 0 {
            unsafe { Layout::from_size_align_unchecked(0, 0) }
        } else {
            Layout::new::<T>()
        }
    }

    /// Identifies the `Layout` used by `Box` for an instance of (unsized) `T`
    pub const fn layout_unsized(val: &T) -> Layout { Layout::for_value(val) }

}

impl<T: Sized> Box<[T]> {

    /// Identifies the `Layout` used by `Box` for an array of (sized) type `T`
    pub const fn layout_arr(size: usize) -> Layout {
        layout_arr::<T>(size)
    }

}

impl<T: Sized> Box<T> {

    /// Allocates memory on the heap and the places `x` into it
    /// - `panic`s on allocation failure
    #[inline]
    pub fn new(x: T) -> Box<T> {
        Box::alloc(x).expect("failed to allocate memory")
    }

    /// Constructs new `Box` with uninitialized contents
    /// - `panic`s on allocation failure
    #[inline]
    pub fn new_uninit() -> Box<MaybeUninit<T>> {
        Box::alloc_uninit().expect("failed to allocate memory")
    }

    /// Constructs new `Box` with uninitialized memory
    /// - All bytes are set to `0`
    /// - `panic`s on allocation failure
    #[inline]
    pub fn new_zeroed() -> Box<MaybeUninit<T>> {
        Box::alloc_zeroed().expect("failed to allocate memory")
    }

    /// Constructs new `Pin<Box>`
    /// - `panic`s on allocation failure
    #[inline]
    pub fn pin(x: T) -> Pin<Box<T>> where T: Unpin {
        Pin::new(Self::new(x))
    }

    /// Tries to allocate memory on the heap and place `x` into it
    /// - `drop`s `x` on allocation failure and returns `Err`
    #[inline]
    pub fn try_new(x: T) -> Result<Box<T>, ()> {
        Box::alloc(x)
    }

    /// Contructs new `Box` with uninitialized contents
    /// - Returns `Err` on allocation error
    #[inline]
    pub fn try_new_uninit() -> Result<Box<MaybeUninit<T>>, ()> {
        Box::alloc_uninit()
    }

    /// Constructs new `Box` with uninitialized contents
    /// - All byes are set to `0`
    /// - Returns `Err` on allocation failure
    #[inline]
    pub fn try_new_zeroed() -> Result<Box<MaybeUninit<T>>, ()> {
        Box::alloc_zeroed()
    }

    /// Constructs new `Pin<Box>`
    /// - `drop`s `x` on allocation failure and returns `Err`
    pub fn try_pin(x: T) -> Result<Pin<Box<T>>, ()>
    where T: Unpin {
        Ok(Pin::new(Box::try_new(x)?))
    }

}

impl<T: Sized> Box<[T]> {

    /// Constructs new `Box` with uninitialized contents
    /// - `panic`s on allocation failure
    pub fn new_uninit_slice(size: usize) -> Box<[MaybeUninit<T>]> {
        Box {
            ptr: unsafe { ALLOCATOR.allocate_array_uninit(size).expect("failed to allocate memory") },
            layout: layout_arr::<T>(size),
        }
    }

    /// Constructs new `Box` for an array with all bytes set to `0`
    /// - `panic`s on allocation failure
    pub fn new_zeroed_slice(size: usize) -> Box<[MaybeUninit<T>]> {
        Box {
            ptr: unsafe { ALLOCATOR.allocate_array_zeroed(size).expect("failed to allocate memory") },
            layout: layout_arr::<T>(size)
        }
    }

    /// Tries to construct new `Box` with uninitialized contents
    pub fn try_new_uninit_slice(size: usize) -> Result<Box<[MaybeUninit<T>]>, ()> {
        Ok(Box {
            ptr: unsafe { ALLOCATOR.allocate_array_uninit(size)? },
            layout: layout_arr::<T>(size)
        })
    }

    /// Tries to construct new `Box` with all bytes set to `0`
    pub fn try_new_zeroed_slice(size: usize) -> Result<Box<[MaybeUninit<T>]>, ()> {
        Ok(Box {
            ptr: unsafe { ALLOCATOR.allocate_array_zeroed(size)? },
            layout: layout_arr::<T>(size),
        })
    }

    /// Allocates an array, uses the closure to determine the value of each element
    /// - `panic`s on allocation failure
    pub fn new_slice_with<F: FnMut() -> T>(size: usize, mut f: F) -> Box<[T]> {
        Box {
            ptr: unsafe { ALLOCATOR.allocate_array_with(size, &mut f).expect("failed to allocate memory") },
            layout: layout_arr::<T>(size),
        }
    }


    /// Tries to allocate an array, uses the closure to determine the value of each element
    pub fn try_new_slice_with<F: FnMut() -> T>(size: usize, mut f: F) -> Result<Box<[T]>, ()> {
        Ok(Box {
            ptr: unsafe { ALLOCATOR.allocate_array_with(size, &mut f)? },
            layout: layout_arr::<T>(size)
        })
    }

    /// Allocates an array, uses the `default` value for each element
    /// - `panic`s on allocation failure
    pub fn new_slice_default(size: usize) -> Box<[T]>
    where T: Default {
        Box {
            ptr: unsafe {
                let mut arr = ALLOCATOR.allocate_array_uninit(size).expect("failed to allocate memory");

                for i in arr.as_mut() {
                    i.write(T::default());
                }

                NonNull::slice_from_raw_parts(arr.cast(), size)
            },
            layout: layout_arr::<T>(size)
        }
    }

    /// Tries to allocate an array, uses the `default` value for each element
    /// - Returns `Err` on allocation failure
    pub fn try_new_slice_default(size: usize) -> Result<Box<[T]>, ()>
    where T: Default {
        Ok(Box {
            ptr: unsafe {
                let mut arr = ALLOCATOR.allocate_array_uninit(size)?;

                for i in arr.as_mut() {
                    i.write(T::default());
                }

                NonNull::slice_from_raw_parts(arr.cast(), size)
            },
            layout: layout_arr::<T>(size)
        })
    }

    /// Tries to convert the inner slice into and array of `N` elements
    /// - Returns `None` if `N` is not exaclty equal to the size of the slice
    pub fn into_array<const N: usize>(self) -> Option<Box<[T; N]>> {
        if N == self.ptr.len() {

            let Box { ptr, layout } = *ManuallyDrop::new(self);

            Some(Box {
                ptr: ptr.cast(),
                layout,
            })

        } else {
            unsafe { ManuallyDrop::drop(&mut ManuallyDrop::new(self)) };
            None
        }
    }

    /// Converts the inner slice into an array of `N` elements
    /// 
    /// # **Safety**
    /// The caller is to guarantee that the length of the inner slice is exaclty equal to `N`
    /// - Other use will result in undefined behaviour
    pub const unsafe fn into_array_unchecked<const N: usize>(self) -> Box<[T; N]> {
        let ptr = self.ptr;
        let layout = self.layout;

        let _ = ManuallyDrop::new(self);

        Box {
            ptr: ptr.cast(),
            layout
        }
    }

    pub fn from_slice<'a, U: Sized>(slice: &'a [U]) -> Self
    where T: From<&'a U> {
        let ptr = unsafe {
            let mut other = slice.iter();

            ALLOCATOR.allocate_array_with(slice.len(), &mut || T::from(other.next().unwrap()) )
                .expect("failed to allocate memory")
        };

        Box {
            ptr,
            layout: layout_arr::<T>(slice.len())
        }

    }

    pub fn clone_from_slice(slice: &[T]) -> Self
    where T: Clone {

        let ptr: NonNull<[T]> = unsafe {
            let mut other = slice.iter();
            ALLOCATOR.allocate_array_with(slice.len(), &mut || other.next().unwrap().clone() )
                .expect("failed to allocate memory")
        };

        Box {
            ptr,
            layout: layout_arr::<T>(slice.len())
        }

    }

}

impl<T: Sized> Box<MaybeUninit<T>> {

    /// Converts to `Box<T>`
    /// 
    /// # **Safety**
    /// As with `MaybeUninit::assume_init``, it is up to the caller to guarantee that the value really is in an initialized state. Calling this when the content is not yet fully initialized causes immediate undefined behavior
    pub const unsafe fn assume_init(self) -> Box<T> {
        let ptr = self.ptr;
        let layout = self.layout;

        let _ = ManuallyDrop::new(self);
        Box {
            ptr: ptr.cast(),
            layout,
        }
    }

    /// Safely converts to `Box<T>` by initializing the inner data
    pub fn write(mut self, val: T) -> Box<T> {
        
        unsafe { self.ptr.as_mut().write(val) };

        let m = ManuallyDrop::new(self);
        let Box { ptr, layout } = *m;

        Box {
            ptr: ptr.cast(),
            layout,
        }
    }

}

impl<T: Sized> Box<[MaybeUninit<T>]> {

    /// Converts to `Box<[T]>`
    /// 
    /// # **Safety**
    ///  As with `MaybeUninit::assume_init``, it is up to the caller to guarantee that the value really is in an initialized state. Calling this when the content is not yet fully initialized causes immediate undefined behavior
    pub const unsafe fn assume_init(self) -> Box<[T]> {

        let ptr = self.ptr;
        let layout = self.layout;

        let _ = ManuallyDrop::new(self);

        Box {
            ptr: NonNull::slice_from_raw_parts(ptr.cast(), ptr.len()),
            layout,
        }
    }

    /// Safely converts to `Box<[T]> by initializing the inner array
    pub fn write(self, value: T) -> Box<[T]>
    where T: Clone {
        let Box { mut ptr, layout} = *ManuallyDrop::new(self);

        unsafe {
            for i in ptr.as_mut() {
                i.write(value.clone());
            }
        }

        Box {
            ptr: NonNull::slice_from_raw_parts(ptr.cast(), ptr.len()),
            layout,
        }
    }

}

impl Box<dyn Any> {

    /// Attempts to downcast the box to a concrete type
    pub fn downcast<T: Any>(self) -> Result<Box<T>, Self> {
        if self.is::<T>() {
            let Box { ptr, layout } = *ManuallyDrop::new(self);
            Ok(Box {
                ptr: ptr.cast(),
                layout,
            })
        } else {
            Err(self)
        }
    }

    /// Forces downcasting the inner value without checking if the value if really an instance of `T`
    /// 
    /// # **Safety**
    /// It is up to the caller to guarantee that the inner value is an instance of `T`. Calling this on an `Box` that does not store `dyn T` will result in immediate undefined behaviour
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Box<T> {
        let Box { ptr, layout } = *ManuallyDrop::new(self);
        Box {
            ptr: ptr.cast(),
            layout
        }
    }

}

impl Box<dyn Any + Send> {

    /// Attempts to downcast the box to a concrete type
    pub fn downcast<T: Any + Send>(self) -> Result<Box<T>, Self> {
        if self.is::<T>() {
            let Box { ptr, layout } = *ManuallyDrop::new(self);
            Ok(Box {
                ptr: ptr.cast(),
                layout,
            })
        } else {
            Err(self)
        }
    }

    /// Forces downcasting the inner value without checking if the value if really an instance of `T`
    /// 
    /// # **Safety**
    /// It is up to the caller to guarantee that the inner value is an instance of `T`. Calling this on an `Box` that does not store `dyn T` will result in immediate undefined behaviour
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Box<T> {
        let Box { ptr, layout } = *ManuallyDrop::new(self);
        Box {
            ptr: ptr.cast(),
            layout
        }
    }

}


impl<T: ?Sized> Box<T> {

    /// Constructs `Box` from raw pointer and `Layout`
    /// 
    /// # **Safety**
    /// The pointer must be allocated by the `ministd::ALLOCATOR` allocator using the passed `Layout`
    pub unsafe fn from_raw(ptr: *mut T, layout: Layout) -> Box<T> {
        Box { ptr: NonNull::new(ptr).expect("pointer is null"), layout, }
    }

    /// Constructs `Box` from an `NonNull` pointer and `Layout`
    /// 
    /// # **Safety**
    /// The pointer must be allocated by the `ministd::ALLOCATOR` allocator using the specified `Layout`
    pub const unsafe fn from_non_null(ptr: NonNull<T>, layout: Layout) -> Box<T> {
        Box { ptr, layout }
    }

    /// Destructs the `Box` and returns its pointer and layout
    pub fn into_raw(self) -> (*mut T, Layout) {
        let Box { ptr, layout } = *ManuallyDrop::new(self);
        (ptr.as_ptr(), layout)
    }

    /// Destructs the `Box` and returns its pointer and layout
    pub fn into_non_null(self) -> (NonNull<T>, Layout) {
        let Box { ptr, layout } = *ManuallyDrop::new(self);
        (ptr, layout)
    }

    /// Returns pointer to the inner data
    #[inline]
    pub const fn as_ptr(&self) -> *const T { self.ptr.as_ptr() }

    /// Returns mutable pointer to the innner data
    #[inline]
    pub const fn as_mut_ptr(&mut self) -> *mut T { self.ptr.as_ptr() }

    /// Leaks the memory and returns mutable reference to the data
    /// - You can choose the lifetime of the reference
    pub unsafe fn leak<'l>(self) -> &'l mut T {
        let Box { mut ptr, .. } = *ManuallyDrop::new(self);
        unsafe { ptr.as_mut() }
    }

    /// Leaks the memory and returns mutable reference to the data and its layout
    /// - You can choose the lifetime of the reference
    pub unsafe fn leak_with_layout<'l>(self) -> (&'l mut T, Layout) {
        let Box { mut ptr, layout } = *ManuallyDrop::new(self);
        (unsafe { ptr.as_mut() }, layout)
    }

    /// `Pin`s the `Box`
    pub fn into_pin(self) -> Pin<Self>
    where T: Unpin {
        let Box { ptr, layout } = *ManuallyDrop::new(self);
        Pin::new(Box { ptr, layout })
    }

}



impl<T: Sized> Box<T> {

    /// Allocates memory for `x` and places it into it
    /// - Does not allocate data if `T` is zero sized
    /// - `x` is `drop`ped on allocation failure
    fn alloc(x: T) -> Result<Box<T>, ()> {
        if size_of::<T>() == 0 {
            Ok(Box {
                ptr: NonNull::dangling(),
                layout: unsafe { Layout::from_size_align_unchecked(0, 0) }
            })
        } else {
            Ok(Box{
                ptr: unsafe { ALLOCATOR.allocate(x).map_err(|v| drop(v) ) }?,
                layout: Layout::new::<T>(),
            })
        }
    }


    /// Allocates uninitialized memory for an instance of `T`
    /// - Does not allocate data if `T` is zero sized
    fn alloc_uninit() -> Result<Box<MaybeUninit<T>>, ()> {
        if size_of::<T>() == 0 {
            Ok(Box {
                ptr: NonNull::dangling(),
                layout: unsafe {
                    Layout::from_size_align_unchecked(0, 0)
                }
            })
        } else {
            Ok(Box {
                ptr: unsafe { ALLOCATOR.allocate_uninit() }?,
                layout: Layout::new::<T>()
            })
        }
    }

    /// Allocates zeroed memory for an instance of `T`
    /// - Does not allocate data if `T` is zero sized
    fn alloc_zeroed() -> Result<Box<MaybeUninit<T>>, ()> {
        if size_of::<T>() == 0 {
            Ok(Box {
                ptr: NonNull::dangling(),
                layout: unsafe {
                    Layout::from_size_align_unchecked(0, 0)
                }
            })
        } else {
            Ok(Box {
                ptr: unsafe { ALLOCATOR.allocate_zeroed() }?,
                layout: Layout::new::<T>()
            })
        }
    }

}

impl<T: ?Sized> Box<T> {
    /// Returns `true` if the `Box` did not allocate any memory
    /// - `size_of::<T>() == 0`
    pub fn is_dangling(&self) -> bool { self.layout.size() == 0}
}




impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> crate::Borrow<T> for Box<T> {
    fn borrow(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> crate::BorrowMut<T> for Box<T> {
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized + Clone> Clone for Box<T> {
    fn clone(&self) -> Self {
        Self::new(self.as_ref().clone())
    }
}

impl<T: Sized + TryClone> TryClone for Box<T> {
    type Error = ();
    fn try_clone(&self) -> Result<Self, Self::Error>
        where Self: Sized {
        Box::try_new(self.as_ref().try_clone().map_err(|_| () )?)
    }
}

impl<T: Sized + Default> Default for Box<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Sized + Default + Unpin> Default for Pin<Box<T>> {
    fn default() -> Self {
        Box::pin(T::default())
    }
}

impl<T: Display> Display for Box<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl<T: Debug> Debug for Box<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "Box {{ ptr: {:p}, layout: {:?}, value: {:?} }}", self.ptr, self.layout, self.as_ref())
        } else {
            write!(f, "{:?}", self.as_ref())
        }
    }
}

impl<T: ?Sized> Deref for Box<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Box<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}


impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        if !self.is_dangling() {
            unsafe {
                drop_in_place(self.ptr.as_mut());
                ALLOCATOR.dealloc(self.ptr.as_ptr().cast(), self.layout);
            }
        }
    }
}

impl<T: Clone> From<&[T]> for Box<[T]> {
    fn from(value: &[T]) -> Self {
        let ptr = unsafe {
            let mut other = value.iter();

            ALLOCATOR.allocate_array_with(value.len(), &mut || other.next().unwrap().clone() )
                .expect("failed to allocate memory")
        };

        Box {
            ptr,
            layout: layout_arr::<T>(value.len())
        }
    }
}

impl<T: Clone> From<&mut [T]> for Box<[T]> {
    fn from(value: &mut [T]) -> Self {
        Self::clone_from_slice(value)
    }
}

impl<T: Sized, const N: usize> From<[T; N]> for Box<[T]> {
    fn from(value: [T; N]) -> Self {
        let m = ManuallyDrop::new(value);
        let mut b = Box::new_uninit_slice(N);

        unsafe {
            core::ptr::copy_nonoverlapping(m.as_ptr(), b.as_mut_ptr() as *mut T, N);
            b.assume_init()
        }
    }
}

//  From<Box<u8>> for String
//  From<Box<T>> for Cow<'l, T>
//  From<Box<[T]>> for Cow<'l, [T]>
//impl<T: ?Sized + Display> ToString for Box<T>

impl<T: Hash> Hash for Box<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}


impl<'l, T: Sized> IntoIterator for &'l Box<[T]> {
    type IntoIter = core::slice::Iter<'l, T>;
    type Item = &'l T;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}


impl<T, U> PartialEq<U> for Box<T>
where T: PartialEq<U> + ?Sized {
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_ref().eq(other)
    }

    #[inline]
    fn ne(&self, other: &U) -> bool {
        self.as_ref().ne(other)
    }
}

impl<T, U> PartialOrd<U> for Box<T>
where T: PartialOrd<U> + ?Sized {
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<core::cmp::Ordering> {
        self.as_ref().partial_cmp(other)
    }

    #[inline]
    fn ge(&self, other: &U) -> bool { self.as_ref().ge(other) }

    #[inline]
    fn gt(&self, other: &U) -> bool { self.as_ref().gt(other) }

    #[inline]
    fn le(&self, other: &U) -> bool { self.as_ref().le(other) }

    #[inline]
    fn lt(&self, other: &U) -> bool { self.as_ref().lt(other) }

}

impl<T: ?Sized> Pointer for Box<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:p}", self.ptr)
    }
}

unsafe impl<T: ?Sized + Sync> Sync for Box<T> {}

unsafe impl<T: ?Sized + Send> Send for Box<T> {}

impl<T: ?Sized + Clone> ToOwned for Box<T> {
    type Owned = T;
    fn to_owned(&self) -> Self::Owned {
        self.as_ref().clone()
    }
}




/*use core::alloc::{GlobalAlloc, Layout};
use core::fmt::Display;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::{drop_in_place, NonNull};
use core::ops::{Deref, DerefMut};

use crate::TryClone;
use crate::{mem::alloc::ALLOCATOR};


/// `Box` is used to safely allocate and deallocate memory of type `T`
/// 
/// Use `Array` to allocate arrays
#[repr(transparent)]
pub struct Box<T: Sized> {
    data: NonNull<T>,
}

impl<T: Sized> Box<T> {

    /// Describes memory layout of single element `Box`
    pub const fn layout() -> Layout {
        Layout::new::<T>()
    }

    /// Allocates memory on heap with `val` value
    /// - panics if allocation fails
    pub fn new(val: T) -> Self {
        Self {
            data: match unsafe { ALLOCATOR.allocate(val) } {
                Ok(data) => data,
                Err(_) => panic!("failed to allocate memory for Box"),
            },
        }
    }

    /// Tries to allocate memory with some value
    /// - returns `Err` if allocation fails
    pub fn try_new(val: T) -> Result<Self, ()> {
        Ok(Self {
            data: unsafe { ALLOCATOR.allocate(val).map_err(|v| {drop(v); ()} )? },
        })
    }

    /// Allocates memory on heap and leaves it uninitialized
    /// - panics if allocation fails
    pub fn new_uninit() -> Box<MaybeUninit<T>> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout())
        } as *mut MaybeUninit<T>;

        assert!(!data.is_null(), "failed to allocate memory for Box");

        Box {
            data: unsafe { NonNull::new_unchecked(data) },
        }
    }

    /// Tries to allocate memory on heap and leaves it uninitialized
    /// - returns `Err` if allocation fails
    pub fn try_new_uninit() -> Result<Box<MaybeUninit<T>>, ()> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout())
        } as *mut MaybeUninit<T>;

        if data.is_null() {
            return Err(());
        }

        Ok(Box {
            data: unsafe { NonNull::new_unchecked(data) },
        })
    }

    /// Allocates memory on heap and forces all bytes to 0
    /// - panics if allocation fails
    pub fn new_zeroed() -> Box<MaybeUninit<T>> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout())
        } as *mut MaybeUninit<T>;

        assert!(!data.is_null(), "failed to allocate memory for Box");

        Box {
            data: unsafe { NonNull::new_unchecked(data) }
        }
    }

    /// Tries to allocate memory while forcing all bytes to 0
    /// - returns `Err` if allocation fails
    pub fn try_new_zeroed() -> Result<Box<MaybeUninit<T>>, ()> {
        let data = unsafe {
            ALLOCATOR.alloc(Self::layout())
        } as *mut MaybeUninit<T>;

        if data.is_null() {
            return Err(());
        }

        Ok(Box {
            data: unsafe { NonNull::new_unchecked(data) }
        })
    }

    /// Constructs `Box` from `NonNull`
    /// - use `Box::layout::<T>()` to describe Layout
    pub const fn from_non_null(ptr: NonNull<T>) -> Self {
        Self {
            data: ptr,
        }
    }

    /// Converts the `Box` to `NonNull` while consuming the Box
    pub unsafe fn into_non_null(self) -> NonNull<T> {
        let m = ManuallyDrop::new(self);
        m.data
    }

    /// Returns pointer to content of the `Box`
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Returns mutable pointer to content of the `Box`
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr()
    }

    /// Destroys the `Box` and passes reference to the data
    pub unsafe fn leak<'l>(self) -> &'l mut T {
        let mut m = ManuallyDrop::new(self);
        unsafe { m.data.as_mut() }
    }

    #[inline(always)]
    pub const fn as_non_null(&self) -> NonNull<T> {
        self.data
    }


}

impl<T: Sized> Box<MaybeUninit<T>> {

    /// Tells the compiler to treat this `Box` as initialized
    pub unsafe fn assume_init(self) -> Box<T> {
        let m = ManuallyDrop::new(self);
        Box {
            data: unsafe { NonNull::new_unchecked(m.data.as_ptr() as *mut T) }
        }
    }

}

impl<T: Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            drop_in_place(self.data.as_ptr());
            ALLOCATOR.dealloc(self.data.as_ptr() as *mut u8, Self::layout());
        }
    }
}


impl<T: Display> Display for Box<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", *unsafe { self.data.as_ref() })
    }
}

impl<T> Deref for Box<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.data.as_ref() }
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.data.as_mut() }
    }
}

impl<T> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<T> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.data.as_mut() }
    }
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Self {
        let val = self.as_ref().clone();
        Self {
            data: match unsafe { ALLOCATOR.allocate(val) } {
                Ok(data) => data,
                Err(_) => panic!("failed to allocate memory for Box"),
            }
        }
    }
}

impl<T: Sized + TryClone> TryClone for Box<T>
where T: Sized + TryClone, T::Error: Default {

    type Error = T::Error;

    fn try_clone(&self) -> Result<Self, Self::Error>
    where Self: Sized, Self::Error: Default {
        
        let val = self.as_ref().try_clone()?;
        
        Ok(Self {
            data: match unsafe { ALLOCATOR.allocate(val) }{
                Ok(data) => data,
                Err(_) => return Err(T::Error::default())
            },
        })
    }
}
*/