//	sync/rosync.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build




/// # Read Only Sync
/// `ReadOnly<T>` allows you to store read-only data and access it wihout the need of using `unsafe` blocks
pub struct ReadOnly<T> {
    data: T,
}

unsafe impl<T> Sync for ReadOnly<T> {}
unsafe impl<T> Send for ReadOnly<T> {}

impl<T> ReadOnly<T> {
    pub const fn new(value: T) -> Self {
        Self { data: value }
    }

    #[inline(always)]
    pub fn as_ref(&self) -> &T {
        &self.data
    }

    #[inline(always)]
    pub fn borrow(&self) -> &T {
        &self.data
    }
}

impl<T> core::ops::Deref for ReadOnly<T> {
    type Target=T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}


impl<T> Drop for ReadOnly<T> {
    fn drop(&mut self) {
        if crate::mem::needs_drop::<T>() {
            //  drop T here
            unsafe {core::ptr::drop_in_place(&mut self.data as *mut T);}
        }
    }
}