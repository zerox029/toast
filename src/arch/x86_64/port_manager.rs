use core::arch::asm;
use core::marker::PhantomData;

pub enum ReadWriteStatus {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

pub struct Port<T: InOut> {
    read_write_status: ReadWriteStatus,
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> Port<T> {
    pub fn new(port: u16, read_write_status: ReadWriteStatus) -> Port<T> {
        Port {
            read_write_status,
            port,
            phantom: PhantomData,
        }
    }

    pub fn read(&mut self) -> Result<T, &str> {
        match self.read_write_status {
            ReadWriteStatus::WriteOnly => Err("Tried to read from a write only port..."),
            _ => Ok(unsafe { T::port_in(self.port) })
        }
    }

    pub fn write(&mut self, value: T) -> Result<(), &str> {
        match self.read_write_status {
            ReadWriteStatus::ReadOnly => Err("Tried to write to a read only port..."),
            _ => Ok(unsafe { T::port_out(self.port, value) })
        }
    }
}

pub trait InOut{
    unsafe fn port_in(port: u16) -> Self;
    unsafe fn port_out(port: u16, value: Self);
}

impl InOut for u8 {
    unsafe fn port_in(port: u16) -> u8 { inb(port) }
    unsafe fn port_out(port: u16, value: u8) { outb(value, port); }
}
impl InOut for u16 {
    unsafe fn port_in(port: u16) -> u16 { inw(port) }
    unsafe fn port_out(port: u16, value: u16) { outw(value, port); }
}
impl InOut for u32 {
    unsafe fn port_in(port: u16) -> u32 { inl(port) }
    unsafe fn port_out(port: u16, value: u32) { outl(value, port); }
}

// Assembly wrappers
pub unsafe fn inb(port: u16) -> u8 {
    let result: u8;
    asm!("in al, dx", in("dx") port, out("al") result, options(nomem, nostack));

    result
}

pub unsafe fn outb(value: u8, port: u16) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
}

pub unsafe fn inw(port: u16) -> u16 {
    let result: u16;
    asm!("in ax, dx", in("dx") port, out("ax") result, options(nomem, nostack));

    result
}

pub unsafe fn outw(value: u16, port: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack));
}

pub unsafe fn inl(port: u16) -> u32 {
    let result: u32;
    asm!("in eax, dx", in("dx") port, out("eax") result, options(nomem, nostack));

    result
}

pub unsafe fn outl(value: u32, port: u16) {
    asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack));
}