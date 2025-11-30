//! This is where you define your own renderer


//  All you have to do:
//      1. Create structure `X` that renders text to the screen
//          - implements the `crate::Renderer` trait
//          - has `const fn new() -> Self` implemented on its own
//      2. Export the structure `X` as `Renderer`
//      3. Create structure `Y` that is used to store framebuffer data
//      4. Export the structure `Y` as `Framebuffer`



//  for example


pub struct Renderer {
    /* ... */
}
impl Renderer {
    //  pub const fn new() -> Self {}
}

/*impl MinistdRenderer for Renderer {

}*/