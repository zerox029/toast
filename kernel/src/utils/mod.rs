use core::arch::asm;
use limine::memory_map::EntryType;
use crate::MEMORY_MAP_REQUEST;

pub mod bitutils;
pub mod tests;

pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
    )
}

pub fn hcf() -> ! {
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}