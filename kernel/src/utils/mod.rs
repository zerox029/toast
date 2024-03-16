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

pub fn print_memory_map() {
    MEMORY_MAP_REQUEST.get_response().unwrap().entries().iter().for_each(|entry| {
        match entry.entry_type {
            EntryType::USABLE => serial_println!("usable entry from 0x{:X} to {:X}", entry.base, entry.base + entry.length),
            EntryType::RESERVED => serial_println!("reserved entry from 0x{:X} to {:X}",  entry.base, entry.base + entry.length),
            EntryType::ACPI_RECLAIMABLE => serial_println!("acpi recl entry from 0x{:X} to {:X}",  entry.base, entry.base + entry.length),
            EntryType::ACPI_NVS => serial_println!("acpi nvs entry from 0x{:X} to {:X}",  entry.base, entry.base + entry.length),
            EntryType::BAD_MEMORY => serial_println!("bad memory entry from 0x{:X} to {:X}",  entry.base, entry.base + entry.length),
            EntryType::BOOTLOADER_RECLAIMABLE => serial_println!("bootloader recl entry from 0x{:X} to {:X}",  entry.base, entry.base + entry.length),
            EntryType::KERNEL_AND_MODULES => serial_println!("kernel entry from 0x{:X} to {:X}",  entry.base, entry.base + entry.length),
            EntryType::FRAMEBUFFER => serial_println!("framebuffer entry from 0x{:X} to {:X}",  entry.base, entry.base + entry.length),
            _ => ()
        }
    });
}