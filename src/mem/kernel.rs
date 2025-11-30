//	mem/kernel.rs (ministd crate)
//	this file originally belonged to the baseOS project
//		an OS template on which to build

//! Marks position and size of each section of the kernel

/*use super::Region;
use crate::RwLock;

pub struct Layout {
    pub kernel: Region,
    pub rodata: Region,
    pub data: Region,
    pub dynamic: Region,
    pub bss: Region,
}

impl Layout {
    pub const fn new() -> Self {
        Self {
            kernel: Region::empty(),
            rodata: Region::empty(),
            data: Region::empty(),
            dynamic: Region::empty(),
            bss: Region::empty(),
        }
    }

    #[inline]
    fn transform(var: &usize) -> usize {
        (var as *const usize) as usize
    }

    pub fn init(&mut self) {

        let kstart = Self::transform(unsafe { &__KERNEL_START });
        let kend = Self::transform(unsafe { &__KERNEL_END });
        self.kernel = Region::new(kstart, 0, kend - kstart);

        let rostart = Self::transform(unsafe { &__RODATA_START });
        let roend = Self::transform(unsafe { &__RODATA_END });
        self.rodata = Region::new(rostart, 0, roend - rostart);

        let dstart = Self::transform(unsafe { &__DATA_START });
        let dend = Self::transform(unsafe { &__DATA_END });
        self.data = Region::new(dstart, 0, dend - dstart);

        let dystart = Self::transform(unsafe { &__DYNAMIC_START });
        let dyend = Self::transform(unsafe { &__DYNAMIC_END });
        self.dynamic = Region::new(dystart, 0, dyend - dystart);

        let bstart = Self::transform(unsafe { &__BSS_START });
        let bend = Self::transform(unsafe { &__BSS_END });
        self.dynamic = Region::new(bstart, 0, bend - bstart);
        


    }

}

/// Stores kernel memory layout metadata
/// - position and size of all linker sections
/// - physical addresses are unknown by default
/// 
/// **NOTE**: do not forget to call the `ministd::init::memory()` to fully initialize the data
pub static LAYOUT: RwLock<Layout> = RwLock::new(Layout::new());


//  variables that are used to identify each sections
unsafe extern "C" {
    pub(crate) static __KERNEL_START: usize;
    pub(crate) static __KERNEL_END: usize;

    pub(crate) static __TEXT_START: usize;
    pub(crate) static __TEXT_END: usize;

    pub(crate) static __RODATA_START: usize;
    pub(crate) static __RODATA_END: usize;

    pub(crate) static __DATA_START: usize;
    pub(crate) static __DATA_END: usize;

    pub(crate) static __DYNAMIC_START: usize;
    pub(crate) static __DYNAMIC_END: usize;

    pub(crate) static __BSS_START: usize;
    pub(crate) static __BSS_END: usize;

}
*/