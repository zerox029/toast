use core::arch::asm;
use x86_64::registers::segmentation::CS;
use x86_64::structures::gdt::SegmentSelector;
use crate::interrupts::interrupt_descriptor_table::*;
use crate::interrupts::interrupt_service_routines::general_handler;
use crate::{println, print};

mod interrupt_descriptor_table;
mod interrupt_service_routines;

#[repr(C, packed)]
pub struct InterruptDescriptorTableRegister {
    pub limit: u16,
    pub base: u64,
}

impl InterruptDescriptorTableRegister {
    fn limit(&self) -> u16 {
        self.limit
    }

    fn base(&self) -> u64 {
        self.base
    }
}

pub fn init_interrupts() {
    init_idt();
}

fn init_idt() {
    let idtr = InterruptDescriptorTableRegister {
        limit: 256 * 8 - 1,
        base: IDT.get_address(),
    };

    unsafe { asm!("lidt [{}]", in(reg) &idtr) }
}

fn map_handlers() {
    let segment: u16;
    unsafe { asm!("mov {0:x}, cs", out(reg) segment, options(nostack, nomem)) };

    IDT.set_entry(0x0, GateDescriptor::new(general_handler as u64, segment, GateType::InterruptGate, 0));
    IDT.set_entry(0x3, GateDescriptor::new(general_handler as u64, segment, GateType::InterruptGate, 0));
}