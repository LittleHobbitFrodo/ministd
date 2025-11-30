//	mem/dynamic_buffer.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build


//! The `DynamicBuffer` is similar to the `Vec` collection, it does allocate, shrink and expand allocated data, but does not work with its contents



use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ptr::{copy_nonoverlapping, null_mut, NonNull};
use core::alloc::{Layout, GlobalAlloc};
use crate::{ALLOCATOR, TryClone};

/// returns the minimum of 3 values
#[inline(always)]
fn min_3(v1: usize, v2: usize, v3: usize) -> usize {
    core::cmp::min(core::cmp::min(v1, v2), v3)
}



/// The `DynamicBuffer` has only ne task: memory management
/// - it is not much useful on its own...
/// 
/// It simply allocates memory like vector would, but does not work with its content
/// - this also means that no elements will be dropped
/// 
/// ## Memory layout
/// The `DynamicBuffer` has standardized memory layout:
/// ```rust
/// pub struct DynamicBuffer<T, STEP, ALIGN> {
///     data: NonNull::<u8>,
///     cap: u32,
///     pub size: u32,
/// }
/// ```
/// 
/// ## Implementation details
/// - uses `self.capacity() > 0` to check if any data is allocated
///   - `self.data` is set to `NonNull::dangling()` if not
/// - `self.size` tells the DynamicBuffer how many elements to copy and is not modified by the `DynamicBuffer` directly
/// - `drop()` will only deallocate the buffer, no elements are dropped
/// 
/// ### Generic parameters
/// 1. `T`: defines the type that is allocated
/// 2. `STEP`: indicates how many elements should be preallocated
///     - set to 0 to enable **geometrical growth**
/// 3. `ALIGN` - defines custom alignment of the data
///     - set to 0 to use `align_of::<T>()`
///     - if used value is invalid, the `DynamicB uffer` will use the closest valid value
#[repr(C)]
pub struct DynamicBuffer<T: Sized, const STEP: usize, const ALIGN: usize = 0> {
    data: NonNull::<u8>,
    cap: u32,
    pub size: u32,
    _marker: PhantomData<T>,
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> DynamicBuffer<T, STEP, ALIGN> {


    /// The real alignment of the data
    const ALGN: usize = if ALIGN < align_of::<T>() {
        align_of::<T>()
    } else {
        if !ALIGN.is_power_of_two() {
            ALIGN.next_power_of_two()
        } else {
            ALIGN
        }
    };


    /// Indicates whether the `ALIGN` generic parameter is valid
    //const VALID: bool = ALIGN == 0 || (ALIGN.is_power_of_two() && ALIGN >= align_of::<T>() );

    /// returns `Layout` describing memory layout for `self`
    /// - use `DynamicBuffer::layout_for_exact(capacity)` for other instances
    const fn layout(&self) -> Layout {
        Self::layout_for_exact(self.capacity())
    }

    /// Constructs empty DynamicBuffer with no allocated data
    pub const fn empty() -> Self {
        Self {
            data: NonNull::dangling(),
            cap: 0,
            size: 0,
            _marker: PhantomData,
        }
    }

    pub(crate) const fn from_raw(data: NonNull<T>, cap: u32, size: u32) -> DynamicBuffer<T, STEP, ALIGN> {
        Self {
            data: unsafe { NonNull::new_unchecked(data.as_ptr() as *mut u8) },
            cap,
            size,
            _marker: PhantomData,
        }
    }

    /// Constructs `DynamicBuffer<T>` with some elements allocated
    /// - **panics** if allocation fails
    /// - `size = 0`
    /// - `capacity` is aligned to `STEP`
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = Self::new_capacity(capacity);

        let l = Self::layout_for_exact(cap);

        let data = unsafe { ALLOCATOR.alloc(l) };

        assert!(!data.is_null(), "failed to allocate data");

        Self {
            data: unsafe { NonNull::new_unchecked(data) },
            cap: cap as u32,
            size: 0,
            _marker: PhantomData
        }
    }

    /// Tries to construct `DynamicBuffer<T>` with some elements allocated
    /// - returns `Err` if allocation fails
    /// - `size = 0`
    /// - `capacity` is aligned to `STEP`
    pub fn try_with_capacity(capacity: usize) -> Result<Self, ()> {
        let cap = Self::new_capacity(capacity);

        let l = Self::layout_for_exact(cap);

        let data = unsafe { ALLOCATOR.alloc(l) };

        if data.is_null() {
            return Err(());
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(data) },
            cap: cap as u32,
            size: 0,
            _marker: PhantomData
        })
    }


    /// Constructs `DynamicBuffer<T>` with some elements allocated
    /// - **panics** if allocation fails
    /// - `size = 0`
    /// - `capacity` is not aligned to `STEP`
    pub fn with_exact_capacity(capacity: usize) -> Self {
        let l = Self::layout_for_exact(capacity);

        let data = unsafe { ALLOCATOR.alloc(l) };

        assert!(!data.is_null(), "failed to allocate data");

        Self {
            data: unsafe { NonNull::new_unchecked(data) },
            cap: capacity as u32,
            size: 0,
            _marker: PhantomData
        }
    }

    /// Tries to construct `DynamicBuffer<T>` with some elements allocated
    /// - returns `Err` if allocation fails
    /// - `size = 0`
    /// - `capacity` is not aligned to `STEP`
    pub fn try_with_exact_capacity(capacity: usize) -> Result<Self, ()> {
        let l = Self::layout_for_exact(capacity);

        let data = unsafe { ALLOCATOR.alloc(l) };

        if data.is_null() {
            return Err(());
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(data) },
            cap: capacity as u32,
            size: 0,
            _marker: PhantomData
        })
    }

    /// Constructs `DynamicBuffer<T>` with some elements allocated and zeroed memory
    /// - **panics** if allocation fails
    /// - `size = 0`
    pub fn with_capacity_zeroed(capacity: usize) -> Self {
        let cap = Self::new_capacity(capacity);

        let l = Self::layout_for_exact(cap);
        
        let data = unsafe { ALLOCATOR.alloc_zeroed(l) };

        assert!(!data.is_null(), "failed to allocate data");

        Self {
            data: unsafe { NonNull::new_unchecked(data) },
            cap: cap as u32,
            size: 0,
            _marker: PhantomData
        }
    }

    /// Tries to construct `DynamicBuffer<T>` with some elements allocated and zeroed memory
    /// - returns `Err` if allocation fails
    /// - `size = 0`
    pub fn try_with_capacity_zeroed(capacity: usize) -> Result<Self, ()> {
        let cap = Self::new_capacity(capacity);

        let l = Self::layout_for_exact(cap);

        let data = unsafe { ALLOCATOR.alloc(l) };

        if data.is_null() {
            return Err(());
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(data) },
            cap: cap as u32,
            size: 0,
            _marker: PhantomData
        })
    }

    /// Resizes (reallocates) the buffer to certain size
    /// - `size` is aligned to `STEP`
    /// - **no elements are dropped**
    /// - **no-op** if `capacity` would be the same`
    /// - if `self.is_empty()` allocates new data
    /// - **panics** if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    pub fn resize(&mut self, size: usize) {

        if self.capacity() == size {
            return
        }

        let wanted = Self::new_capacity(size);

        let layout = Self::layout_for_exact(wanted);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        assert!(!new.is_null(), "failed to allocate memory");

        if self.capacity() > 0 {
            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), wanted);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = wanted as u32;

    }



    /// Tries to resize (reallocate) the buffer to certain size
    /// - `size` is aligned to `STEP`
    /// - **no elements are dropped**
    /// - **no-op** if `capacity` would be the same`
    /// - if `self.is_empty()` allocates new data
    /// - returns `Err` if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    pub fn try_resize(&mut self, size: usize) -> Result<(), ()> {

        if self.capacity() == size {
            return Ok(())
        }

        let wanted = Self::new_capacity(size);

        let layout = Self::layout_for_exact(wanted);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        if new.is_null() {
            return Err(());
        }

        if self.capacity() > 0 {

            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), wanted);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = wanted as u32;

        Ok(())

    }

    /// Resizes (reallocates) the buffer to exact size
    /// - **no-op** if `capacity` would be the same`
    /// - **no elements are dropped**
    /// - if `self.is_empty()` allocates new data
    /// - **panics** if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    ///   - Copies all elements if `self.size > self.capacity()`
    pub fn resize_exact(&mut self, size: usize) {

        if size == self.capacity() {
            return
        }

        let layout = Self::layout_for_exact(size);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        assert!(!new.is_null(), "failed to allocate memory");

        if self.capacity() > 0 {
            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), size);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = size as u32;

    }

    /// Tries to resize (reallocate) the buffer to exact size
    /// - **no-op** if `capacity` would be the same`
    /// - **no elements are dropped**
    /// - if `self.is_empty()` allocates new data
    /// - **panics** if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    ///   - Copies all elements if `self.size > self.capacity()`
    pub fn try_resize_exact(&mut self, size: usize) -> Result<(), ()> {

        let layout = Self::layout_for_exact(size);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        if new.is_null() {
            return Err(());
        }

        if self.capacity() > 0 {
            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), size);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = size as u32;

        Ok(())

    }


    /// Expands the `capacity` by `STEP` elements
    /// - this function always reallocates memory
    /// - **panics** if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    ///   - Copies all elements if `self.size > self.capacity()`
    pub fn expand(&mut self) {

        let wanted = Self::next_capacity(self.capacity());

        let layout = Self::layout_for_exact(wanted);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        assert!(!new.is_null(), "failed to allocate memory");

        if self.capacity() > 0 {
            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), wanted);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = wanted as u32;

    }

    /// Tries to expand the `capacity` by `STEP` elements
    /// - this function always reallocates memory
    /// - returns `Err` if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    ///   - Copies all elements if `self.size > self.capacity()`
    pub fn try_expand(&mut self) -> Result<(), ()> {

        let wanted = Self::next_capacity(self.capacity());

        let layout = Self::layout_for_exact(wanted);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        if new.is_null() {
            return Err(());
        }

        if self.capacity() > 0 {
            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), wanted);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = wanted as u32;

        Ok(())

    }

    /// Expands the `capacity` by `STEP * steps` elements
    /// - this function always reallocates memory
    /// - **panics** if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    ///   - Copies all elements if `self.size > self.capacity()`
    pub fn expand_by(&mut self, steps: usize) {

        let wanted = Self::next_capacity(self.capacity() + (STEP * steps));

        let layout = Self::layout_for_exact(wanted);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        assert!(!new.is_null(), "failed to allocate memory");

        if self.capacity() > 0 {
            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), wanted);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = wanted as u32;

    }

    /// Tries to expanf the `capacity` by `STEP * steps` elements
    /// - this function always reallocates memory
    /// - returns `Err` if allocation fails
    /// - **Copies exactly `self.size` elements to the new location**
    ///   - Copies all elements if `self.size > self.capacity()`
    pub fn try_expand_by(&mut self, steps: usize) -> Result<(), ()> {

        let wanted = Self::next_capacity(self.capacity() + (STEP * steps));

        let layout = Self::layout_for_exact(wanted);

        let new = unsafe { ALLOCATOR.alloc(layout) };

        if new.is_null() {
            return Err(());
        }

        if self.capacity() > 0 {
            if self.size > 0 {
                unsafe {
                    //  eliminate buffer overflow
                    let copy_size = min_3(self.size as usize, self.capacity(), wanted);
                    copy_nonoverlapping(self.data.as_ptr(), new, copy_size * size_of::<T>());
                }
            }
            unsafe { ALLOCATOR.dealloc(self.data.as_ptr(), self.layout()); }
        }

        self.data = unsafe { NonNull::new_unchecked(new) };
        self.cap = wanted as u32;

        Ok(())

    }

    /// Constructs new `DynamicBuffer` from raw parts
    /// - **warning**: may be potentially unsafe
    pub fn from_raw_parts(ptr: NonNull<T>, layout: Layout) -> Self {
        Self {
            data: unsafe { NonNull::new_unchecked(ptr.as_ptr() as *mut u8) },
            cap: (layout.size()/size_of::<T>()) as u32,
            size: 0,
            _marker: PhantomData
        }
    }

    /// Decomposes `self` and returns individual parts of the `DynamicBuffer`
    /// - returns `(ptr, size, capacity)`
    /// - **panics** if no data is allocated
    pub unsafe fn into_parts(self) -> (NonNull<T>, usize, usize) {
        if self.capacity() == 0 {
            panic!("no memory allocated");
        }

        let m = ManuallyDrop::new(self);
        (m.data(), m.size as usize, m.capacity())
    }

    /// Decomposes `self` and returns individial parts of the `DynamicBuffer`
    /// - returns `(ptr, size, capacity)`
    pub unsafe fn into_raw_parts(self) -> (*mut T, usize, usize) {
        let m = ManuallyDrop::new(self);
        if m.capacity() > 0 {
            (m.as_ptr(), m.size as usize, m.capacity())
        } else {
            (null_mut(), m.size as usize, m.capacity())
        }
    }


}


impl<T: Sized, const STEP: usize, const ALIGN: usize> DynamicBuffer<T, STEP, ALIGN> {

    /// Returns the `STEP` generic for this instance
    pub const fn step(&self) -> usize { STEP }

    /// Returns the `STEP` generic for this type
    pub const fn step_of() -> usize { STEP }

    /// Checks if has any data allocated
    pub const fn has_data(&self) -> bool { self.capacity() > 0 }

    /// Returns alignment of this `DynamicBuffer` instance
    pub const fn align(&self) -> usize { Self::ALGN }

    /// Returns alignment for this generic `DynamicBuffer` type
    pub const fn align_of() -> usize { Self::ALGN }

    /// Describes memory layout for some capacity
    /// 
    /// - to be clear: (this may change in next versions)
    /// ```
    /// Layout::from_size_align_unchecked(size_of::<T>() * Self::cap_next(capacity), Self::align_of())
    /// ``````
    pub const fn layout_for(capacity: usize) -> Layout {
        //  ALGN should hold correct value
        unsafe { Layout::from_size_align_unchecked(size_of::<T>() * capacity, Self::align_of()) }
    }

    /// Describes memory layout for some capacity without aligning to `STEP`
    /// 
    /// - to be clear: (this may change in next version)
    /// ```
    /// Layout::from_size_align_unchecked(size_of::<T>() * capacity, Self::align_of())
    /// ```
    pub const fn layout_for_exact(capacity: usize) -> Layout {
        //  ALGN should hold correct value
        unsafe { Layout::from_size_align_unchecked(size_of::<T>() * capacity, Self::align_of()) }
    }

    /// aligns the capacity up to next generic `STEP`
    /// - result is greater than `STEP`
    /// - returns number of elements
    /// 
    /// ```
    /// if STEP == 0 {
    ///     (cap + 1).next_power_of_two()
    /// } else {
    ///     (cap + 1).next_multiple_of(STEP)
    /// }
    /// ```
    pub const fn next_capacity(cap: usize) -> usize {
        if STEP == 0 {
            (cap + 1).next_power_of_two()
        } else {
            (cap + 1).next_multiple_of(STEP)
        }
    }

    /// aligns the capacity to generic `STEP`
    /// - result is equal or greater than `STEP`
    /// - returns number of elements
    pub const fn new_capacity(cap: usize) -> usize {
        if STEP == 0 {
            cap.next_power_of_two()
        } else {
            cap.next_multiple_of(STEP)
        }
    }

    /// Returns number of elements allocated in the buffer
    pub const fn capacity(&self) -> usize { self.cap as usize }

    /// Indicates if no data is allocated
    pub const fn is_empty(&self) -> bool {
        self.capacity() == 0
    }

    /// Returns pointer to allocated data
    pub const fn as_ptr(&self) -> *mut T {
        self.data.as_ptr() as *mut T
    }

    pub const fn as_non_null(&self) -> NonNull<T> {
        self.data.cast()
    }

    /// Returns pointer to data as `NonNull`
    pub const fn data(&self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(self.data.as_ptr() as *mut T) }
    }


}


impl<T: Sized, const STEP: usize, const ALIGN: usize> Drop for DynamicBuffer<T, STEP, ALIGN> {
    fn drop(&mut self) {
        if self.capacity() > 0 {
            unsafe {
                ALLOCATOR.dealloc(self.data.as_ptr(), self.layout());
            }
        }
    }
}

impl<T: Sized, const STEP: usize, const ALIGN: usize> Clone for DynamicBuffer<T, STEP, ALIGN> {
    /// `DynamicBuffer::clone()` does **not copy** any data
    fn clone(&self) -> Self {
        if self.capacity() == 0 {
            Self {
                data: NonNull::dangling(),
                cap: 0,
                size: 0,
                _marker: PhantomData,
            }
        } else {
            let data = unsafe {
                ALLOCATOR.alloc(self.layout())
            };

            assert!(!data.is_null(), "failed to allocate memory");

            Self {
                data: unsafe { NonNull::new_unchecked(data) },
                cap: self.capacity() as u32,
                size: self.size,
                _marker: PhantomData,
            }
        }
    }
}


impl<T: Sized, const STEP: usize, const ALIGN: usize> TryClone for DynamicBuffer<T, STEP, ALIGN> {
    type Error = ();
    /// `DynamicBuffer::try_clone()` does **not copy** any data
    fn try_clone(&self) -> Result<Self, Self::Error>
    where Self: Sized, Self::Error: Default {

        if self.capacity() == 0 {
            Ok(Self {
                data: NonNull::dangling(),
                cap: 0,
                size: 0,
                _marker: PhantomData,
            })
        } else {

            let data = unsafe {
                ALLOCATOR.alloc(self.layout())
            };

            if data.is_null() {
                return Err(());
            }

            Ok(Self {
                data: unsafe { NonNull::new_unchecked(data) },
                cap: self.capacity() as u32,
                size: self.size,
                _marker: PhantomData,
            })
        }

    }
}
