//  mem/string/mod.rs (ministd crate)
//  this file originally belonged to baseOS project
//      an OS template on which to build

//! Provides `std::String`- like structure: the `ministd::String` will allow you to tweak its memory managemet!


pub mod pattern;
pub mod searcher;

pub use pattern::Pattern;
pub use searcher::{Searcher, ReverseSearcher, SearchStep};

use crate::mem::DynamicBuffer;
use crate::Cow;

#[cfg(all(feature="allocator", feature="spin", feature="spin", feature="string"))]
use crate::panic_fmt;
#[cfg(all(feature="allocator", feature="spin", feature="vector"))]
use crate::Vec;

use core::{fmt::{Debug, Display, Write}, mem::ManuallyDrop, ops::{Deref, DerefMut, Index, IndexMut, RangeBounds}, ptr::{self, copy, copy_nonoverlapping, null_mut, NonNull}, slice, str::Utf8Error};
use crate::convert::{strify, strify_mut};
use core::alloc::Layout;

use core::ops::Bound::*;

/// Indicates alignment of the data
const ALIGN: usize = 4;


/// A ASCII–encoded, growable string.
/// - This implementation will also allow you to tweak memory management using generic parameter
/// 
/// **note**: Implementation of the `Drop` trait is not needed for the memory is deallocated by `DynamicBuffer::drop()` automatically
///  - Data in this implementation of `String` are aligned to `align_of::<u32>()` for faster copying an searching
///    - This is the minimal required align of the string
/// 
/// # Memory layout
/// - The `ministd::String` has the same memory layout as `ministd::DynamicBuffer`
/// 
/// # Generic parameter
/// `STEP` tells the structure how many characters has to be preallocated
/// - Has to be either `0` (for geometrical growth) or multiple of 4
#[repr(transparent)]
pub struct String<const STEP: usize = 0> {
    data: DynamicBuffer<u8, STEP, ALIGN>,
}

impl<const STEP: usize> String<STEP> {

    const VALID: bool = STEP == 0 || STEP.is_multiple_of(4);

    /// Describes memory layout for some capacity
    pub const fn layout_for(capacity: usize) -> Layout {
        DynamicBuffer::<u8, STEP, ALIGN>::layout_for(capacity)
    }

    /// Describes memory layout for some capacity without aligning to STEP
    pub const fn layout_for_exact(capacity: usize) -> Layout {
        DynamicBuffer::<u8, STEP, ALIGN>::layout_for_exact(capacity)
    }

    /// Expands the `capacity` of the vector by `STEP`
    /// - this function always reallocates memory
    /// - **panics** if allocation fails
    #[inline(always)]
    pub fn expand(&mut self) {
        self.data.expand();
    }

    /// Tries to expand the `capacity` of the vector by `STEP`
    /// - this function always reallocates memory
    /// - returns `Err` if allocation fails
    #[inline(always)]
    pub fn try_expand(&mut self) -> Result<(), ()> {
        self.data.try_expand()
    }


    /// Creates a new empty `String`
    /// - no data is allocated
    pub const fn new() -> Self {
        if Self::VALID {
            Self { data: DynamicBuffer::empty() }
        } else {
            panic!("STEP has to be either `0` or multiple of 4");
        }
    }


    /// Creates new `String` with at least the specified capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if Self::VALID {
            Self { data: DynamicBuffer::with_capacity(capacity) }
        } else {
            panic!("STEP has to be either `0` or multiple of 4");
        }
    }

    /// Tries to create new `String` with at least the specified capacity
    /// - returns `Err` if allocation fails
    #[inline]
    pub fn try_with_capacity(capacity: usize) -> Result<Self, ()> {
        if Self::VALID {
            Ok(Self {
                data: DynamicBuffer::try_with_capacity(capacity)?
            })
        } else {
            panic!("STEP has to be either `0` or multiple of 4");
        }
    }

    /// Converts a vector of bytes to a String
    /// - returns `Err(None)` on allocation failure
    #[cfg(all(feature="allocator", feature="vector"))]
    pub fn from_utf8<const VSTEP: usize>(vec: Vec<u8, VSTEP>) -> Result<String<STEP>, Option<Utf8Error>> {

        if Self::VALID {
            let s = core::str::from_utf8(unsafe { vec.as_slice_unchecked() })?;

            let mut db = DynamicBuffer::<u8, STEP, ALIGN>::try_with_capacity(s.len()).map_err(|_| None)?;
            db.size = vec.len() as u32;

            unsafe {
                copy_nonoverlapping(s.as_ptr(), db.as_ptr(), s.len());
            }

            Ok(String { data: db })
        } else {
            panic!("STEP has to be either `0` or multiple of 4");
        }
    }

    //  TODO: add `from_utf8_lossy`


    /// Converts a `Vec<u8>` to a `String`, substituting invalid UTF-8 sequences with replacement characters.
    /// Note that this function does not guarantee reuse of the original Vec allocation.
    #[cfg(all(feature="allocator", feature="vector"))]
    pub fn from_utf8_lossy_owned<const VSTEP: usize>(v: Vec<u8, STEP>) -> String<VSTEP> {

        if Self::VALID {
            let v = ManuallyDrop::new(v);

            let s = unsafe { core::str::from_utf8_unchecked(v.as_slice().expect("vector is empty")) };

            let mut db = DynamicBuffer::<u8, VSTEP, ALIGN>::with_capacity(s.len());
            db.size = s.len() as u32;

            unsafe {
                copy_nonoverlapping(s.as_ptr(), db.as_ptr(), s.len());
            }

            String { data: db }
        } else {
            panic!("STEP has to be either `0` or multiple of 4");
        }

    }




    /// Appends a given string slice onto the end of this `String`
    /// - **panics** if allocation fails
    pub fn push_str(&mut self, string: &str) {
        
        self.reserve(string.len());

        unsafe {
            let ptr = self.data.as_ptr().add(self.len());

            ptr::copy_nonoverlapping(string.as_ptr(), ptr, string.len());
        }
        self.data.size += string.len() as u32;
    }

    /// Appends a given string slice onto the end of this `String` without checking bounds
    /// - **safety** - misuse may cause buffer overflow and/or undefined behaviour
    ///   - use only if you are 100% sure overflow will not happen
    pub unsafe fn push_str_unchecked(&mut self, string: &str) {
        unsafe {
            let ptr = self.data.as_ptr().add(self.len());

            ptr::copy_nonoverlapping(string.as_ptr(), ptr, string.len());
        }
        self.data.size += string.len() as u32;
    }

    /// Tries to append a given string slice onto the end of this `String`
    /// - returns `Err` if allocation fails
    pub fn try_push_str(&mut self, string: &str) -> Result<(), ()> {

        self.try_reserve(string.len())?;

        unsafe {
            let ptr = self.data.as_ptr().add(self.len());

            ptr::copy_nonoverlapping(string.as_ptr(), ptr, string.len());
        }
        self.data.size += string.len() as u32;

        Ok(())
    }

    /// Copies elements from src range to the end of the `String`
    pub fn extend_from_within<R>(&mut self, src: R)
        where R: RangeBounds<usize> {
        
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
            copy(self.as_ptr().add(start), self.as_mut_ptr().add(self.len()), len);
        }

        self.data.size += len as u32;

    }

    /// Appends the given character to the end of the `String`
    /// - **panics** if allocation fails
    pub fn push(&mut self, c: u8) {
        if self.len() == self.capacity() {
            self.expand();
        }

        unsafe {

            self.data.as_ptr().add(self.len()).write(c);
        }
        self.data.size += 1;
    }

    /// Pushes character withouch checking bounds
    /// - **safety** - misuse may cause buffer overflow and/or undefined behavoiur
    ///   - use only if you are 100% sure overflow will not happen
    #[inline]
    pub unsafe fn push_unchecked(&mut self, c: u8) {
        unsafe {
            self.data.as_ptr().add(self.len()).write(c);
        }
        self.data.size += 1;
    }

    /// Tries to push the given character to the end of the `String`
    /// - returns `Err` if allocation fails
    pub fn try_push(&mut self, c: u8) -> Result<(), ()> {
        if self.len() == self.capacity() {
            self.try_expand()?;
        }

        unsafe {
            self.data.as_ptr().add(self.len()).write(c);
        }

        self.data.size += 1;

        Ok(())

    }

    /// Removes the last character from the `String`
    /// - does not affect `capacity`
    #[inline]
    pub fn pop_noret(&mut self) {
        self.data.size = self.data.size.saturating_sub(1);
    }

    /// Removes the last character from the `String` and returns it
    /// - does not affect `capacity`
    #[inline]
    pub fn pop(&mut self) -> Option<u8> {
        if self.len() > 0 {
            self.data.size -= 1;
            Some(unsafe { self.data.as_ptr().add(self.len()).read() })
        } else {
            None
        }
    }

    /// Removes last `n` characters from the string
    /// - does not affect `capacity`
    #[inline(always)]
    pub fn pop_n(&mut self, n: usize) {
        self.data.size = self.data.size.saturating_sub(n as u32);
    }


    /// Reserves capacity for at least `add` more characters
    /// - **panics** if allocation fails
    /// - `capacity` will be greater than or equal to `self.len() + add`
    ///   - `capacity` is aligned to `STEP`
    #[inline(always)]
    pub fn reserve(&mut self, add: usize) {
        self.data.resize(self.len() + add);
    }

    /// Tries to reserve capacity for at least `add` more characters
    /// - returns `Err` if allocation fails
    /// - capacity will be greater than or equal to `self.len() + add.len()`
    #[inline(always)]
    pub fn try_reserve(&mut self, add: usize) -> Result<(), ()> {
        self.data.try_resize(self.len() + add)
    }

    /// Reserves capacity for at least `add` more characters
    /// - **panics** if allocation fails
    /// - `capcity` will be greater than or equal to `self.len() + add`
    ///   - `capacity` is not aligned
    #[inline(always)]
    pub fn reserve_exact(&mut self, add: usize) {
        self.data.resize_exact(self.len() + add);
    }

    /// Reserves capacity for at least `add` more characters
    /// - **panics** if allocation fails
    /// - `capacity` will be greater than or equal to `self.len() + add`
    ///   - `capacity` is not aligned
    pub fn try_reserve_exact(&mut self, add: usize) -> Result<(), ()> {
        self.data.try_resize_exact(self.len() + add)
    }

    /// Shortens this `String` to the specified length.
    /// If new_len is greater than or equal to the string’s current length, this has no effect
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        if self.len() > len {
            self.data.size = len as u32;
        }
    }

    /// Shortens this `String` to the specified length without checking the length of the `String`
    /// - please use only if you are sure that `self.len() > len`
    #[inline]
    pub unsafe fn truncate_unchecked(&mut self, len: usize) {
        self.data.size = len as u32;
    }

    /// Shrinks the `capacity` of this `String` to match its length
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.data.resize(self.len());
    }

    /// Shrinks the `capacity` of this `String` to the specified value
    /// The `capacity` will remain at least as large as both the length and the supplied value
    pub fn shrink_to(&mut self, len: usize) {
        self.data.resize(core::cmp::max(self.len(), len));
    }
    
    /// Removes character at the `index` position
    /// - **panics** if index is out of bounds
    /// - this os `O(n)` operation
    /// - **no-op** if empty
    pub fn remove(&mut self, index: usize) {

        if self.len() > 0 {
            if index > self.len() {
                #[cfg(all(feature="allocator", feature="spin", feature="string"))]
                panic_fmt!("index {index} is out of bounds 0..{}", self.len());
                #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
                panic!("index is out of bounds");
            }

            self.data.size -= 1;

            unsafe {
                let start = self.data.as_ptr().add(index);
                ptr::copy(start.add(1), start, self.len() - index);
            }

        }
    }

    /// Retains only the characters specified by the predicate
    /// 
    /// In other words, remove all characters `c` such that `f(c)` returns `false`. This method operates in place, visiting each character exactly once in the original order, and preserves the order of the retained characters
    /// - this is an `O(n)` operation
    pub fn retain<F>(&mut self, f: F)
    where F: Fn(u8) -> bool {

        if self.len() == 0 {
            return
        }

        let mut ptr = self.data.data();
        let mut i = 0;

        loop {

            if !f(unsafe { ptr.read() }) {
                self.remove(i);
            }

            if i >= self.len() {
                return
            }
            
            ptr = unsafe { ptr.add(1) };
            i += 1;
        }

    }


    /// Inserts character at the `index` position
    /// - **panics** if allocation fails
    /// - pushes the character if `index >= self.len()`
    /// - this is `O(n)` operation
    pub fn insert(&mut self, index: usize, c: u8) {

        let len = self.len();

        if index >= len {
            self.push(c);
            return;
        }

        if len == self.capacity() {
            self.expand();
        }

        unsafe {
            let ptr = self.data.as_ptr().add(index);

            ptr::copy(ptr, ptr.add(1), len - index);

            ptr.write(c)
        }

        self.data.size += 1;

    }

    /// Tries to insert character at the `index` position
    /// - returns `Err` if allocation fails
    /// - this is `O(n)` operation
    /// - pushes the character if `index >= self.len()`
    pub fn try_insert(&mut self, index: usize, c: u8) -> Result<(), ()> {

        let len = self.len();

        if index >= len {
            return self.try_push(c);
        }

        if len == self.capacity() {
            self.try_expand()?;
        }

        unsafe {
            let ptr = self.data.as_ptr().add(index);

            ptr::copy(ptr, ptr.add(1), len - index);

            ptr.write(c)
        }

        self.data.size += 1;
        Ok(())
    } 

    /// Inserts string slice at the `index` position
    /// - **panics** if allocation fails
    /// - this is `O(n)` operation
    pub fn insert_str(&mut self, index: usize, string: &str) {

        let len = self.len();

        if index >= len {
            self.push_str(string);
            return;
        }

        self.reserve(string.len());

        unsafe {
            let ptr = self.data.as_ptr().add(index);

            ptr::copy(ptr, ptr.add(string.len()), len - index);

            ptr::copy_nonoverlapping(string.as_ptr(), ptr, string.len());
        }

        self.data.size += string.len() as u32;

    }

    /// Returns a mutable reference to the contents of the string.
    /// - **warning** this function is not tested enough yet and may result in undefined behaviour
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<u8, ALIGN> {
        unsafe { ((self as *mut Self) as *mut Vec<u8, ALIGN>).as_mut().unwrap_unchecked() }
    }

    /// Tries to insert string at the `index` position
    /// - returns `Err` if allocation fails
    /// - this is `O(n)` operation
    /// - pushes the character if `index >= self.len()`
    pub fn try_insert_str(&mut self, index: usize, string: &str) -> Result<(), ()> {

        let len = self.len();

        if index >= len {
            return self.try_push_str(string);
        }

        self.try_reserve(string.len())?;

        unsafe {
            let ptr = self.data.as_ptr().add(index);

            ptr::copy(ptr, ptr.add(string.len()), len - index);

            ptr::copy_nonoverlapping(string.as_ptr(), ptr, string.len());
        }

        self.data.size += string.len() as u32;

        Ok(())

    }

    /// Splits the string into two at the given byte index
    /// 
    /// Returns a newly allocated `String`. self contains bytes `[0, at)`, and the returned `String` contains bytes `[at, len)`
    /// 
    /// Note that the `capacity` of `self` does not change
    /// 
    /// **panics** if `at` is out of bounds or allocation fails
    pub fn split_off(&mut self, at: usize) -> String<STEP> {
        if at >= self.len() {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("index {at} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("index is out of bounds");
            
        }

        let len = self.len() - at;

        let mut new: String<STEP> = String::with_capacity(len);

        unsafe {
            new.set_len(len);

            copy_nonoverlapping(self.as_ptr().add(at), new.as_mut_ptr(), len);

        }

        self.data.size = at as u32;

        new

    }

    /// Splits the string into two at the given byte index
    /// 
    /// Returns a newly allocated `String`. self contains bytes `[0, at)`, and the returned `String` contains bytes `[at, len)`
    /// 
    /// Note that the `capacity` of `self` does not change
    /// 
    /// **panics** if `at` is out of bounds
    /// - returns `Err` if allocation fails
    pub fn try_split_off(&mut self, at: usize) -> Result<String<STEP>, ()> {
        if at >= self.len() {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("index {at} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("index is out of bounds");
        }

        let len = self.len() - at;

        let mut new: String<STEP> = String::try_with_capacity(len)?;

        unsafe {
            new.set_len(len);

            copy_nonoverlapping(self.as_ptr().add(at), new.as_mut_ptr(), len);

        }

        self.data.size = at as u32;

        Ok(new)
    }

    /// Removes the specified range in the string, and replaces it with the given string. The given string doesn’t need to be the same length as the range
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where R: RangeBounds<usize> {

        let (start, end) = self.handle_bounds(&range);

        if start > self.len() || end > self.len() {
            #[cfg(all(feature="allocator", feature="spin", feature="string"))]
            panic_fmt!("slice {start}..{end} is out of bounds 0..{}", self.len());
            #[cfg(not(all(feature="allocator", feature="spin", feature="string")))]
            panic!("index is out of bounds");
        }

        let len = end - start;

        unsafe {
            let mut ptr = self.data.data().add(start);
            let bytes = replace_with.as_bytes();
            for i in 0..len {

                ptr.write(bytes[i % replace_with.len()]);

                ptr = ptr.add(1);
            }
        }
    }

    /// Returns the byte index of the first character of this string slice that matches the pattern.
    /// - `None` if the pattern doesn’t match
    ///     
    /// The `pattern` can be a `&str`, `char` (`u8`), a slice of chars, or a function or closure that determines if a character matches
    pub fn find<P>(&self, pattern: P) -> Option<usize>
        where P: Pattern {

        P::Searcher::new(self.as_str(), pattern)
            .next_match().map(|(start, _)| start)
    }

    

    /// Forces `length` of this vector to the specified value without cheking `capacity`
    pub unsafe fn set_len(&mut self, len: usize) {
        self.data.size = len as u32;
    }

    /// Removes all characters from the `String`
    /// - does not affect `capacity`
    pub const fn clear(&mut self) {
        self.data.size = 0;
    }


    /// Removes substring from the `String`
    /// - **panics** if allocation fails or out of bounds or if empty
    /// - this is an `O(n)` operation
    pub fn remove_str<R>(&mut self, range: R)
    where R: RangeBounds<usize> {

        let len = self.len();

        let (start, end) = self.handle_bounds(&range);

        if start > len || end > len {
            if len == 0 {
                panic!("String is empty");
            } else {
                panic!("slice out of bounds");
            }
        }

        let count = end - start;

        unsafe {
            let ptr = self.data.as_ptr().add(start);

            ptr::copy(ptr.add(count), ptr, len - start);
        }

        self.data.size -= count as u32;

    }

    /// Consumes and leaks the String, returning a mutable reference to the contents, &'a mut str
    /// - the caller can freely choose lifetime of the reference
    /// - dropping the reference will cause memory leak
    pub fn leak<'l>(self) -> &'l mut str {
        let m = ManuallyDrop::new(self);
        if m.is_empty() {
            panic!("String has no data");
        }

        unsafe { strify_mut(slice::from_raw_parts_mut(m.data.as_ptr(), m.len())) }
    }

}




impl<const STEP: usize> String<STEP> {

    /// Returns the number of ASCII characters (bytes) of the string
    pub const fn len(&self) -> usize { self.data.size as usize }

    /// Returns the constant generic `STEP` of this instance
    pub const fn step(&self) -> usize { STEP }

    /// Returns the `String`s capacity in bytes
    pub const fn capacity(&self) -> usize { self.data.capacity() }


    /// Returns a byte slice of this `String`’s contents
    /// - **panics** if empty
    pub const fn as_bytes(&self) -> &[u8] {
        if self.len() > 0 {
            unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len()) }
        } else {
            panic!("String is empty");
        }
    }

    /// Returns a byte slice of this `String`’s content without checking for NULL
    /// - **safety** - even if the `String` does not contain any data, the pointer is valid
    ///   - misuse may cause undefined behavoiur
    ///   - use only if you are 100% sure that the string contains value
    pub const unsafe fn as_bytes_unchecked(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len()) }
    }

    /// Returns a byte slice of this `String`’s content
    /// - returns `None` if empty
    pub const fn as_bytes_checked(&self) -> Option<&[u8]> {
        if self.len() > 0 {
            Some(unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len()) })
        } else {
            None
        }
    }

    /// Returns a mutable byte slice of this `String`’s contents
    /// - **panics** if empty
    pub const fn as_bytes_mut(&mut self) -> &mut [u8] {
        if self.len() > 0 {
            unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.len()) }
        } else {
            panic!("String is empty");
        }
    }

    /// Returns content of the `String` as slice or `None` if the `String` is empty
    pub fn as_slice(&self) -> Option<&[u8]> {
        if self.len() > 0 {
            Some(unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) })
        } else {
            None
        }
    }

    /// Returns content of the `String` as slice without checking if the `String` is empty
    /// - please use only if you are sure that the `String` is not empty
    #[inline]
    pub unsafe fn as_slice_unchecked(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Returns content of the `String` as mutable slice or `None` if the `String` is empty
    pub fn as_mut_slice(&mut self) -> Option<&mut [u8]> {
        if self.len() > 0 {
            Some(unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) })
        } else {
            None
        }
    }

    /// Returns content of the `String` as slice without checking if the `String` is empty
    /// - please use only if you are ure that the `String` is not empty
    pub fn as_mut_slice_unchecked(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }


    /// Extracts a string slice containing the entire `String`
    /// - **panics** if empty
    pub const fn as_str(&self) -> &str {
        if self.data.has_data() {
            strify(unsafe { slice::from_raw_parts(self.data.as_ptr(), self.data.size as usize) })
        } else {
            panic!("String is empty");
        }
    }

    /// Extracts a string slice containing the entire `String` without checking for NULL
    /// - **safety** - even if the `String` does not contain any data, the pointer is valid
    ///   - misuse may cause undefined behavoiur
    ///   - use only if you are 100% sure that the string contains value
    pub const unsafe fn as_str_unchecked(&self) -> &str {
        strify(unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len()) })
    }

    /// Extracts a string slice containing the entire `String`
    /// - returns `None` if empty
    pub const fn as_str_checked(&self) -> Option<&str> {
        if self.data.has_data() {
            Some(strify(unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len()) }))
        } else {
            None
        }
    }

    /// Converts a String into a mutable string slice
    /// - **panics** if empty
    pub const fn as_mut_str(&mut self) -> &mut str {
        if self.data.has_data() {
            strify_mut(unsafe { slice::from_raw_parts_mut(self.data.data().as_ptr(), self.data.size as usize) })
        } else {
            panic!("String is empty");
        }
    }

    /// Converts a String into a mutable string slice without checking for NULL
    /// - **safety** - even if the `String` does not contain any data, the pointer is valid
    ///   - misuse may cause undefined behavoiur
    ///   - use only if you are 100% sure that the string contains value
    pub const unsafe fn as_mut_str_unchecked(&self) -> &mut str {
        strify_mut(unsafe {  slice::from_raw_parts_mut(self.data.as_ptr(), self.len())})
    }

    /// Converts a String into a mutable string
    /// - returns `None` if the `String` has no data
    pub const fn as_mut_str_checked(&self) -> Option<&mut str> {
        if self.data.has_data() {
            Some(strify_mut(unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.len()) }))
        } else {
            None
        }
    }


    /// Checks whether the `String` has length of zero
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }


    /// Decomposes a String into its raw components: `(pointer, length, capacity)`
    /// 
    /// After calling this function, the caller is responsible for the memory previously managed by the `String`
    /// - you can deallocate the memory with `String::layout_for_exact(capacity)` used as layout
    /// - or reconstruct the string with `from_raw_parts`
    pub unsafe fn into_raw_parts(self) -> (*mut u8, usize, usize) {
        let mut m = ManuallyDrop::new(self);
        if m.len() > 0 {
            (m.as_mut_ptr(), m.len(), m.capacity())
        } else {
            (null_mut(), m.len(), m.capacity())
        }
    }

    /// Creates a new String from a pointer, a length and a capacity
    /// 
    /// Safety:
    /// - data must be obtained (and not modified) from `String::into_raw_parts`
    /// - or allocated with `String::layout_for_exact(capacity)`
    pub const unsafe fn from_raw_parts(ptr: *mut u8, len: usize, capacity: usize) -> Self {
        Self {
            data: DynamicBuffer::from_raw(NonNull::new(ptr).expect("pointer is null"), capacity as u32, len as u32)
        }
    }

    /// Converts String into `Vec<u8>`
    #[cfg(all(feature="allocator", feature="vector"))]
    pub fn into_bytes(self) -> Vec<u8, STEP> {
        let (data, size, capacity) = unsafe { self.data.into_parts() };
        unsafe { Vec::from_parts(data, size, capacity) }
    }

    /// Checks `RangeBounds` for this vector
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

    #[inline(always)]
    pub fn iter<'l>(&'l self) -> core::slice::Iter<'l, u8> {
        self.as_slice().expect("String is empty").into_iter()
    }

    pub fn iter_mut<'l>(&'l mut self) -> core::slice::IterMut<'l, u8> {
        self.as_mut_slice().expect("String is empty").into_iter()
    }


}


impl<const STEP: usize> Display for String<STEP> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<const STEP: usize> Debug for String<STEP> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "size: {}, capacity: {}, conatent: {self}", self.len(), self.capacity())
    }
}


impl<const STEP: usize> PartialEq<str> for String<STEP> {

    #[inline]
    fn eq(&self, other: &str) -> bool {
        if let Some(s) = self.as_str_checked() {
            s.eq(other)
        } else {
            false
        }
    }

    #[inline]
    fn ne(&self, other: &str) -> bool {
        if let Some(s) = self.as_str_checked() {
            s.ne(other)
        } else {
            true
        }
    }
}

impl<const STEP: usize> PartialEq<&str> for String<STEP> {

    #[inline]
    fn eq(&self, other: &&str) -> bool {
        if let Some(s) = self.as_str_checked() {
            s.eq(*other)
        } else {
            false
        }
    }

    #[inline]
    fn ne(&self, other: &&str) -> bool {
        if let Some(s) = self.as_str_checked() {
            s.ne(*other)
        } else {
            true
        }
    }
}

impl<const STEP: usize> PartialOrd<&str> for String<STEP> {
    fn ge(&self, other: &&str) -> bool {
        let s = if let Some(s) = self.as_str_checked() {
            s
        } else {
            return true;
        };

        s.ge(other)

    }

    fn le(&self, other: &&str) -> bool {
        let s = if let Some(s) = self.as_str_checked() {
            s
        } else {
            return other.len() == 0;
        };

        s.le(other)

    }

    fn lt(&self, other: &&str) -> bool {
        let s = if let Some(s) = self.as_str_checked() {
            s
        } else {
            return false;
        };

        s.lt(other)

    }

    fn gt(&self, other: &&str) -> bool {
        let s = if let Some(s) = self.as_str_checked() {
            s
        } else {
            return other.len().gt(&0);
        };

        s.gt(other)

    }

    fn partial_cmp(&self, other: &&str) -> Option<core::cmp::Ordering> {
        let s = if let Some(s) = self.as_str_checked() {
            s
        } else {
            return other.len().partial_cmp(&0)
        };

        s.partial_cmp(other)
    }

}


impl<const STEP: usize> PartialEq<String> for String<STEP> {

    fn eq(&self, other: &String) -> bool {
        let s1 = if let Some(s) = self.as_str_checked() {
            s
        } else {
            return if other.len() == 0 {
                true
            } else {
                false
            }
        };

        let s2 = if let Some(s) = other.as_str_checked() {
            s
        } else {
            return false;
        };

        s1.eq(s2)
    }

    fn ne(&self, other: &String) -> bool {
        let s1 = if let Some(s) = self.as_str_checked() {
            s
        } else {
            return if other.len() == 0 {
                false
            } else {
                true
            }
        };

        let s2 = if let Some(s) = other.as_str_checked() {
            s
        } else {
            return true;
        };

        s1.ne(s2)
    }
}

impl<const STEP: usize> Clone for String<STEP> {
    fn clone(&self) -> Self {

        let data = self.data.clone();

        unsafe {
            copy_nonoverlapping(self.data.as_ptr(), data.as_ptr(), self.len());
        }


        Self { data }
    }
}

impl<const STEP: usize> From<&str> for String<STEP> {
    fn from(value: &str) -> Self {
        
        let mut data = DynamicBuffer::with_capacity(value.len());

        unsafe {
            ptr::copy_nonoverlapping(value.as_ptr(), data.as_ptr(), value.len());
        }

        data.size = value.len() as u32;

        Self { data }

    }
}

impl<const STEP: usize> From<&[u8]> for String<STEP> {
    fn from(value: &[u8]) -> Self {
        let mut data = DynamicBuffer::with_capacity(value.len());
        unsafe {
            ptr::copy_nonoverlapping(value.as_ptr(), data.as_ptr(), value.len());
        }

        data.size = value.len() as u32;

        Self { data }
    }
}

impl<const STEP: usize> Deref for String<STEP> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        if let Some(s) = self.as_str_checked() {
            s
        } else {
            panic!("String is empty");
        }
    }
}

impl<const STEP: usize> DerefMut for String<STEP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(s) = self.as_mut_str_checked() {
            s
        } else {
            panic!("String is empty");
        }
    }
}

impl<const STEP: usize> Index<usize> for String<STEP> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        if self.len() > 0 && index < self.len(){
            unsafe {
                self.data.as_ptr().add(index).as_ref().unwrap_unchecked()
            }
        } else {
            if self.is_empty() {
                panic!("String is empty");
            } else {
                panic!("index out of bounds");
            }
        }
    }
}

impl<const STEP: usize> IndexMut<usize> for String<STEP> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if self.len() > 0 && index < self.len(){
            unsafe {
                self.data.as_ptr().add(index).as_mut().unwrap_unchecked()
            }
        } else {
            if self.is_empty() {
                panic!("String is empty");
            } else {
                panic!("index out of bounds");
            }
        }
    }
}

impl<const STEP: usize> Default for String<STEP> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}


impl<const STEP: usize> Write for String<STEP> {
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.try_push(c as u8).map_err(|_| core::fmt::Error)
    }
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.try_push_str(s).map_err(|_| core::fmt::Error)
    }
}


impl<'l, const STEP: usize> PartialEq<String<STEP>> for &'l str {
    #[inline(always)]
    fn eq(&self, other: &String<STEP>) -> bool {
        other == self
    }
    #[inline(always)]
    fn ne(&self, other: &String<STEP>) -> bool {
        other != self
    }
}



impl crate::Borrow<str> for String {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl crate::BorrowMut<str> for String {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}


impl<'l> From<&'l String> for Cow<'l, str> {
    fn from(value: &'l String) -> Self {
        Cow::Borrowed(value.as_str())
    }
}

impl<'l, const STEP: usize> From<Cow<'l, str>> for String<STEP> {
    /// Converts `Cow<str>` into String
    /// - reallocates the buffer
    fn from(value: Cow<'l, str>) -> Self {
        let mut s: String<STEP> = String::with_capacity(value.len());
        unsafe {
            s.set_len(value.len());
            let ptr = value.as_ref().as_ptr();
            copy_nonoverlapping(ptr, s.as_mut_ptr(), value.len());
        }
        s
    }
}


impl<'l> From<&'l String> for Cow<'l, String> {
    #[inline]
    fn from(value: &'l String) -> Self {
        Cow::Borrowed(value)
    }
}




pub trait ToString {
    fn to_string(&self) -> String;
}


impl<T: Display> ToString for T {
    fn to_string(&self) -> String {
        let mut s: String = String::new();
        let _ = write!(&mut s, "{self}");
        s
    }
}