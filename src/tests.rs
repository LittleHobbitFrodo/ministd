//	tests.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

//! Makes unit testing possible, but in a different way than the `std` does it
//! 
//! **TODO**: **REDO** (yes, **complete redo**) the testing mechanisms, its bit (well, bit more than a bit) scatchy and does not work well


/*use crate::RENDERER;
use crate::renderer::MinistdRenderer;
use crate::{eprint, println};

unsafe extern "C" {
    pub(crate) static __start_tests: usize;
    pub(crate) static __stop_tests: usize;
}

type TestFn = unsafe extern "Rust" fn() -> Result<(), Option<&'static str>>;
type TestReurnType = Result<(), Option<&'static str>>;

/// This structure is internally used for unit testing
/// - It may not be a good idea to use it on your own...
/// 
/// **name**: name of the test
/// - if tests are run manually at some point, pass this name to run this test
#[repr(C)]
pub struct Test {
    test_name: Option<&'static str>,
    fn_name: &'static str,
    f: TestFn,
    tested: bool,
}

impl Test {

    /// Returns slice of all `Test`s or `None` if there is an error
    pub fn get_all() -> Option<&'static mut [Test]> {

        let start = (unsafe { &__start_tests } as *const usize) as *mut Test;
        let end = (unsafe { & __stop_tests } as *const usize) as *mut Test;

        if start.is_null() || end.is_null() {
            return None;
        }

        let len = (((unsafe { &__stop_tests } as *const usize) as usize) - start as usize) / size_of::<Test>();

        if len == 0 {
            return None;
        }


        Some(unsafe {
            core::slice::from_raw_parts_mut(start, len)
        })
    }

    /// Constructs new test case
    pub const fn new(test_name: Option<&'static str>, fn_name: &'static str, f: TestFn) -> Self {
        Self {
            test_name,
            fn_name,
            f,
            tested: false,
        }
    }

    /// Returns name of this test
    pub const fn test_name(&self) -> Option<&'static str> {
        self.test_name
    }

    /// Returns name of the function
    pub const fn fn_name(&self) -> &'static str {
        self.fn_name
    }

    /// Return pointer to function
    pub const fn func(&self) -> TestFn {
        self.f
    }

    /// Runs the test marking it as executed
    pub fn run(&mut self) -> TestReurnType {
        self.tested = true;
        unsafe { (self.f)() }
    }

    /// Returns whether the test has been executed
    pub fn tested(&self) -> bool {
        self.tested
    }

}

#[unsafe(no_mangle)]
pub(crate) extern "Rust" fn __run_tests_with(test_name: Option<&'static str>, clear: bool) {

    let Some(slice) = Test::get_all() else {
        panic!("there are no tests");
    };

    //  check if lock can be acquired
    if let None = RENDERER.try_lock() {
        panic!("failed to acquire lock for RENDERER")
    }

    //  closure to reduce code
    let loop_body = |test: &mut Test| {
        if clear { RENDERER.lock().clear(); }

        if let Some(name) = test.test_name() {
            eprint!("RUNNING TEST \"{}\" ({}())", name, test.fn_name());
        } else {
            eprint!("RUNNING TEST {}()", test.fn_name());
        }

        if let Err(err) = test.run() {
            let mut rend = crate::RENDERER.lock();
            let width = rend.fb().width();
            //  report error
            rend.set_color(0xff0000);   //  red
            if let Some(e) = err {

                if width - rend.column() >= e.len() + 1 {
                    //  print error message
                    _ = rend.set_column(width - e.len() - 1);
                    println!(rend: "{e}");
                } else {
                    rend.endl();
                    println!(rend: "\tERROR: {e}");
                }
            } else {
                let msg = "UNKNOWN ERROR";
                _ = rend.set_column(width - msg.len());
                println!(rend: "{msg}");
            }

            //  reset color
            rend.set_color(0xffffff);
        } else {
            let mut rend = crate::RENDERER.lock();
            let width = rend.fb().width();
            rend.set_color(0x00ff00);   //  green
            _ = rend.set_column(width - 3);
            println!(rend: "OK");
            //  reset color
            rend.set_color(0xffffff);
        }

        
    };

    //  iterate over all tests and run them
    if let Some(tn) = test_name {        
        //  only tests with corresponding name

        for test in slice.iter_mut() {
            if test.tested() {
                continue;
            }
            if let Some(name) = test.test_name() {
                if name == tn {
                    loop_body(test);
                }
            }
        }
        
    } else {

        //  run all tests
        //  - does not return

        let mut tests = slice.iter_mut();


        while let Some(test) = tests.next() {
            if !test.tested() {
                loop_body(test);
            }
        }

        let mut rend = crate::RENDERER.lock();
        rend.set_color(0x90ff90);
        println!(rend: "ALL TESTS PASSED");

        crate::hang();
        
    }

}


/// Runs tests if built with `./util test` or `./util test custom`
/// 1. `run_tests!()` - runs all tests and hangs the kernel
/// 2. `run_tests!(<test name>)` - run all tests with specific name
///     - `run_tests!("test_memory")`
///     - you can name tests by using `#[testing(name)]`, for example `#[testing(test_memory)`
/// 3. `run_tests!(false)` - does not clear screen between individual tests
///     - too large output may end up in undefined behaviour
/// 4. `run_tests!(<test name>, false)` runs all tests with specific name (does not clear screen)
///     - `run_tests!("test_memory", false)`
#[macro_export]
macro_rules! run_tests {
    () => {
        #[cfg(feature = "custom_testing")]
        {
            unsafe extern "Rust" {
                fn __run_tests_with(test_name: Option<&'static str>, clear: bool);
            }
            unsafe { __run_tests_with(None, true) }
            $crate::hang();
        }
    };
    (false) => {
        #[cfg(feature = "custom_testing")]
        {
            unsafe extern "Rust" {
                fn __run_tests_with(test_name: Option<&'static str>, clear: bool);
            }
            unsafe { __run_tests_with(None, false) }
        }
    };
    ($name:literal) => {
        #[cfg(feature = "custom_testing")]
        {
            unsafe extern "Rust" {
                fn __run_tests_with(test_name: Option<&'static str>, clear: bool);
            }
            unsafe { __run_tests_with(Some($name), true) }
        }
    };
    ($name:literal, false) => {
        #[cfg(feature = "custom_testing")]
        {
            unsafe extern "Rust" {
                fn __run_tests_with(test_name: Option<&'static str>, clear: bool);
            }
            unsafe { __run_tests_with(Some($name), false) }
        }
    };
}*/