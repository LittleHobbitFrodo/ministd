//  mem/string/searcher/searchers.rs (ministd crate)
//  this file originally belonged to baseOS project
//      an OS template on which to build

use super::{ReverseSearcher, SearchStep, Searcher};

use super::find_from;

/// Associated type for <char as Pattern>::Searcher<'a>.
#[derive(Clone, Debug, PartialEq)]
#[repr(C)]  //  make sure the layout will be as dense as possible
pub struct CharSearcher<'l> {
    haystack: &'l [u8],  //  string where to find
    finger: u32,         //  last search index
    finger_back: u32,    //  last backwards search index
    needle: u8,         //  character to find
}

impl<'l> Searcher<'l, u8> for CharSearcher<'l> {

    type Needle = u8;

    #[inline]
    fn new(haystack: &'l str, needle: u8) -> Self {
        Self {
            haystack: haystack.as_bytes(),
            finger: 0,
            finger_back: (haystack.len() - 1) as u32,
            needle: needle,
        }
    }

    #[inline(always)]
    fn haystack(&self) -> &str { crate::convert::strify(self.haystack) }

    fn next(&mut self) -> SearchStep {

        if self.finger as usize >= self.haystack.len() {
            return SearchStep::Done
        }

        if self.haystack[self.finger as usize] == self.needle {

            let start = self.finger as usize;

            self.finger += 1;

            return if self.finger as usize == self.haystack.len() {
                SearchStep::LastMatch(start, self.finger as usize)
            } else {
                SearchStep::Match(start, self.finger as usize)
            }
        }

        if let Some(index) = find_from(self.needle, self.haystack, self.finger as usize) {
            let start = self.finger as usize;
            self.finger = index as u32;
            return if self.finger as usize == self.haystack.len() - 1 {
                SearchStep::LastReject(start, self.finger as usize)
                //SearchStep::LastMatch(index, self.finger as usize)
            } else {
                SearchStep::Reject(start, self.finger as usize)
                //SearchStep::Match(index, self.finger as usize)
            }
        } else {
            self.finger = self.haystack.len() as u32;
            SearchStep::LastReject(self.finger as usize, self.haystack.len())
        }
        
    }

    fn next_match(&mut self) -> Option<(usize, usize)> {

        //find_from(self.needle, self.haystack, self.finger as usize).map(|index| (index, index + 1))

        if self.finger as usize >= self.haystack.len() {
            return None
        }

        if let Some(index) = find_from(self.needle, self.haystack, self.finger as usize) {
            self.finger = (index + 1) as u32;
            Some((index, self.finger as usize))
        } else {
            self.finger = self.haystack.len() as u32;
            None
        }
    }

    fn next_reject(&mut self) -> Option<(usize, usize)> {

        if self.finger as usize >= self.haystack.len() {
            return None
        }
        
        if let Some(index) = find_from(self.needle, self.haystack, self.finger as usize) {
            self.finger = index as u32;
            Some((index -1, self.finger as usize))
        } else {
            Some((self.finger as usize, self.haystack.len()))
        }
    }

}

impl<'l> ReverseSearcher<'l, u8> for CharSearcher<'l> {
    
    fn next_back(&mut self) -> SearchStep {
        let bytes = self.haystack;

        if let Some(c) = bytes.get(self.finger_back as usize) {
            if *c == self.needle {
                let start = self.finger_back as usize;
                self.finger_back -= 1;
                if start == 0 {
                    SearchStep::LastMatch(start, start + 1)
                } else {
                    SearchStep::Match(start, start + 1)
                }
            } else {
                //  find the whole reject region
                let mut iter = bytes[..self.finger_back as usize].iter();
                let end = (self.finger_back + 1) as usize;
                self.finger_back -= 1;

                while let Some(c) = iter.next_back() {
                    if *c == self.needle {
                        return SearchStep::Reject((self.finger_back + 1) as usize, end);
                    }

                    self.finger_back -= 1;
                }

                SearchStep::LastReject(0, end)
            }
        } else {
            SearchStep::Done
        }
    }

    fn next_match_back(&mut self) -> Option<(usize, usize)> {
        let mut iter = self.haystack[..self.finger_back as usize].iter();

        while let Some(c) = iter.next_back() {
            if *c == self.needle {
                let end = self.finger_back as usize;
                self.finger_back -= 1;
                return Some((self.finger_back as usize, end));
            }

            self.finger_back -= 1;
        }
        None
    }

    fn next_reject_back(&mut self) -> Option<(usize, usize)> {
        let mut iter = self.haystack[..self.finger_back as usize].iter();
        let end = self.finger_back as usize;

        while let Some(c) = iter.next_back() {
            if *c == self.needle {
                let start = self.finger_back as usize;
                self.finger_back -= 1;
                return Some((start, end))
            }
            self.finger_back -= 1;
        }
        
        if end != self.finger_back as usize {
            Some((0, end))
        } else {
            None
        }

    }

}












/// Searches for substring in strings
pub struct StrSearcher<'haystack, 'needle> {
    haystack: &'haystack [u8],
    needle: &'needle [u8],
    finger: u32,
    //finger_back: u32,
    next_match: bool,   //  next iteration will match
}

impl<'haystack,'needle > Searcher<'haystack, &'needle str> for StrSearcher<'haystack, 'needle> {

    type Needle = &'needle str;
    
    #[inline]
    fn new(haystack: &'haystack str, needle: &'needle str) -> Self {
        /*let (finger, back) = if haystack.len() == 0 || needle.len() == 0 || haystack.len() < needle.len() {
            //  errorous input -> searching functions will return `Done`
            (haystack.len() as u32, 0)
        } else {
            (0, haystack.len() as u32)
        };*/
        let finger = (haystack.len() * ( haystack.len() == 0 || needle.len() == 0
            || haystack.len() < needle.len()) as usize) as u32;
        Self {
            haystack: haystack.as_bytes(),
            needle: needle.as_bytes(),
            finger: finger,
            //finger_back: back,
            next_match: false,
        }
    }

    #[inline]
    fn haystack(&self) -> &str { crate::convert::strify(self.haystack) }

    fn next(&mut self) -> SearchStep {

        let start = self.finger as usize;

        if start >= self.haystack.len() {
            return SearchStep::Done
        }

        let max = self.haystack.len() - self.needle.len();

        if self.next_match {
            self.next_match = false;
            self.finger += self.needle.len() as u32;
            if start >= max {
                return SearchStep::LastMatch(start, self.finger as usize)
            } else {
                return SearchStep::Match(start, self.finger as usize)
            }
        }

        let first = self.needle[0];
        let slice = &self.haystack[..max + 1];

        while let Some(index) = find_from(first, slice, self.finger as usize) {
            let last_index = index + self.needle.len();
            if &self.haystack[index..last_index] == self.needle {
                if index != start {
                    self.finger = index as u32;
                    self.next_match = true;
                    return SearchStep::Reject(start, index)
                } else {
                    return if index >= max {
                        self.finger = self.haystack.len() as u32;
                        SearchStep::LastMatch(start, self.haystack.len())   
                    } else {
                        self.finger = last_index as u32;
                        SearchStep::Match(start, last_index)
                    }
                }
            } else {
                self.finger = (index + 1) as u32;
                continue;
            }
        }

        //  no more matching characters
        SearchStep::LastReject(start, self.haystack.len())
    }

    fn next_match(&mut self) -> Option<(usize, usize)> {
        
        if self.finger as usize >= self.haystack.len() {
            return None
        }

        let max = self.haystack.len() - self.needle.len();
        let first = self.needle[0];
        let slice = &self.haystack[..max + 1];

        while let Some(index) = find_from(first, slice, self.finger as usize) {
            let last_index = index + self.needle.len();
            if &self.haystack[index..last_index] == self.needle {
                self.finger = last_index as u32;
                return Some((index, last_index))
            } else {
                self.finger += 1;
            }
        }

        self.finger = self.haystack.len() as u32;
        None

    }

    fn next_reject(&mut self) -> Option<(usize, usize)> {

        let start = self.finger as usize;

        if start >= self.haystack.len() {
            return None
        }

        let max = self.haystack.len() - self.needle.len();
        let first = self.needle[0];
        let slice = &self.haystack[..max +  1];

        while let Some(index) = find_from(first, slice, self.finger as usize) {
            let last = index + self.needle.len();
            if &self.haystack[index..last] == self.needle {
                return if index > start {
                    self.finger = last as u32;
                    Some((start, index))
                } else {
                    self.finger = last as u32;
                    self.next_reject()
                };
            } else {
                self.finger = (index + 1) as u32;
            }
        }
        self.finger = self.haystack.len() as u32;
        Some((start, self.haystack.len()))
    }

}






pub struct CharPredicateSearcher<'haystack, F>
where F: FnMut(u8) -> bool {
    haystack: &'haystack [u8],
    predicate: F,
    finger: u32,
    //finger_back: u32,
    next_match: bool,
}

impl<'haystack, F> Searcher<'haystack, F> for CharPredicateSearcher<'haystack, F>
where F: FnMut(u8) -> bool + Clone {

    type Needle = F;
    
    fn new(haystack: &'haystack str, needle: F) -> Self {
        Self {
            haystack: haystack.as_bytes(),
            predicate: needle.clone(),
            finger: 0,
            //finger_back: (haystack.len() - 1) as u32,
            next_match: false,
        }
    }

    fn haystack(&self) -> &str { crate::convert::strify(self.haystack) }

    fn next(&mut self) -> SearchStep {

        let start = self.finger as usize;

        if self.next_match {
            self.next_match = false;
            self.finger += 1;
            return SearchStep::Match(start, self.finger as usize)
        } else if self.finger as usize >= self.haystack.len() {
            return SearchStep::Done
        }

        let slice = &self.haystack[self.finger as usize..];

        for (i, item) in slice.iter().enumerate() {
            if (self.predicate)(*item) {
                self.next_match = true;
                self.finger = (start + i) as u32;
                return SearchStep::Reject(start, self.finger as usize)
            }
        }

        self.finger = self.haystack.len() as u32;
        SearchStep::LastReject(start, self.haystack.len())


    }

    fn next_match(&mut self) -> Option<(usize, usize)> {

        let start = self.finger as usize;

        if self.finger as usize >= self.haystack.len() {
            return None
        }

        let slice = &self.haystack[self.finger as usize..];

        for (i, item) in slice.iter().enumerate() {
            if (self.predicate)(*item) {
                self.finger = (start + i) as u32;
                return Some((self.finger as usize, (self.finger + 1) as usize))
            }
        }

        None
    }

    fn next_reject(&mut self) -> Option<(usize, usize)> {
        None
    }

} 

