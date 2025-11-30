//	io/hardware.rs (ministd crate)
//	this file originally belonged to baseOS project
//		an OS template on which to build

use core::arch::asm;

pub use inner::*;


#[cfg(not(target_arch = "x86_64"))]
mod inner {
    /// Does not support other architectures that x86_64
    pub fn outb(_: u16, _: u8) {}
    /// Does not support other architectures that x86_64
    pub fn inb(_: u16) -> u8 { 0 }

    /// Does not support other architectures that x86_64
    pub fn outw(_: u16, _: u16) {}
    /// Does not support other architectures that x86_64
    pub fn inw(_: u16) -> u16 { 0 }


    /// Does not support other architectures that x86_64
    pub fn outd(_: u16, _: u32) {}
    /// Does not support other architectures that x86_64
    pub fn ind(_: u16) -> u32 { 0 }

    /// Does not support other architectures that x86_64
    pub fn outq(_: u16, _: u64) {}
    /// Does not support other architectures that x86_64
    pub fn inq(_: u16) -> u64 { 0 }

    pub mod int {
        /// Does not support other architectures that x86_64
        pub fn disable() {}
        /// Does not support other architectures that x86_64
        pub fn enable() {}
    }
}

#[cfg(target_arch = "x86_64")]
mod inner {
    use core::arch::asm;
    /// Sends byte to a specified port
    #[inline(always)]
    pub fn outb(port: u16, data: u8) {
        unsafe {
            asm!("out dx, al",
                in("dx") port,
                in("al") data,
                options(nostack, preserves_flags, nomem)
            );
        }
    }

    /// Reads byte from specified port
    #[inline]
    pub fn inb(port: u16) -> u8 {
        let mut ret: u8;

        //#[cfg(not(feature = "testing"))]
        unsafe {
            asm!("in al, dx",
                in("dx") port,
                out("al") ret,
                options(nostack, preserves_flags, nomem)
            );
        }
        ret
    }

    /// Sends a word (`u16`) to specified port
    #[inline(always)]
    pub fn outw(port: u16, data: u16) {

        //#[cfg(not(feature = "testing"))]
        unsafe {
            asm!("out dx, ax",
                in("dx") port,
                in("ax") data,
                options(nostack, preserves_flags, nomem)
            );
        }
    }

    /// Reads a word (`u16`) from specified port
    #[inline]
    pub fn inw(port: u16) -> u16 {
        let mut ret: u16;

        //#[cfg(not(feature = "testing"))]
        unsafe {
            asm!("in, ax, dx",
                in("dx") port,
                out("ax") ret,
                options(nostack, preserves_flags, nomem)
            );
        }
        ret
    }

    /// Sends a dword (`u32`) to specified port
    #[inline(always)]
    pub fn outd(port: u16, data: u32) {
        //#[cfg(not(feature = "testing"))]
        unsafe {
            asm!("out dx, eax",
            in("eax") data,
            in("dx") port,
            options(nostack, preserves_flags, nomem));
        }
    }

    /// Reads a dword (`u32`) from specified port
    #[inline]
    pub fn ind(port: u16) -> u32 {
        let mut ret: u32;

        //#[cfg(not(feature = "testing"))]
        unsafe {
            asm!("in, eax, dx",
                in("dx") port,
                out("eax") ret,
                options(nostack, preserves_flags, nomem)
            );
        }
        ret
    }

    /// Sends a qword (`u64`) to specified port
    #[inline]
    pub fn outq(port: u16, data: u64) {
        outd(port, data as u32);
        outd(port.wrapping_add(4), (data >> 32) as u32);
    }

    #[inline]
    /// Reads a qword from specified port
    pub fn inq(port: u16) -> u64 {
        ind(port) as u64 | ((ind(port.wrapping_add(4)) as u64) << 32)
    }


    /// Wait aprox. nanosecond
    pub fn wait() {
        outb(0x80, 0);
    }

    pub mod int {
    use core::arch::asm;
    
    #[inline(always)]
    pub fn disable() {
        //#[cfg(not(feature = "testing"))]
        unsafe { asm!("cli", options(nostack, nomem)); }
    }

    #[inline(always)]
    pub fn enable() {
        //#[cfg(not(feature = "testing"))]
        unsafe { asm!("sti", options(nostack, nomem)); }
    }
}

}