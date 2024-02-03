#![allow(internal_features)]
#![allow(dead_code)]
#![feature(lang_items)]
#![feature(ptr_internals)]
#![feature(panic_info_message)]
#![no_std]

extern crate rlibc;

use core::panic::PanicInfo;

pub mod vga_buffer;
pub mod arch;
pub mod memory;

#[no_mangle]
pub extern fn _main(multiboot_information_address: usize) {
    let boot_info = unsafe{ arch::multiboot2::load(multiboot_information_address) };
    let memory_map = boot_info.memory_map().expect("Memory map tag required");
    let elf_symbols = boot_info.elf_symbols().expect("Elf symbols tag required");

    let kernel_start = elf_symbols.section_headers().map(|s| s.start_address()).min().unwrap();
    let kernel_end = elf_symbols.section_headers().map(|s| s.end_address()).min().unwrap();

    let multiboot_start = multiboot_information_address;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);

    let mut frame_allocator = memory::page_frame_allocator::PageFrameAllocator::new(kernel_start,
                                                                                    kernel_end,
                                                                                    multiboot_start,
                                                                                    multiboot_end,
                                                                                    memory_map.entries());

    memory::test_paging(&mut frame_allocator);
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
    println!("\nPANIC in {} at line {}...", location.file(), location.line());

    loop {}
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}