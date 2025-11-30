//	mem/mod.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

//! The classic reference counted pointer `Rc`
//! - This implementation does not offer the `Weak` pointer yet

use core::alloc::GlobalAlloc;
use core::borrow::{Borrow, BorrowMut};
use core::fmt::{Debug, Display, Pointer};
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::{drop_in_place, write_bytes};
use core::{cell::Cell, ptr::NonNull};

mod rc_inner;

pub(crate) use rc_inner::*;

use crate::{alloc::*, TryClone, Cow, ToOwned};

/// A single-threaded reference-counting pointer
/// - With no `Weak` pointer unfortunately
pub struct Rc<T: Sized> {
    data: NonNull<RcInner<T>>,
    _not_sync_not_send: PhantomData<Cell<()>>,    //  for !Send
}



impl<T> Rc<T> {

    /// Constructs new `Rc<T>`
    /// - **panics** if allocation fails
    pub fn new(val: T) -> Self {
        Self {
            data: unsafe { ALLOCATOR.allocate(RcInner::new(val)).map_err(|inner| {drop(inner)} ) }.expect("allocation failed"),
            _not_sync_not_send: PhantomData
        }
    }

    /// Tries to construct new `Rc<T>`
    /// - returns `Err` if allocation fails
    pub fn try_new(val: T) -> Result<Self, ()> {
        Ok(Self {
            data: unsafe { ALLOCATOR.allocate(RcInner::new(val)).map_err(|inner| drop(inner) )? },
            _not_sync_not_send: PhantomData
        })
    }

    /// Constructs new `Rc<T>` with uninitalized content
    /// - **panics** if allocation fails
    pub fn new_uninit() -> Rc<MaybeUninit<T>> {

        Rc {
            data: unsafe {
                let data = (ALLOCATOR.alloc(Layout::new::<RcInner<MaybeUninit<T>>>())
                as *mut RcInner<MaybeUninit<T>>).as_mut().expect("allocation failed");

                data.set_strong(1);
                data.set_weak(1);
                

                NonNull::new_unchecked(data)
            },
            _not_sync_not_send: PhantomData
        }

    }

    /// Tries to construct new `Rc<T>` with uninitialized content
    /// - returns `Err` if allocation fails
    pub fn try_new_uninit() -> Result<Rc<MaybeUninit<T>>, ()> {

        Ok(Rc {
            data: unsafe {
                let data = (ALLOCATOR.alloc(Layout::new::<RcInner<MaybeUninit<T>>>())
                as *mut RcInner<MaybeUninit<T>>).as_mut().ok_or(())?;

                data.set_strong(1);
                data.set_weak(1);

                NonNull::new_unchecked(data)

            },
            _not_sync_not_send: PhantomData
        })

    }

    /// Constructs new `Rc<T>` with all bytes set to 0
    /// - **panics** if allocation fails
    pub fn new_zeroed() -> Rc<MaybeUninit<T>> {

        Rc {
            data: unsafe {
                let data = (ALLOCATOR.alloc(Layout::new::<RcInner<MaybeUninit<T>>>())
                as *mut RcInner<MaybeUninit<T>>).as_mut().expect("allocation failed");

                data.set_strong(1);
                data.set_weak(1);

                write_bytes(data.data_as_ptr(), 0, size_of::<T>());
                

                NonNull::new_unchecked(data)

            },
            _not_sync_not_send: PhantomData
        }

    }

    /// Tries to construct new `Rc<T>` with all bytes set to 0
    /// - returns `Err` if allocation fails
    pub fn try_new_zeroed() -> Result<Rc<MaybeUninit<T>>, ()> {

        Ok(Rc {
            data: unsafe {
                let data = (ALLOCATOR.alloc(Layout::new::<RcInner<MaybeUninit<T>>>())
                as *mut RcInner<MaybeUninit<T>>).as_mut().ok_or(())?;

                data.set_strong(1);
                data.set_weak(1);

                write_bytes(data.data_as_ptr(), 0, size_of::<T>());
                

                NonNull::new_unchecked(data)

            },
            _not_sync_not_send: PhantomData
        })

    }

    /// Returns the inner value, if the Rc has exactly one strong reference
    /// - else returns `self`
    /// - if returning -> deallocates the inner value
    pub fn try_unwrap(self) -> Result<T, Self> {
        
        let mut this = ManuallyDrop::new(self);
        let inner = this.inner_mut();
        
        if inner.strong() == 0 {

            let ret = unsafe { inner.data_as_ptr().read() };

            unsafe {
                ALLOCATOR.dealloc(inner.as_mut_ptr() as *mut u8, Layout::new::<RcInner<T>>());
            }

            //  do not drop `self`

            Ok(ret)

        } else {
            Err((*this).clone())
            //  `clone` does only create another `Rc` (no allocation)
        }
    }

    /// Destructs `self` and returns its value
    /// - returns `None` if there are more than one strong references
    pub fn into_inner(self) -> Option<T> {
        let mut this = ManuallyDrop::new(self);
        let inner = this.inner_mut();

        if inner.strong() == 0 {
            let ret = unsafe { inner.data_as_ptr().read() };

            unsafe {
                ALLOCATOR.dealloc(inner.as_mut_ptr() as *mut u8, Layout::new::<RcInner<T>>());
            }

            //  do not drop `self`
            Some(ret)

        } else {
            //  no need to drop `self`
            None
        }
    }



    /// Converts the `Rc` to pointer **without deleting the strong reference**
    /// - to prevent memory leak, this must be converted back to `Rc` by `Rc::from_raw`
    /// - `self` will not be dropped
    pub fn into_raw(self) -> *const T {
        let ptr = self.inner().data_as_ptr();
        _ = ManuallyDrop::new(self);
        ptr
    }

    /// Converts raw pointer to `Rc`
    /// 
    /// safety:
    /// - the pointer must be previously returned by `Rc::into_raw`
    ///   - otherwise use of this function may result in undefined behaviour
    /// - the strong reference counter is not increased
    pub const fn from_raw(ptr: *const T) -> Self {
        Self {
            //  sub 8 (2 * sizeof(u32) ) to get the actual address of the allocated memory
            //  - the pointer returned by the `Rc::into_raw` points to the data (T type), not the allocated memory
            //  - see `RcInner` structure
            data: unsafe { NonNull::new_unchecked(ptr.byte_offset(-8) as *mut RcInner<T> ) },
            _not_sync_not_send: PhantomData
        }
    }

    /*/// Creates new `Weak` pointer to this allocation
    #[inline]
    pub fn downgrade(&self) -> Weak<T> {
        self.inner().inc_weak();

        Weak { data: self.data }

    }*/

    /// Returns mutable reference to the data if there are no other weak or strong references
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.strong_count() == 0 && self.weak_count() == 0 {
            Some(unsafe { self.data.as_mut().data_mut() })
        } else {
            None
        }
    }

    /// Returns mutable reference to the data without doing any checks
    /// - use `Rc::get_mut` if possible
    pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        unsafe { self.data.as_mut().data_mut() }
    }

}

impl<T> Rc<MaybeUninit<T>> {

    pub const fn assume_init(self) -> Rc<T> {
        let data = self.data;
        _ = ManuallyDrop::new(self);
        Rc {
            data: unsafe { NonNull::new_unchecked(data.as_ptr() as *mut RcInner<T>) },
            _not_sync_not_send: PhantomData
        }
    }

}

impl<T> Rc<T> {

    /// Returns reference to the inner (allocated) value
    const fn inner(&self) -> &RcInner<T> {
        unsafe { self.data.as_ref() }
    }

    /// Returns mutable reference to the inner (allocated) value
    const fn inner_mut(&mut self) -> &mut RcInner<T> {
        unsafe {
            self.data.as_mut()
        }
    }

    /// Returns the status of the weak reference counter
    #[inline(always)]
    pub fn weak_count(&self) -> usize {
        self.inner().weak() as usize
    }

    /// Returns the status of the strong reference counter
    #[inline(always)]
    pub fn strong_count(&self) -> usize {
        self.inner().strong() as usize
    }

    /// Returns the pointer to the data
    pub fn as_ptr(&self) -> *const T {
        self.inner().data_as_ptr()
    }

    /// Checks if the two Rcs are pointing to the same allocation
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.data == other.data
    }

    /// Checks if the two Rcs are pointing to the same allocation
    /// - does exaclty the same as `Rc::ptr_eq`
    #[inline]
    pub fn is_same(&self, other: &Self) -> bool {
        self.data == other.data
    }

}

impl<T: Clone> Rc<T> {
    /// If we have the only reference to T then unwrap it. Otherwise, clone T and return the clone
    pub fn unwrap_or_clone(self) -> T {
        self.try_unwrap().unwrap_or_else(|rc| rc.inner().data().clone())
    }
}

impl<T: TryClone> Rc<T> {

    /// If we have the only reference to T then unwrap it. Otherwise, try cloning T and return the clone
    pub fn unwrap_or_tryclone(self) -> Result<T, T::Error> {

        let mut this = ManuallyDrop::new(self);
        let inner = this.inner_mut();

        
        if inner.strong() == 0 {

            let ret = unsafe { inner.data_as_ptr().read() };

            unsafe {
                ALLOCATOR.dealloc(inner.as_mut_ptr() as *mut u8, Layout::new::<RcInner<T>>());
            }

            //  do not drop `self`

            Ok(ret)

        } else {
            inner.data().try_clone()
            //  `clone` does only create another `Rc` (no allocation)
        }

    }

}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        let inner = self.inner_mut();
        inner.dec_strong();
        if inner.strong() == 0 {
            unsafe {
                //drop_in_place(inner.data_as_ptr());
                ALLOCATOR.delete(NonNull::new_unchecked(inner.as_mut_ptr()));
            }
        }
    }
}

impl<T> Clone for Rc<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.inner().inc_strong();
        Self {
            data: self.data,
            _not_sync_not_send: PhantomData
        }
    }
}

impl<T> Borrow<T> for Rc<T> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.inner().data()
    }
}
impl<T> BorrowMut<T> for Rc<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self.get_mut().expect("there are pointing references to this Rc")
    }
}


impl<T: Debug> Debug for Rc<T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.inner().data().fmt(f)
    }
}

impl<T: Display> Display for Rc<T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.inner().data().fmt(f)
    }
}

impl<T: Default> Default for Rc<T> {
    #[inline]
    fn default() -> Self {
        Rc::new(T::default())
    }
}

impl<T: PartialOrd> PartialOrd for Rc<T> {
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        self.inner().data().ge(&other.inner().data())
    }

    #[inline]
    fn gt(&self, other: &Self) -> bool {
        self.inner().data().gt(&other.inner().data())
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        self.inner().data().le(&other.inner().data())
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        self.inner().data().lt(&other.inner().data())
    }

    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.inner().data().partial_cmp(&other.inner().data())
    }
    
}

impl<T: PartialEq> PartialEq for Rc<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner().data().eq(&other.inner().data())
    }

    #[inline]
    fn ne(&self, other: &Self) -> bool {
        self.inner().data().ne(&other.inner().data())
    }

}

impl<T: core::cmp::Eq> core::cmp::Eq for Rc<T> {}


impl<T: Ord> Ord for Rc<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.inner().data().cmp(&other.inner().data())
    }
}

impl<T> Pointer for Rc<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:p}", self.data )
    }
}

impl<T> Unpin for Rc<T> {}


impl<'a, B> From<Cow<'a, B>> for Rc<B>
where
    B: ToOwned + Sized,
    Rc<B>: From<&'a B> + From<B::Owned>,
{
    #[inline]
    fn from(cow: Cow<'a, B>) -> Rc<B> {
        match cow {
            Cow::Borrowed(s) => Rc::from(s),
            Cow::Owned(s) => Rc::from(s),
        }
    }
}