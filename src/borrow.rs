//	borrow.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build


//! Reimplements the usual `cow` bussiness from the `std` library
//! 
//! (moo)

pub use Cow::{Borrowed, Owned};
pub use core::borrow::{Borrow, BorrowMut};

use core::{ops::*, fmt, hash::{Hash, Hasher}, cmp::{Ord, Ordering}};

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
use crate::{String, Vec};


pub enum Cow<'a, T: ?Sized + 'a>
where
    T: ToOwned,
{
    /// Borrowed data.
    Borrowed(&'a T),

    /// Owned data.
    Owned(<T as ToOwned>::Owned),
}

impl<T: ?Sized + ToOwned> Clone for Cow<'_, T> {
    fn clone(&self) -> Self {
        match *self {
            Borrowed(b) => Borrowed(b),
            Owned(ref o) => {
                let b: &T = o.borrow();
                Owned(b.to_owned())
            }
        }
    }

    fn clone_from(&mut self, source: &Self) {
        match (self, source) {
            (&mut Owned(ref mut dest), &Owned(ref o)) => o.borrow().clone_into(dest),
            (t, s) => *t = s.clone(),
        }
    }
}

impl<T: ?Sized + ToOwned> Cow<'_, T> {
    /// Returns true if the data is borrowed, i.e. if `to_mut` would require additional work.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(cow_is_borrowed)]
    /// use std::borrow::Cow;
    ///
    /// let cow = Cow::Borrowed("moo");
    /// assert!(cow.is_borrowed());
    ///
    /// let bull: Cow<'_, str> = Cow::Owned("...moo?".to_string());
    /// assert!(!bull.is_borrowed());
    /// ```
    pub const fn is_borrowed(&self) -> bool {
        match *self {
            Borrowed(_) => true,
            Owned(_) => false,
        }
    }

    /// Returns true if the data is owned, i.e. if `to_mut` would be a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(cow_is_borrowed)]
    /// use std::borrow::Cow;
    ///
    /// let cow: Cow<'_, str> = Cow::Owned("moo".to_string());
    /// assert!(cow.is_owned());
    ///
    /// let bull = Cow::Borrowed("...moo?");
    /// assert!(!bull.is_owned());
    /// ```
    pub const fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }

    /// Acquires a mutable reference to the owned form of the data.
    ///
    /// Clones the data if it is not already owned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let mut cow = Cow::Borrowed("foo");
    /// cow.to_mut().make_ascii_uppercase();
    ///
    /// assert_eq!(
    ///   cow,
    ///   Cow::Owned(String::from("FOO")) as Cow<'_, str>
    /// );
    /// ```
    pub fn to_mut(&mut self) -> &mut <T as ToOwned>::Owned {
        match *self {
            Borrowed(borrowed) => {
                *self = Owned(borrowed.to_owned());
                match *self {
                    Borrowed(..) => unreachable!(),
                    Owned(ref mut owned) => owned,
                }
            }
            Owned(ref mut owned) => owned,
        }
    }

    /// Extracts the owned data.
    ///
    /// Clones the data if it is not already owned.
    ///
    /// # Examples
    ///
    /// Calling `into_owned` on a `Cow::Borrowed` returns a clone of the borrowed data:
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let s = "Hello world!";
    /// let cow = Cow::Borrowed(s);
    ///
    /// assert_eq!(
    ///   cow.into_owned(),
    ///   String::from(s)
    /// );
    /// ```
    ///
    /// Calling `into_owned` on a `Cow::Owned` returns the owned data. The data is moved out of the
    /// `Cow` without being cloned.
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let s = "Hello world!";
    /// let cow: Cow<'_, str> = Cow::Owned(String::from(s));
    ///
    /// assert_eq!(
    ///   cow.into_owned(),
    ///   String::from(s)
    /// );
    /// ```
    pub fn into_owned(self) -> <T as ToOwned>::Owned {
        match self {
            Borrowed(borrowed) => borrowed.to_owned(),
            Owned(owned) => owned,
        }
    }
}

impl<T> ToOwned for T
where T: Clone {
    type Owned = T;
    fn to_owned(&self) -> Self::Owned {
        self.clone()
    }
}




/// A generalization of `Clone` to borrowed data.
/// 
/// Some types make it possible to go from borrowed to owned, usually by implementing the `Clone` trait. But `Clone` works only for going from `&T` to `T`. The `ToOwned` trait generalizes `Clone` to construct owned data from any borrow of a given type.
pub trait ToOwned {
    type Owned: Borrow<Self>;

    // Required method
    fn to_owned(&self) -> Self::Owned;

    // Provided method
    fn clone_into(&self, target: &mut Self::Owned) {
        *target = self.to_owned();
    }
}


#[cfg(all(feature="string", feature="allocator", feature="spin"))]
impl ToOwned for str {
    type Owned = String;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        String::from(self.as_bytes())
    }

    #[inline]
    fn clone_into(&self, target: &mut crate::String) {
        target.clear();
        target.push_str(self);
    }
}


#[cfg(all(feature="string", feature="allocator", feature="spin"))]
impl<T: Clone> ToOwned for [T] {
    type Owned = Vec<T>;

    fn to_owned(&self) -> Vec<T> {
        let mut vec: Vec<T> = Vec::with_capacity(self.len());
        for i in self.iter() {
            unsafe { vec.push_within_capacity_unchecked(i.clone()) };
        }
        vec
    }
}


impl<B: ?Sized> Eq for Cow<'_, B> where B: Eq + ToOwned {}


impl<B: ?Sized + ToOwned> Deref for Cow<'_, B>
where
    B::Owned: Borrow<B>,
{
    type Target = B;

    fn deref(&self) -> &B {
        match *self {
            Borrowed(borrowed) => borrowed,
            Owned(ref owned) => owned.borrow(),
        }
    }
}

impl<B: ?Sized> Ord for Cow<'_, B>
where
    B: Ord + ToOwned,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<'a, 'b, B: ?Sized, C: ?Sized> PartialEq<Cow<'b, C>> for Cow<'a, B>
where
    B: PartialEq<C> + ToOwned,
    C: ToOwned,
{
    #[inline]
    fn eq(&self, other: &Cow<'b, C>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<'a, B: ?Sized> PartialOrd for Cow<'a, B>
where
    B: PartialOrd + ToOwned,
{
    #[inline]
    fn partial_cmp(&self, other: &Cow<'a, B>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<B: ?Sized> fmt::Debug for Cow<'_, B>
where
    B: fmt::Debug + ToOwned<Owned: fmt::Debug>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Borrowed(ref b) => fmt::Debug::fmt(b, f),
            Owned(ref o) => fmt::Debug::fmt(o, f),
        }
    }
}

impl<B: ?Sized> fmt::Display for Cow<'_, B>
where
    B: fmt::Display + ToOwned<Owned: fmt::Display>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Borrowed(ref b) => fmt::Display::fmt(b, f),
            Owned(ref o) => fmt::Display::fmt(o, f),
        }
    }
}

impl<B: ?Sized> Default for Cow<'_, B>
where
    B: ToOwned<Owned: Default>,
{
    /// Creates an owned Cow<'a, B> with the default value for the contained owned value.
    fn default() -> Self {
        Owned(<B as ToOwned>::Owned::default())
    }
}

impl<B: ?Sized> Hash for Cow<'_, B>
where
    B: Hash + ToOwned,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<T: ?Sized + ToOwned> AsRef<T> for Cow<'_, T> {
    fn as_ref(&self) -> &T {
        self
    }
}

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
impl<'a> Add<&'a str> for Cow<'a, str> {
    type Output = Cow<'a, str>;

    #[inline]
    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
impl<'a> Add<Cow<'a, str>> for Cow<'a, str> {
    type Output = Cow<'a, str>;

    #[inline]
    fn add(mut self, rhs: Cow<'a, str>) -> Self::Output {
        self += rhs;
        self
    }
}

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
impl<'a> AddAssign<&'a str> for Cow<'a, str> {
    fn add_assign(&mut self, rhs: &'a str) {
        if self.is_empty() {
            *self = Cow::Borrowed(rhs)
        } else if !rhs.is_empty() {
            if let Cow::Borrowed(lhs) = *self {
                let mut s = String::with_capacity(lhs.len() + rhs.len());
                s.push_str(lhs);
                *self = Cow::Owned(s);
            }
            self.to_mut().push_str(rhs);
        }
    }
}

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
impl<'a> AddAssign<Cow<'a, str>> for Cow<'a, str> {
    fn add_assign(&mut self, rhs: Cow<'a, str>) {
        if self.is_empty() {
            *self = rhs
        } else if !rhs.is_empty() {
            if let Cow::Borrowed(lhs) = *self {
                let mut s = String::with_capacity(lhs.len() + rhs.len());
                s.push_str(lhs);
                *self = Cow::Owned(s);
            }
            self.to_mut().push_str(&rhs);
        }
    }
}

impl<'l, T: Sized + ToOwned<Owned = T>> From<&'l T> for Cow<'l, T>
where T: From<&'l T> {
    #[inline]
    fn from(value: &'l T) -> Self {
        Self::Borrowed(value)
    }
}

impl<T: Sized + ToOwned<Owned = T>> From<T> for Cow<'_, T> {
    fn from(value: T) -> Self {
        Self::Owned(value)
    }
}

#[cfg(all(feature="string", feature="allocator", feature="spin"))]
impl<'a, T: Clone> From<&'a [T]> for Cow<'a, [T]> {
    fn from(s: &'a [T]) -> Cow<'a, [T]> {
        Cow::Borrowed(s)
    }
}

