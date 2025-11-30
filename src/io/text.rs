//	mem/text.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build


pub use core::fmt::write;

/// formats and renders text onto the screen
/// 
/// ## Usage
/// ```rust
/// let x = 64;
/// print!("x = {x}");   //  prints "x = 64"
/// ```
/// ### While the `RENDERER` is locked
/// ```rust
/// let x = 64;
/// let mut rend = ministd::RENDERER.lock();
/// print!(rend: "x = {x}");
/// ```
#[macro_export]
macro_rules! print {
    ($guard:ident: $($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($guard, $($arg)*);
    }};
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!(*$crate::RENDERER.lock(), $($arg)*);
    }};
}

/// formats and renders text onto the screen and breaks line
/// 
/// ## Usage
/// ```rust
/// let msg = "Hello world!";
/// println!("message: \"{msg}\"");     //  prints "message: "Hello world!"\n"
/// ```
/// ### While the `RENDERER` is locked
/// ```rust
/// let msg = "Hello world!";
/// let mut rend = ministd::RENDERER.lock();
/// println!(rend: "message: \"{msg}\"");
/// ```
#[macro_export]
macro_rules! println {
    () => {
        #[cfg(all(feature="renderer", feature="spin"))]
        {
        use $crate::renderer::MinistdRenderer;
        $crate::RENDERER.lock().endl();
    }};
    ($guard:ident: $($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = writeln!($guard, $($arg)*);
    }};
    ($($arg:tt)*) => {
        #[cfg(all(feature="renderer", feature="spin"))]
        {
        use core::fmt::Write;
        let _ = writeln!(*$crate::RENDERER.lock(), $($arg)*);
    }};
}


/// formats and renders text as errorous
/// 
/// ## Usage
/// ```rust
/// let err = "something failed :(";
/// eprint!("ERROR: {err}");    //  prints "ERROR: something failed :("
/// ```
/// ### While the `RENDERER` is locked
/// ```rust
/// let err = "something failed :(";
/// let mut rend = ministd::RENDERER.lock();
/// eprint!(rend: "ERROR: {err}");
/// ```
#[macro_export]
macro_rules! eprint {
    ($guard:ident: $($arg:tt)*) => {{
        use core::fmt::Write;
        use $crate::renderer::MinistdRenderer;


        let c = $guard.color();
        $guard.set_color(0xff9a9a);

        let _ = write!($guard, $($arg)*);

        $guard.set_color(c);

    }};
    ($($arg:tt)*) => {
        #[cfg(all(feature="renderer", feature="spin"))]
        {
        use core::fmt::Write;
        use $crate::renderer::MinistdRenderer;

        let mut rend = $crate::renderer::RENDERER.lock();

        let c = rend.color();

        rend.set_color(0xff9a9a);

        let _ = write!(*rend, $($arg)*);

        rend.set_color(c);
    }};
}

/// formats and renders text as errorous and braks line
/// 
/// ## Usage
/// ```rust
/// let err = "something failed :(";
/// eprintln!("ERROR: {err}");    //  prints "ERROR: something failed :(\n"
/// ```
/// ### While the `RENDERER` is locked
/// ```rust
/// let err = "something failed :(";
/// let mut rend = ministd::RENDERER.lock();
/// eprintln!(rend: "ERROR: {err}");
/// ```
#[macro_export]
macro_rules! eprintln {
    ($guard:ident: $($arg:tt)*) => {{
        use core::fmt::Write;
        use $crate::renderer::MinistdRenderer;

        let c = $guard.color();
        $guard.set_color(0xff9a9a);

        let _ = write!($guard, $($arg)*);

        $guard.set_color(c);

    }};
    ($($arg:tt)*) => {
        #[cfg(all(feature="renderer", feature="spin"))]
        {
        use core::fmt::Write;
        let mut rend = $crate::renderer::RENDERER.lock();

        let c = rend.color();

        rend.set_color(0xff9a9a);

        let _ = writeln!(*rend, $($arg)*);
        
        rend.set_color(c);
    }};
}


/// Prints message if in debug mode
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        use $crate::renderer::MinistdRenderer;
        #[cfg(feature = "debug_build")]
            println!("DEBUG");
    }};
}

#[macro_export]
macro_rules! dbg {

    () => {
        $crate::println!("[{}:{}:{}]", core::file!(), core::line!(), core::column!());
    };
    ($guard:ident: $val:expr, $(,)?) => {{
        use $crate::renderer::MinistdRenderer;

        let color = $guard.color();
        $guard.set_color(0xff9a9a);

        let value = &$val;
        $crate::println!($guard: "[{}:{}:{}] = {:?}", core::file!(), core::line!(), core::column!(), core::stringify!($val),
        &&value as &dyn core::fmt::Debug);

        $guard.set_color(color);
    }};
    ($val:expr $(,)?) => {
        #[cfg(all(feature="renderer", feature="spin"))]
        {
        use $crate::renderer::MinistdRenderer;

        let mut rend = $crate::RENDERER.lock();
        let color = rend.color();
        rend.set_color(0xff9a9a);

        let value = &$val;

        $crate::println!(rend: "[{}:{}:{}] {} = {:?}", core::file!(), core::line!(), core::column!(), core::stringify!($val),
        &&value as &dyn core::fmt::Debug);
        rend.set_color(color);
    }};
}

/// Uses the `ministd::DebugRaw` trait to show implementation details about structure
#[macro_export]
macro_rules! dbg_raw {

    () => {
        $crate::println!("[{}:{}:{}]", core::file!(), core::line!(), core::column!());
    };
    ($guard:ident: $val:expr, $(,)?) => {{
        use $crate::renderer::MinistdRenderer;

        let color = $guard.color();
        $guard.set_color(0xff9a9a);

        let value = &$val;
        $crate::println!($guard: "[{}:{}:{}] = {:#?}", core::file!(), core::line!(), core::column!(), core::stringify!($val),
        &&value as &dyn core::fmt::Debug);

        $guard.set_color(color);
    }};
    ($val:expr $(,)?) => {
        #[cfg(all(feature="renderer", feature="spin"))]
        {
        use $crate::renderer::MinistdRenderer;

        let mut rend = $crate::RENDERER.lock();
        let color = rend.color();
        rend.set_color(0xff9a9a);

        let value = &$val;

        $crate::println!(rend: "[{}:{}:{}] {} = {:#?}", core::file!(), core::line!(), core::column!(), core::stringify!($val),
        &&value as &dyn core::fmt::Debug);
        rend.set_color(color);
    }};
}
