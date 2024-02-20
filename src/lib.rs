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
#![feature(new_uninit)]
#![feature(str_from_raw_parts)]

extern crate downcast_rs;
extern crate alloc;
extern crate rlibc;

use core::panic::PanicInfo;
use x86_64::registers::model_specific::Efer;
use x86_64::registers::control::{Cr0, Cr0Flags, EferFlags};
use crate::cpuid::CPU_INFO;
use crate::interrupts::{INTERRUPT_CONTROLLER, InterruptController};
use crate::drivers::ps2::{init_ps2_controller};
use crate::drivers::ps2::keyboard::PS2Keyboard;
use crate::drivers::ps2::PS2DeviceType::*;
use crate::fs::ext2::mount_filesystem;
use crate::memory::MemoryManagementUnit;
use crate::task::executor::{Executor};
use crate::task::keyboard::print_key_inputs;
use crate::task::Task;

mod vga_buffer;
mod arch;
mod memory;
mod interrupts;
mod acpi;
mod utils;
mod drivers;
mod cpuid;
mod task;
mod fs;
mod serial;

#[no_mangle]
pub extern fn _main(multiboot_information_address: usize) {
    init(multiboot_information_address);
}

fn init(multiboot_information_address: usize) {
    vga_buffer::clear_screen();

    info_println!("Toast version v0.0.1-x86_64");
    unsafe { CPU_INFO.lock().print_brand(); }

    let boot_info = unsafe{ arch::multiboot2::load(multiboot_information_address) };

    unsafe {
        Efer::write(EferFlags::NO_EXECUTE_ENABLE);
        Cr0::write(Cr0::read() | Cr0Flags::WRITE_PROTECT);
    }

    let mut mmu = MemoryManagementUnit::new(boot_info);
    InterruptController::init_interrupts();
    //init_acpi(boot_info, &mut allocator, &mut active_page_table); // TODO: Fix this

    let mut ahci_devices = drivers::pci::ahci::init(&mut mmu);
    let fs = mount_filesystem(&mut mmu, &mut ahci_devices[0]);

    let file = fs.get_file_contents(&mut mmu, &mut ahci_devices[0], "/files/file.txt").unwrap();
    let string_content = core::str::from_utf8(file.as_slice()).expect("Failed to read file");

    println!("Reading file /files/file.txt...");
    println!("{}", string_content);

    let ps2_devices = init_ps2_controller();
    let mut executor = Executor::new();
    if ps2_devices.0.is_some() {
        let device = ps2_devices.0.unwrap();
        if let MF2Keyboard = device.device_type() {
            let keyboard: PS2Keyboard = *device.downcast::<PS2Keyboard>().unwrap();
            executor.spawn(Task::new(print_key_inputs(keyboard)));
            INTERRUPT_CONTROLLER.lock().enable_keyboard_interrupts();
        }
    }

    print!(">");

    executor.run();
}

fn print_memory_areas(multiboot_information_address: usize) {
    let boot_info = unsafe{ arch::multiboot2::load(multiboot_information_address) };
    let memory_map = boot_info.memory_map().expect("Memory map tag required");
    let elf_symbols = boot_info.elf_symbols().expect("Elf symbols tag required");

    serial_println!("Memory areas:");
    for entry in memory_map.entries() {
        serial_println!("    start: 0x{:x}, length: 0x{:x}",
                 entry.base_addr, entry.size);
    }

    serial_println!("kernel sections:");
    for section in elf_symbols.section_headers() {
        serial_println!("    addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
                 section.start_address(), section.size(), section.flags());
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error_println!("{}", info);

    loop {}
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}