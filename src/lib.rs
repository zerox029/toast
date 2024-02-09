#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(internal_features)]
#![feature(lang_items)]
#![feature(ptr_internals)]
#![feature(panic_info_message)]
#![feature(allocator_api)]
#![feature(const_mut_refs)]
#![feature(abi_x86_interrupt)]
#![feature(never_type)]

extern crate downcast_rs;
extern crate alloc;
extern crate rlibc;

use core::panic::PanicInfo;
use x86_64::registers::model_specific::Efer;
use x86_64::registers::control::{Cr0, Cr0Flags, EferFlags};
use crate::acpi::init_acpi;
use crate::cpuid::CPUInfo;
use crate::interrupts::{INTERRUPT_CONTROLLER, InterruptController};
use crate::memory::init_memory_modules;
use crate::drivers::ps2::{init_ps2_controller};
use crate::drivers::ps2::keyboard::PS2Keyboard;
use crate::drivers::ps2::PS2DeviceType::*;

pub mod vga_buffer;
pub mod arch;
pub mod memory;
mod test_runner;
mod interrupts;
mod acpi;
mod utils;
mod drivers;
mod cpuid;

#[no_mangle]
pub extern fn _main(multiboot_information_address: usize) {
    init(multiboot_information_address);

    loop {}
}

fn init(multiboot_information_address: usize) {
    vga_buffer::clear_screen();

    println!("Toast version v0.0.1-x86_64");

    let cpu_info = CPUInfo::get_current_cpu_info();

    let boot_info = unsafe{ arch::multiboot2::load(multiboot_information_address) };

    unsafe {
        Efer::write(EferFlags::NO_EXECUTE_ENABLE);
        Cr0::write(Cr0::read() | Cr0Flags::WRITE_PROTECT);
    }

    let (mut allocator, mut active_page_table) = init_memory_modules(boot_info);
    InterruptController::init_interrupts();
    init_acpi(boot_info, &mut allocator, &mut active_page_table);

    let ps2_devices = init_ps2_controller();
    print!(">");
    if ps2_devices.0.is_some() {
        let device = ps2_devices.0.unwrap();
        match device.device_type() {
            MF2Keyboard => {
                INTERRUPT_CONTROLLER.lock().enable_keyboard_interrupts(*device.downcast::<PS2Keyboard>().unwrap())
            },
            _ => ()
        }
    }
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
    println!("{}", info);

    loop {}
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}