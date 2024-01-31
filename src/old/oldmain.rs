#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(toast::test_runner)]
#![reexport_test_harness_main = "test_main"]

use toast::println;
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    #[cfg(test)]
    test_main();

    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
mod tests {
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        toast::test_panic_handler(info)
    }

    #[test_case]
    fn trivial_assertion() {
        assert_eq!(1, 1);
    }
}