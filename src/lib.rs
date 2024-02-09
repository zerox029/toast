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

#[macro_use]
extern crate downcast_rs;
extern crate alloc;
extern crate rlibc;

use core::any::{Any, TypeId};
use core::mem;
use core::panic::PanicInfo;
use x86_64::registers::model_specific::Efer;
use x86_64::registers::control::{Cr0, Cr0Flags, EferFlags};
use crate::acpi::init_acpi;
use crate::interrupts::{InterruptController};
use crate::memory::init_memory_modules;
use crate::drivers::ps2::{init_ps2_controller, PS2ControllerDevices};
use crate::drivers::ps2::keyboard::PS2Keyboard;
use crate::drivers::ps2::PS2DeviceType::*;
use crate::drivers::ps2::PS2Device;

pub mod vga_buffer;
pub mod arch;
pub mod memory;
mod test_runner;
mod interrupts;
mod acpi;
mod utils;
mod drivers;

#[no_mangle]
pub extern fn _main(multiboot_information_address: usize) {
    init(multiboot_information_address);

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

    let (mut allocator, mut active_page_table) = init_memory_modules(boot_info);
    let mut interrupt_controller = InterruptController::init_interrupts();
    init_acpi(boot_info, &mut allocator, &mut active_page_table);

    let ps2_devices = init_ps2_controller();
    if ps2_devices.0.is_some() {
        let device = ps2_devices.0.unwrap();
        match device.device_type() {
            MF2Keyboard => {
                interrupt_controller.enable_keyboard_interrupts(*device.downcast::<PS2Keyboard>().unwrap())
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