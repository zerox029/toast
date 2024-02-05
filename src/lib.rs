#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(internal_features)]
#![feature(lang_items)]
#![feature(ptr_internals)]
#![feature(panic_info_message)]
#![feature(allocator_api)]

extern crate alloc;
extern crate rlibc;

use core::panic::PanicInfo;
use alloc::boxed::Box;
use x86_64::registers::model_specific::Efer;
use x86_64::registers::control::{Cr0, Cr0Flags, EferFlags};
use crate::memory::init_memory_modules;

pub mod vga_buffer;
pub mod arch;
pub mod memory;
mod test_runner;

#[no_mangle]
pub extern fn _main(multiboot_information_address: usize) {
    init(multiboot_information_address);

    let x = Box::new(41);

    println!("No crash");

    loop {}
}

fn init(multiboot_information_address: usize) {
    vga_buffer::clear_screen();

    println!("Toast version v0.0.1-x86_64");

    let boot_info = unsafe{ arch::multiboot2::load(multiboot_information_address) };

    unsafe {
        Efer::write(EferFlags::NO_EXECUTE_ENABLE);
        Cr0::write(Cr0::read() | Cr0Flags::WRITE_PROTECT);
    }

    init_memory_modules(boot_info);
}

fn print_memory_areas(multiboot_information_address: usize) {
    let boot_info = unsafe{ arch::multiboot2::load(multiboot_information_address) };
    let memory_map = boot_info.memory_map().expect("Memory map tag required");
    let elf_symbols = boot_info.elf_symbols().expect("Elf symbols tag required");

    println!("Memory areas:");
    for entry in memory_map.entries() {
        println!("    start: 0x{:x}, length: 0x{:x}",
                 entry.base_addr, entry.size);
    }

    println!("kernel sections:");
    for section in elf_symbols.section_headers() {
        println!("    addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
                 section.start_address(), section.size(), section.flags());
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let location = info.location().unwrap();
    println!("{}", info);

    loop {}
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}