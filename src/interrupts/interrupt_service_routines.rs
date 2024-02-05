use core::arch::asm;
use crate::{println, print};

/*
#[cfg(feature = "abi_x86_interrupt")]
pub type HandlerFuncWithErrCode = extern "x86-interrupt" fn(InterruptStackFrame, error_code: u64);
*/

pub extern "x86-interrupt" fn general_handler() {
    println!("Hello from the general exception handler");
    unsafe {
        asm! {
        "cli; hlt"
        };
    }
}