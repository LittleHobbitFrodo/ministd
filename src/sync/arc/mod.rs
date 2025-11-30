//  mem/sync/arc/mod.rs (ministd crate)
//  this file originally belonged to baseOS project
//      an OS template on which to build

mod inner;
use inner::ArcInner;
use core::borrow::Borrow;
use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::Deref;
use core::panic::UnwindSafe;
use core::pin::Pin;
use core::ptr::{drop_in_place, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::alloc::{ALLOCATOR, GlobalAlloc};


/// A thread-safe reference-counting pointer. ‘Arc’ stands for ‘Atomically Reference Counted’.
/// 
/// ## Implementation details
/// - both `Arc` and `Weak` does keep the allocation
///   - if `strong` counter is zero, the value is dropped
///   - if both `strong` and `weak` counters are zero, the value is deallocated
pub struct Arc<T: Sized> {
    ptr: NonNull<ArcInner<T>>,
}

impl<T: Sized> Arc<T> {

    /// Constructs and allocates new `Arc`
    /// - **panics** if allocation fails
    pub fn new(data: T) -> Self {

        let ptr =  unsafe {
            ALLOCATOR.allocate(ArcInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(0),
                data,
            })
        };

        let ptr = match ptr {
            Ok(p) => p,
            Err(_) => panic!("Arc: allocation failed"),
        };

        Self { ptr }
    }

    /// Tries to contruct and allocate new `Arc`
    /// - return `Err` if allocation fails
    pub fn try_new(data: T) -> Result<Self, ()> {

        Ok(Self {
            ptr: unsafe {
                ALLOCATOR.allocate(ArcInner {
                    strong: AtomicUsize::new(1),
                    weak: AtomicUsize::new(0),
                    data
                })
            }.map_err(|inner| drop(inner) )?
        })

    }

    /// Constructs and allocates new `Arc` with uninitialized content
    /// - **panics** if allocation fails
    pub fn new_uninit() -> Arc<MaybeUninit<T>> {
        let a = unsafe {
            ALLOCATOR.allocate(ArcInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(0),
                data: MaybeUninit::uninit()
            })
        };

        let ptr = match a {
            Ok(p) => p,
            Err(_) => panic!("Arc: allocation failed"),
        };

        Arc { ptr }
    }

    /// Tries to construct and allocate new `Arc` with uninitialize content
    /// - **panics** if allocation fails
    pub fn try_new_uninit() -> Result<Arc<MaybeUninit<T>>, ()> {

        Ok(Arc {
            ptr: unsafe {
                ALLOCATOR.allocate(ArcInner {
                    strong: AtomicUsize::new(1),
                    weak: AtomicUsize::new(0),
                    data: MaybeUninit::uninit()
                })
            }.map_err(|inner| drop(inner) )?
        })

    }

    /// Constructs and allocates new `Arc` with zeroed content
    pub fn new_zeroed() -> Arc<MaybeUninit<T>> {
        let p = unsafe {
            ALLOCATOR.alloc_zeroed(ArcInner::<T>::layout())
            as *mut ArcInner<MaybeUninit<T>>
        };

        match NonNull::new(p) {
            Some(mut ptr) => {
                unsafe {
                    ptr.as_mut().strong = AtomicUsize::new(1);
                    ptr.as_mut().weak = AtomicUsize::new(0);
                }
                Arc { ptr }
            },
            None => panic!("Arc: allocation failed"),
        }
    }


    /// Tries to construct and allocate new `Arc` with zeroed content
    pub fn try_new_zeroed() -> Result<Arc<MaybeUninit<T>>, ()> {

        Ok(Arc {
            ptr: unsafe {

                let mut ptr = NonNull::new(ALLOCATOR.alloc_zeroed(ArcInner::<T>::layout())
                    as *mut ArcInner<MaybeUninit<T>>).ok_or(())?;

                ptr.as_mut().strong = AtomicUsize::new(1);
                ptr.as_mut().weak = AtomicUsize::new(0);

                ptr
                
            }
        })

    }

    /// Constructs a new `Arc<T>` while giving you a `Weak<T>` to the allocation, to allow you to construct a `T` which holds a weak pointer to itself
    /// - **panics** if allocation fails
    pub fn new_cyclic<F>(data_fn: F) -> Self
    where F: FnOnce(&Weak<T>) -> T {

        let mut ptr = unsafe {
            ALLOCATOR.allocate::<ArcInner<MaybeUninit<T>>>(ArcInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(0),
                data: MaybeUninit::uninit(),
            })
        }.map_err(|inner| drop(inner) ).expect("Arc: allocation failed");

        unsafe {
            let weak = Weak::new_from_inner(ptr.cast::<ArcInner<T>>().as_ref());

            ptr.as_mut().data.write(data_fn(&weak));
        }

        Self { ptr: ptr.cast::<ArcInner<T>>() }

    }

    /// Tries to construct a new `Arc<T>` while giving you a `Weak<T>` to the allocation, to allow you to construct a `T` which holds a weak pointer to itself
    /// - returns `Err` if allocation fails
    pub fn try_new_cyclic<F>(data_fn: F) -> Result<Self, ()>
    where F: FnOnce(&Weak<T>) -> T {

        let mut ptr = unsafe {
            ALLOCATOR.allocate::<ArcInner<MaybeUninit<T>>>(ArcInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(0),
                data: MaybeUninit::uninit(),
            })
        }.map_err(|inner| drop(inner) )?;

        unsafe {
            let weak = Weak::new_from_inner(ptr.cast::<ArcInner<T>>().as_ref());

            ptr.as_mut().data.write(data_fn(&weak));
        }

        Ok(Self { ptr: ptr.cast::<ArcInner<T>>() })

    }

    /// Tries to construct a new `Arc<T>` while giving you a `Weak<T>` to the allocation, to allow you to construct a `T` which holds a weak pointer to itself
    /// - closure is allowed to return an error
    /// - returns `Err` if allocation fails
    pub fn try_new_cyclic_result<F>(data_fn: F) -> Result<Self, ()>
    where F: FnOnce(&Weak<T>) -> Result<T, ()> {

        let mut ptr = unsafe {
            ALLOCATOR.allocate::<ArcInner<MaybeUninit<T>>>(ArcInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(0),
                data: MaybeUninit::uninit(),
            })
        }.map_err(|inner| drop(inner) )?;

        unsafe {
            let weak = Weak::new_from_inner(ptr.cast::<ArcInner<T>>().as_ref());

            let val = match data_fn(&weak) {
                Ok(v) => v,
                Err(_) => {
                    ALLOCATOR.dealloc(ptr.as_ptr() as *mut u8, ArcInner::<T>::layout());
                    return Err(());
                }
            };

            ptr.as_mut().data.write(val);
        }

        Ok(Self { ptr: ptr.cast::<ArcInner<T>>() })

    }


    pub fn pin(data: T) -> Pin<Arc<T>> {
        unsafe { Pin::new_unchecked(Arc::new(data)) }
    }


    /// Constructs new `Arc` from an inner value
    /// - increases `strong` counter
    pub(crate) fn from_inner<'l>(inner: &'l ArcInner<T>) -> Self
    where Self: 'l {
        
        inner.strong.fetch_add(1, Ordering::Acquire);
        Self {
            ptr: NonNull::from(inner),
        }
    }

    /// Returns reference to the allocated data
    pub(crate) const fn inner(&self) -> &ArcInner<T> {
        unsafe { self.ptr.as_ref() }
    }

    /// Returns mutable reference to the allocated data
    pub(crate) const fn inner_mut(&mut self) -> &mut ArcInner<T> {
        unsafe { self.ptr.as_mut() }
    }


    /// Constructs an `Arc<T>` from a raw pointer.
    /// - the raw pointer must have been previously returned by a call to `Arc<T>::into_raw`
    pub const unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        Self {
            ptr: unsafe { ptr.cast::<usize>().offset(-2) }.cast()
        }
    }


    /// Consumes the `Arc`, returning the wrapped pointer
    /// - to avoid memory leak, the pointer must be converted back to an `Arc` using `Arc::from_raw`
    pub fn into_raw(self) -> NonNull<T> {
        let this = ManuallyDrop::new(self);
        unsafe {
            NonNull::new_unchecked(this.inner().data_as_mut_ptr())
        }
    }

    /// Increments the `strong` reference count on the `Arc<T>` associated with the provided pointer by one
    /// - the pointer must have been obtained through `Arc::into_raw`
    #[inline]
    pub unsafe fn increment_strong_count(ptr: NonNull<T>) {
        unsafe {
            let inner = (ptr.cast::<usize>().offset(-2)).cast::<ArcInner<T>>();
            inner.as_ref().strong.fetch_add(1, Ordering::Acquire);
        }
    }

    /// Decrements the `strong` reference count on the `Arc<T>` associated with the provided pointer by one
    /// - the pointer must have been obtained through `Arc::into_raw`
    #[inline]
    pub unsafe fn decrement_strong_count(ptr: NonNull<T>) {
        unsafe {
            let inner = (ptr.cast::<usize>().offset(-2)).cast::<ArcInner<T>>();
            inner.as_ref().strong.fetch_sub(1, Ordering::Acquire);
        }
    }



    /// Returns the `Weak` reference count
    #[inline]
    pub fn weak_count(&self) -> usize {
        self.inner().weak()
    }

    /// Returns the `Weak` reference count
    #[inline]
    pub fn strong_count(&self) -> usize {
        self.inner().strong()
    }

    /// Returns whether these two pointers has the same allocation
    #[inline]
    pub fn ptr_eq(&self, other: &Arc<T>) -> bool {
        self.ptr == other.ptr
    }

    /// Creates new `Weak` pointer to this allocation
    #[inline]
    pub fn downgrade(&self) -> Weak<T> {
        Weak::new_from_inner(self.inner())
    }
    
    
}

impl<T: Sized> Arc<MaybeUninit<T>> {

    /// Converts to `Arc<T>`
    pub unsafe fn assume_init(self) -> Arc<T> {
        let this = ManuallyDrop::new(self);
        Arc {
            ptr: this.ptr.cast(),
        }
    }

}




impl<T: Sized> AsRef<T> for Arc<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self.inner().get_ref()
    }
}

impl<T: Sized> Borrow<T> for Arc<T> {
    #[inline(always)]
    fn borrow(&self) -> &T {
        self.inner().get_ref()
    }
}

impl<T: Sized> Clone for Arc<T> {
    /// Makes a clone of the Arc pointer.
    /// - this creates another pointer to the same allocation, increasing the strong reference count.
    #[inline]
    fn clone(&self) -> Self {
        self.inner().strong.fetch_add(1, Ordering::Acquire);
        Self {
            ptr: self.ptr,
        }
    }
}

impl<T: Sized + Debug> Debug for Arc<T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.inner().data)
    }
}

impl<T: Sized + Default> Default for Arc<T> {
    #[inline]
    fn default() -> Self {
        Arc::new(T::default())
    }
}

impl<T: Sized> Deref for Arc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner().get_ref()
    }
}

impl<T: Sized + Display> Display for Arc<T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.inner().data)
    }
}

impl<T: Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        
        let inner = self.inner();
        let strong = inner.strong.fetch_sub(1, Ordering::Acquire);
        let weak = inner.weak.load(Ordering::Relaxed);

        crate::println!("\t\tarc: strong: {}, weak: {}", self.strong_count(), self.weak_count());
        if strong == 1 {
            //  this is the last holding reference => drop
            unsafe { drop_in_place(inner.data_as_mut_ptr()); }

            if weak == 1 {
                //  no other weak references => deallocate
                unsafe {
                    ALLOCATOR.dealloc(self.ptr.as_ptr() as *mut u8, ArcInner::<T>::layout());
                }
            }

        }
    }
}

impl<T: Sized> From<T> for Arc<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Sized + Clone> From<&T> for Arc<T> {
    fn from(value: &T) -> Self {
        Self::new(value.clone())
    }
}

impl<T: Sized + Hash> Hash for Arc<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.inner().data.hash(state);
    }
}

impl<T: Sized + Ord> Ord for Arc<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.inner().data.cmp(other)
    }
}

impl<T: Sized + PartialOrd> PartialOrd for Arc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.inner().data.partial_cmp(other)
    }
}

impl<T: Sized + PartialEq> PartialEq for Arc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner().data.eq(other)
    }
    fn ne(&self, other: &Self) -> bool {
        self.inner().data.ne(other)
    }
}

impl<T: Sized + Eq> Eq for Arc<T> {}

impl<T: Sized> Unpin for Arc<T> {}

impl<T: Sized> UnwindSafe for Arc<T> {}













/// `Weak` is a version of `Arc` that holds a non-owning reference to the managed allocation
/// - non-owning means that the allocation will exist as long as at least one `Weak` pointer exists
///   - however, `Weak` does not promise the value is initialized
///
/// The allocation is accessed by calling `upgrade` on the `Weak` pointer, which returns an `Option<Arc<T>>`
#[repr(transparent)]
pub struct Weak<T: Sized> {
    ptr: NonNull<ArcInner<T>>,
}

pub const fn dangling<T>() -> NonNull<T> {
    unsafe { NonNull::new_unchecked((usize::MAX) as *mut T) }
}

#[inline]
pub fn is_dangling<T>(ptr: NonNull<T>) -> bool {
    (ptr.as_ptr() as usize) == usize::MAX
}

impl<T: Sized> Weak<T> {

    /// Creates new `Weak` with no data
    /// - calling `upgrade()` on this will always return `None`
    pub const fn new() -> Self {
        Self {
            ptr: dangling(),
        }
    }

    /// Converts a raw pointer previously created by `into_raw` back into `Weak<T>`.
    pub const unsafe fn from_raw(ptr: *const T) -> Self {

        assert!(!ptr.is_null(), "pointer is NULL");

        unsafe {
            let ptr = (ptr as *const usize).offset(-2) as *mut ArcInner<T>;

            Self {
                ptr: NonNull::new_unchecked(ptr),
            }
        }
    }


    /// Destroys this `Weak` pointer and returns pointer to the allocated data
    /// - does not decrease the weak counter
    #[inline]
    pub fn into_raw(self) -> *const T {
        let this = ManuallyDrop::new(self);
        this.inner().data_as_ptr()
    }

    /// Returns pointer to the allocated data
    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        self.inner().data_as_ptr()
    }

    /// Attempts to upgrade `Weak` pointer to `Arc`
    pub fn upgrade(&self) -> Option<Arc<T>> {
        if is_dangling(self.ptr) {
            None
        } else {
            Some(Arc::from_inner(self.inner()))
        }
    }

    /// Returns strong reference count
    #[inline]
    pub fn strong_count(&self) -> usize {
        self.inner().strong()
    }

    /// Returns weak reference count
    #[inline]
    pub fn weak_count(&self) -> usize {
        self.inner().weak()
    }

    /// Returns whether these two pointers has the same allocation
    pub fn ptr_eq(&self, other: &Weak<T>) -> bool {
        self.ptr == other.ptr
    }


    /// Adds one `Weak` pointer to allocation of the arc
    pub(crate) fn new_from<'l>(arc: &'l Arc<T>) -> Self
    where Self: 'l {

        let inner = arc.inner();

        inner.weak.fetch_add(1, Ordering::Acquire);

        Self {
            ptr: arc.ptr,
        }
    }

    /// Constructs new `Weak` and makes it point to the allocation
    pub(crate) fn new_from_inner<'l>(inner: &'l ArcInner<T>) -> Self
    where Self: 'l {
        inner.weak.fetch_add(1, Ordering::Acquire);

        Self {
            ptr: NonNull::from(inner),
        }
    }

    pub(crate) const fn inner(&self) -> &ArcInner<T> {
        unsafe { self.ptr.as_ref() }
    }

}

impl<T: Sized> Clone for Weak<T> {
    fn clone(&self) -> Self {
        let inner = self.inner();
        inner.weak.fetch_add(1, Ordering::Acquire);
        Self { ptr: NonNull::from(inner) }
    }
}

impl<T: Sized + Debug> Debug for Weak<T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.inner().data)
    }
}

impl<T: Sized> Default for Weak<T> {
    /// Constructs new `Weak<T>` without allocating any memory
    /// - calling `upgrade` will return None
    #[inline(always)]
    fn default() -> Self {
        Self { ptr: NonNull::dangling() }
    }
}

impl<T: Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        
        let inner = self.inner();

        crate::println!("\t\tweak: strong: {}, weak: {}", self.strong_count(), self.weak_count());

        if inner.weak.fetch_sub(1, Ordering::Acquire) == 1 &&
            inner.strong.load(Ordering::Relaxed) == 1 {
            //  this is the last weak reference + no holding references
            //      => deallocate
            unsafe {
                ALLOCATOR.dealloc(self.inner().as_mut_ptr() as *mut u8, ArcInner::<T>::layout());
            }
        }

    }
}

impl<T: Sized> Deref for Weak<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner().data
    }
}


impl<T: Sized> Borrow<T> for Weak<T> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.inner().data
    }
}


