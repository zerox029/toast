#![allow(internal_features)]
#![allow(dead_code)]
#![feature(lang_items)]
#![feature(ptr_internals)]
#![no_std]

extern crate rlibc;

use core::panic::PanicInfo;

pub mod vga_buffer;
pub mod arch;

#[no_mangle]
pub extern fn _start(multiboot_information_address: usize) {
    let boot_info = unsafe { arch::multiboot2::load_boot_information(multiboot_information_address) };

    let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");


    println!("Memory areas: ");
    for area in memory_map_tag.memory_areas() {
        println!("    start: 0x{:x}, length: 0x{:x}", area.base_addr, area.length);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}