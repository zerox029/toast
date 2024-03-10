use crate::arch::x86_64::port_manager::Port;
use crate::arch::x86_64::port_manager::ReadWriteStatus::WriteOnly;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failure = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    let mut port = Port::new(0xF4, WriteOnly);
    port.write(exit_code as u32).unwrap();
}

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T where T: Fn() {
    fn run(&self) {
        serial_println!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}