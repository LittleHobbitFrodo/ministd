//  mem/string/searcher/mod.rs (ministd crate)
//  this file originally belonged to baseOS project
//      an OS template on which to build

mod searchers;
pub use searchers::{CharSearcher, StrSearcher, CharPredicateSearcher};

use super::Pattern;

/// Indicates the status of last searching operation
#[derive(Clone, PartialEq)]
pub enum SearchStep {
    /// Found match in `haystack[self.0..self.1]`
    Match(usize, usize),
    /// Found match in `haystack[self.0..self.1]` and encountered the end of the string
    /// - treat it as `Done` with data
    LastMatch(usize, usize),
    /// Found no matchin `haystack[self.0..self.1]`
    Reject(usize, usize),
    /// Found no match in `haystack[self.0..self.1]` and encountered the end of the string
    /// - treat it as `Done` with data
    LastReject(usize, usize),
    /// Encountered the end of the string
    Done
}

impl core::fmt::Display for SearchStep {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Match(start, end) => {
                write!(f, "match: {start}..{end}")
            },
            Self::Reject(start, end) => {
                write!(f, "reject: {start}..{end}")
            },
            Self::LastMatch(start, end) => {
                write!(f, "last match: {start}..{end}")
            },
            Self::LastReject(start, end) => {
                write!(f, "last reject: {start}..{end}")
            },
            Self::Done => {
                write!(f, "done")
            }
        }
    }
}

pub trait Searcher<'haystack, P: Sized + Pattern> {

    type Needle;

    /// Type of what are you searching for

    fn new(haystack: &'haystack str, needle: P) -> Self;

    /// Returns the string to be searching in
    /// - its always the same string
    fn haystack(&self) -> &str;

    /// Performs the next search step
    /// - returns `Match(a, b)` if `haystack[a..b]` matches the pattern
    /// - returns `Reject(a, b)` if `haystack[a..b]` cannot match the pattern (even partially)
    /// - returns `Done` if every byte of the haystack has been visited
    /// 
    /// The stream of `Match` and `Reject` values up to a `Done` will contain index ranges that are adjacent, non-overlapping and covering the whole haystack
    /// 
    /// A `Match` result needs to contain the whole matched pattern, however `Reject` results may be split up into arbitrary many adjacent fragments. Both ranges may have zero length.
    fn next(&mut self) -> SearchStep;

    /// Finds the next `Match` result. See `next()`
    /// 
    /// Unlike `next()`, there is no guarantee that the returned ranges of this and `next_reject` will overlap. This will return `(start_match, end_match)`, where `start_match` is the index of where the match begins, and `end_match` is the index after the end of the match.
    fn next_match(&mut self) -> Option<(usize, usize)>;


    /// Finds the next `Reject` result. See `next()` and `next_match()`.
    /// Unlike `next()`, there is no guarantee that the returned ranges of this and `next_match` will overlap.
    fn next_reject(&mut self) -> Option<(usize, usize)>;

}

pub trait ReverseSearcher<'haystack, P: Sized + Pattern>: Searcher<'haystack, P> {

    /// Performs the next search step starting from the back
    /// - returns `Match(a, b)` if `haystack[a..b]` matches the pattern
    /// - returns `Reject(a, b)` if `haystack[a..b]` cannot match the pattern (even partially)
    /// - returns `Done` if every byte of the haystack has been visited
    /// 
    /// The stream of `Match` and `Reject` values up to a `Done` will contain index ranges that are adjacent, non-overlapping, covering the whole haystack
    /// 
    /// A `Match` result needs to contain the whole matched pattern, however `Reject` results may be split up into arbitrary many adjacent fragments. Both ranges may have zero length
    fn next_back(&mut self) -> SearchStep;

    /// Finds the next `Match` result. See `next_back()`.
    fn next_match_back(&mut self) -> Option<(usize, usize)>;

    /// Finds the next `Reject` result. See `next_back()`.
    fn next_reject_back(&mut self) -> Option<(usize, usize)>;

}


/// Finds given needle in the haystack.
/// - returns index starting from haystack index 0
pub(self) fn find_from(needle: u8, haystack: &[u8], start: usize) -> Option<usize> {
    
    for (i, item) in haystack[start..].iter().enumerate() {
        if *item == needle {
            return Some(start + i)
        }
    }

    None

}

pub(self) fn rfind_from(needle: u8, haystack: &[u8], end: usize) -> Option<usize> {

    for (i, item) in haystack[..end].iter().enumerate().rev() {
        if *item == needle {
            return Some(i)
        }
    }

    None

}