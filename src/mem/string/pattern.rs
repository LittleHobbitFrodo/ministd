//  mem/string/pattern.rs (ministd crate)
//  this file originally belonged to baseOS project
//      an OS template on which to build

//  this module provides the string `Pattern` trait that is used to search for patterns in strings

use super::{searcher::{CharPredicateSearcher, CharSearcher, StrSearcher}, ReverseSearcher, Searcher};

pub trait Pattern: Sized {
    type Searcher<'l>: Searcher<'l, Self>;

    /// Constructs the associated searcher from `self` and the `haystack` to search in.
    //fn into_searcher(self, haystack: &str) -> Self::Searcher<'_>;

    /// Constructs the associated searcher for given haystack
    fn searcher<'l>(&self, haystack: &'l str) -> Self::Searcher<'l>;
    /// Checks whether the pattern matches anywhere in the `haystack`
    fn is_contained_in(&self, haystack: &str) -> bool;

    /// Checks whether the pattern matches at the front of the `haystack`
    fn is_prefix_of(&self, haystack: &str) -> bool;

    /// Checks whether the pattern matches at the back of the `haystack`
    fn is_suffix_of<'a>(&self, haystack: &'a str) -> bool
    where Self::Searcher<'a>: ReverseSearcher<'a, Self>;

}

impl Pattern for u8 {
    type Searcher<'l> = CharSearcher<'l>;

    fn searcher<'l>(&self, haystack: &'l str) -> Self::Searcher<'l> {
        CharSearcher::new(haystack, *self)
    }

    fn is_contained_in(&self, haystack: &str) -> bool {
        CharSearcher::new(haystack, *self as u8).next_match().is_some()
    }

    fn is_prefix_of(&self, haystack: &str) -> bool {
        haystack.as_bytes()[0] == *self as u8
    }

    fn is_suffix_of<'a>(&self, haystack: &'a str) -> bool
        where Self::Searcher<'a>: ReverseSearcher<'a, Self> {
        let last = match haystack.as_bytes().last() {
            Some(l) => l,
            None => return false,
        };
        *last == *self as u8
    }

}

impl<'n> Pattern for &'n str {
    type Searcher<'hay> = StrSearcher<'hay, 'n>;

    #[inline]
    fn searcher<'l>(&self, haystack: &'l str) -> Self::Searcher<'l> {
        Self::Searcher::new(haystack, self)
    }
    
    fn is_contained_in(&self, haystack: &str) -> bool {
        self.searcher(haystack).next_match().is_some()
    }

    fn is_prefix_of(&self, haystack: &str) -> bool {

        if self.len() <= haystack.len() {
            &haystack[..self.len()] == *self
        } else {
            false
        }
    }

    fn is_suffix_of<'a>(&self, haystack: &'a str) -> bool
        where Self::Searcher<'a>: ReverseSearcher<'a, Self> {
        
        if self.len() <= haystack.len() {
            &haystack[haystack.len() - self.len() ..] == *self
        } else {
            false
        }

    }

}

impl<F> Pattern for F
where F: FnMut(u8) -> bool, F: Clone {

    type Searcher<'hay> = CharPredicateSearcher<'hay, Self>;

    #[inline]
    fn searcher<'l>(&self, haystack: &'l str) -> Self::Searcher<'l> {
        Self::Searcher::new(haystack, self.clone())
    }

    fn is_contained_in(&self, haystack: &str) -> bool {
        Self::Searcher::new(haystack, self.clone()).next_match().is_some()
    }

    fn is_prefix_of(&self, haystack: &str) -> bool {
        match Self::Searcher::new(haystack, self.clone()).next_match() {
            Some((start, _)) => start == 0,
            None => false,
        }
    }

    fn is_suffix_of<'a>(&self, haystack: &'a str) -> bool
        where Self::Searcher<'a>: ReverseSearcher<'a, Self> {
        match Self::Searcher::new(haystack, self.clone()).next_match_back() {
            Some((_, end)) => end == haystack.len(),
            None => false,
        }
    }


}