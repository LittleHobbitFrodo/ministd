//	renderer.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

//! Provides the default text renderer for the `ministd` library through external crate (`lib/renderer`)


use spin::{Mutex, MutexGuard};
use super::Color;
use super::font;
extern crate renderer;
use renderer::{SPACE_BETWEEN_LINES, TAB_SIZE, MinistdRenderer, RendererStatus};
use core::ptr::NonNull;




pub struct Renderer {
    row: usize,
    line: usize,
    fb: FrameBuffer,
    col: Color,
    //space: u16,
    status: RendererStatus
}

//unsafe impl Sync for DefaultRenderer {}
unsafe impl core::marker::Send for Renderer {}

impl Renderer {

    pub const fn new() -> Self {
        Self {
            row: 0,
            line: 0,
            fb: FrameBuffer::new(),
            col: Color::new(0xffffff),
            //space: 0,
            status: RendererStatus::Uninit,
        }
    }

    fn rend(&mut self, c: u8) {
        match c {
            0..31 => {
                match c {
                    b'\n' => {
                        self.endl();
                        return;
                    },
                    b'\t' => {
                        self.tab();
                        return;
                    },
                    _ => return,
                }
            },
            _ => {
                let fnt = match super::font::FONT.as_ref().get_char(c) {
                    Some(f) => f,
                    None => &font::ERR_CHAR,
                };
                let fb = self.fb();
                let arr = unsafe {fb.address().cast::<u32>().add((self.line * fb.width * (font::FONT_BITS + SPACE_BETWEEN_LINES as usize)) + self.row * font::FONT_BITS)};

                for i in 0..font::FONT_BITS {
                    for ii in 0..font::FONT_BITS {
                        unsafe {arr.add((i as usize * fb.width) + (font::FONT_BITS - ii as usize)).write(self.color() * ((fnt[i] as u32 >> ii as u32) & 1))};
                    }
                }
                self.row += 1;
                if self.row >= self.fb.width {
                    self.endl();
                }
            }
        }
    }
}

impl MinistdRenderer for Renderer {

    type FrameBuffer = FrameBuffer;


    fn init(&mut self, fb: NonNull<u32>, width: usize, height: usize) -> Result<(), ()> {
        self.col = Color::new_rgb(255, 255, 255);
        self.row = 0;
        self.line = 0;
        //self.space = SPACE_BETWEEN_LINES;
        if let Ok(_) = FrameBuffer::init(&mut self.fb, fb, width, height) {
            self.status = RendererStatus::ScreenRendering;
            Ok(())
        } else {
            Err(())
        }
    }

    #[inline(always)] fn column(&self) -> usize { self.row }
    #[inline(always)] fn line(&self) -> usize { self.line }
    #[inline(always)] fn fb(&self) -> &FrameBuffer { &self.fb }
    #[inline(always)] fn color(&self) -> u32 { self.col.as_int() }
    #[inline(always)] fn set_color(&mut self, color: u32) {self.col.set_int(color);}

    #[inline(always)] fn status(&self) -> RendererStatus { self.status }

    #[inline(always)] fn position(&self) -> (usize, usize) { (self.row, self.line) }

    fn set_line(&mut self, line: usize) -> Result<(), ()> {
        if line < self.fb.height {
            self.line = line;
            Ok(())
        } else {
            Err(())
        }
    }

    fn set_column(&mut self, column: usize) -> Result<(), ()> {
        if column < self.fb.width() {
            self.row = column;
            Ok(())
        } else {
            Err(())
        }
    }



    /*fn space(&mut self) {
        self.row += 1;
        if self.row >= self.fb.width {
            self.row = 0;
            self.line += 1;
        }
    }*/
    #[inline]
    fn endl(&mut self) {
        self.line += 1;
        self.row = 0;
    }
    #[inline]
    fn tab(&mut self) {
        self.row += TAB_SIZE - (self.row % TAB_SIZE);
        if self.row >= self.fb.width {
            self.endl();
        }
    }

    fn clear(&mut self) {
        for i in unsafe { core::slice::from_raw_parts_mut(self.fb.address, self.fb.width*self.fb.height) } {
            i.set_int(0);       //  black
        }
    }

    fn set_pos(&mut self, line: usize, row: usize) -> Result<(), ()> {
        if line < self.fb.height && row < self.fb.width {
            self.line = line;
            self.row = row;
            Ok(())
        } else {
            Err(())
        }
    }

    

    #[inline(always)]
    fn render(&mut self, c: u8) {
        match self.status {
            RendererStatus::ScreenRendering => self.rend(c),
            RendererStatus::Other => todo!(),
            _ => return,
        }
    }

    #[inline(always)]
    fn print(&mut self, str: &[u8]) {
        match self.status {
            RendererStatus::ScreenRendering => {
                for i in str {
                    self.rend(*i);
                }
            },
            RendererStatus::Other => todo!(),
            _ => return,
        }
    }
    fn println(&mut self, str: &[u8]) {
        match self.status {
            RendererStatus::ScreenRendering => {
                for i in str {
                    self.rend(*i);
                }
                self.endl();
            },
            RendererStatus::Other => todo!(),
            _ => return,
        }
    }
}

impl AsRef<Renderer> for Renderer {
    #[inline(always)]
    fn as_ref(&self) -> &Renderer {
        &self
    }
}

impl AsMut<Renderer> for Renderer {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Renderer {
        self
    }
}

pub struct FrameBuffer {
    width: usize,
    height: usize,
    address: *mut Color,
    bpp: usize,
    initialized: bool
}

impl FrameBuffer {
    pub const fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            address: core::ptr::null_mut(),
            bpp: 0,
            initialized: false
        }
    }
    pub fn init(&mut self, fb: NonNull<u32>, width: usize, height: usize) -> Result<(), ()> {
        self.bpp = 32;
        self.width = width;
        self.height = height;
        self.address = fb.as_ptr() as *mut Color;
        self.initialized = true;
        Ok(())
    }

    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }
    pub fn bpp(&self) -> usize { self.bpp }
    pub fn address(&self) -> *mut Color {
        self.address
    }
}


#[inline(always)]
pub fn init<T>(renderer: &Mutex<T>, fb: NonNull<u32>, width: usize, height: usize) -> Result<(), ()>
where T: renderer::MinistdRenderer {
    renderer.lock().init(fb, width, height)
}


impl core::fmt::Write for Renderer {
    #[inline]
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.render(c as u8);
        Ok(())
    }

    #[inline]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.print(s.as_bytes());
        Ok(())
    }

}

impl Renderer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.print(s.as_bytes());
        Ok(())
    }
}



/// Helper trait for the [`Renderer`]
/// - classic [`core::fmt::Display`] should be prefered
pub trait Render {
    fn render(&self);
    fn render_locked<'l>(&self, guard: &'l mut MutexGuard<Renderer>);
}