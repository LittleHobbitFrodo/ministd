pub mod font;

#[cfg(feature="spin")]
use crate::Mutex;
extern crate renderer;
pub use renderer::{MinistdRenderer, RendererStatus, Color};


#[cfg(feature = "default-renderer")]
pub mod default_renderer;
#[cfg(feature = "default-renderer")]
pub use default_renderer::Renderer;

#[cfg(not(feature = "default-renderer"))]
pub use renderer::Renderer;

#[cfg(feature="spin")]
pub static RENDERER: Mutex<Renderer> = Mutex::new(Renderer::new());