//	lib.rs (renderer crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

//! Lets you build your own renderer to be use by the `ministd` crate instead of the default one
//! read more in the `renderer/mod.rs` file

#![no_std]



use spin::{Mutex, MutexGuard};


//  renderer configuration

/// Sets the tab size for the renderer
pub const TAB_SIZE: usize = 6;
/// Sets space between each lines of rendered text (in pixels)
pub const SPACE_BETWEEN_LINES: u16 = 3;

pub mod renderer;
pub mod color;

pub use renderer::Renderer;
pub use color::Color;
use core::ptr::NonNull;

/// Marks the status of the renderer
#[derive(Copy, Clone)]
pub enum RendererStatus {
    /// Uninitialized (does not work)
    Uninit,
    /// Renders text to the screen
    ScreenRendering,
    /// Passes data to custom function
    /// - will be printed to UART, or something..
    Other,
}


/// Unifying the API for the Renderer and how should it work
/// 
/// all functions that renders something does not return any value
/// - if framebuffer is missing, passes the data to `TODO` function
/// 
//  Do not change the declaration of the trait for it could break the default renderer
//  - and alwo it may break the rendering system as whole
pub trait MinistdRenderer: Send + core::fmt::Write {

    type FrameBuffer;

    /// Returns the status of the renderer
    fn status(&self) -> RendererStatus;

    fn fb(&self) -> &Self::FrameBuffer;

    /// Initializes the renderer and makes it work
    /// - returns `Err` if failes
    fn init(&mut self, fb: NonNull<u32>, width: usize, height: usize) -> Result<(), ()>;
    //fn init(&mut self, fb: &limine::request::FramebufferRequest) -> Result<(), ()>;

    /// Returns the current horizontal position of the cursor
    /// - left to right
    fn column(&self) -> usize;

    /// Returns the current vertical position of the cursor
    /// - up to bottom
    fn line(&self) -> usize;

    /// Returns the position of the cursor as `(x, y)`
    fn position(&self) -> (usize, usize);

    /// Clears the display (sets all pixels to default background colors)
    fn clear(&mut self);

    /// Sets the vertical position of the cursor to the specified value
    /// - returns `Err` if `line` is out of bounds of the framebuffer
    fn set_line(&mut self, line: usize) -> Result<(), ()>;

    /// Sets the horizontal position of the cursor to the specified value
    fn set_column(&mut self, column: usize) -> Result<(), ()>;

    /// Sets horizontal and verical position of the cursor
    /// - returns `Err` if `line` or `row` is out of bounds of the framebuffer
    fn set_pos(&mut self, line: usize, row: usize) -> Result<(), ()>;

    /// Sets text color
    fn set_color(&mut self, color: u32);

    /// Returns the current text color
    fn color(&self) -> u32;

    /// Renders one character at the cursor position while moving the cursor to right
    /// - potentionally moves the cursor one line below
    /// - alignes the cursor to `TAB_SIZE` constant on tab character
    fn render(&mut self, c: u8);

    /// Renders the string at the cursor position
    fn print(&mut self, str: &[u8]);

    /// Renders the string at the cursor position and breaks the line
    fn println(&mut self, str: &[u8]);

    /// Breaks the line
    fn endl(&mut self);

    /// Prints the tab character (aligns the horizontal position of the cursor to the `TAB_SIZE` constant)
    fn tab(&mut self);


}
