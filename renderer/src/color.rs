//	renderer/color.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build


//  this file provides simple [`Color`] structure to use with the renderer

//use core::{fmt::Display, time::Duration};

use crate::{MinistdRenderer, Mutex, MutexGuard};



#[derive(Copy, Clone)]
pub struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Copy, Clone)]
union Col {
    rgb: Rgb,
    int: u32,
}

impl core::fmt::Debug for Col {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe { write!(f, "rgb({}, {}, {})", self.rgb.r, self.rgb.g, self.rgb.b) }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
    value: Col
}

impl Color {
    #[inline(always)]
    pub fn as_int(&self) -> u32 {
        unsafe { self.value.int }
    }
    #[inline(always)]
    pub fn as_rgb(&self) -> Rgb {
        unsafe { self.value.rgb }
    }
    #[inline(always)]
    pub fn set_int(&mut self, val: u32) {
        self.value.int = val;
    }
    #[inline(always)]
    pub fn set_rgb(&mut self, val: Rgb) {
        self.value.rgb = val;
    }

    /// Tries to lock the renderer and set `self` as the color of the `RENDERER`
    /// - returns `false` if fails
    #[inline]
    pub fn set<R>(&self, renderer: &Mutex<R>) -> bool
    where R: crate::MinistdRenderer {
        match renderer.try_lock() {
            Some(mut guard) => {
                guard.set_color(self.as_int());
                true
            },
            None => false,
        }
    }

    /// Sets `self` as the color of the renderer behind the `guard`
    #[inline(always)]
    pub fn set_locked<R>(&self, guard: &mut MutexGuard<R>)
    where R: MinistdRenderer {
        guard.set_color(self.as_int());
    }

}


impl Color {

    pub const fn new(value: u32) -> Self {
        Self { value: Col {int: value} }
    }
    pub const fn new_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self { value: Col { rgb: Rgb { r: red, g: green, b: blue } } }
    }
    pub const fn new_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {value: Col { int: red as u32 | (green as u32) << 8 | (blue as u32) << 16 | (alpha as u32) << 24 } }
    }

    #[inline(always)]
    pub fn red(&self) -> u8 {
        unsafe { self.value.rgb.r }
    }
    #[inline(always)]
    pub fn green(&self) -> u8 {
        unsafe { self.value.rgb.g }
    }
    #[inline(always)]
    pub fn blue(&self) -> u8 {
        unsafe { self.value.rgb.b }
    }

    #[inline]
    pub fn set_red(&mut self, red: u8) {
        self.value.rgb.r = red;
    }
    #[inline]
    pub fn set_green(&mut self, green: u8) {
        self.value.rgb.g = green;
    }
    #[inline]
    pub fn set_blue(&mut self, blue: u8) {
        self.value.rgb.b = blue;
    }

}