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
#![feature(extract_if)]

extern crate downcast_rs;
extern crate alloc;

use alloc::{format, vec};
use core::arch::asm;
use core::panic::PanicInfo;
use lazy_static::lazy_static;
use limine::BaseRevision;
use limine::memory_map::EntryType;
use limine::request::{FramebufferRequest, HhdmRequest, MemoryMapRequest};
use rlibc::memcpy;
use x86_64::registers::model_specific::Efer;
use x86_64::registers::control::{Cr0, Cr0Flags, EferFlags};
use crate::drivers::cpuid::CPU_INFO;
use crate::drivers::ps2::init_ps2_controller;
use crate::drivers::ps2::keyboard::PS2Keyboard;
use crate::drivers::ps2::PS2DeviceType;
use crate::fs::ext2::mount_filesystem;
use drivers::fbdev::FrameBufferDevice;
use crate::fs::Vfs;
use crate::graphics::fonts::{FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::framebuffer_device::Writer;
use crate::interrupts::{INTERRUPT_CONTROLLER, InterruptController};
use crate::memory::{MemoryManager, VirtualAddress};
use crate::task::keyboard::print_key_inputs;
use crate::task::executor::Executor;
use crate::task::Task;

#[macro_use]
mod graphics;
mod arch;
mod memory;
mod interrupts;
mod utils;
mod drivers;
mod task;
mod fs;
mod serial;

pub const KERNEL_START_VMA_ADDRESS: VirtualAddress = 0xFFFFFFFF80000000;

lazy_static! {
    pub static ref HHDM_OFFSET: VirtualAddress = HHDM_REQUEST.get_response().expect("could not retrieve the HHDM info").offset() as usize;
}

pub static BASE_REVISION: BaseRevision = BaseRevision::new();

pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
unsafe extern fn _start() {
    assert!(BASE_REVISION.is_supported());

    init();

    hcf();
}

unsafe fn init() {
    MemoryManager::init(MEMORY_MAP_REQUEST.get_response().expect("could not retrieve the kernel address"));
    Writer::init();

    info!("Toast version v0.0.1-x86_64");
    unsafe { CPU_INFO.lock().print_brand(); }

    unsafe {
        Efer::write(EferFlags::NO_EXECUTE_ENABLE);
        Cr0::write(Cr0::read() | Cr0Flags::WRITE_PROTECT);
    }

    InterruptController::init();
    //GlobalDescriptorTable::init();

    // init_acpi(boot_info); // TODO: This broke at some point, fix it

    let mut ahci_devices = drivers::pci::ahci::init();
    let fs = mount_filesystem(&mut ahci_devices[0]);

    let file_name = "/files/file.txt";
    println!("Reading file {}...", file_name);
    let file = fs.get_file_contents(&mut ahci_devices[0], file_name).unwrap_or_else(|| panic!("could not find the file {}", file_name));
    let string_content = core::str::from_utf8(file.as_slice()).expect("Failed to read file");
    println!("{}", string_content);

    let ps2_devices = init_ps2_controller();
    let mut executor = Executor::new();
    if ps2_devices.0.is_some() {
        let device = ps2_devices.0.unwrap();
        if let PS2DeviceType::MF2Keyboard = device.device_type() {
            let keyboard: PS2Keyboard = *device.downcast::<PS2Keyboard>().unwrap();
            executor.spawn(Task::new(print_key_inputs(keyboard)));
            INTERRUPT_CONTROLLER.lock().enable_keyboard_interrupts();
        }
    }

    Vfs::init();

    FRAMEBUFFER_REQUEST.get_response().expect("could not retrieve the frame buffer").framebuffers().for_each(|fbdev| {
        FrameBufferDevice::register(&fbdev, format!("/dev/fb{}", 0));
    });

    //print!(">");

    //executor.run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);

    loop {}
}

fn hcf() -> ! {
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}

fn print_memory_map() {
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