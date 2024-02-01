#![allow(internal_features)]
#![allow(dead_code)]
#![feature(lang_items)]
#![feature(ptr_internals)]
#![no_std]

use core::panic::PanicInfo;

extern crate rlibc;

pub mod vga_buffer;

#[no_mangle]
pub extern fn _start() {
    println!("Test");
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}