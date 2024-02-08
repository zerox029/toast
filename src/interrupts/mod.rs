use core::arch::asm;
use crate::arch::x86_64::port_manager::{io_wait, Port};
use crate::arch::x86_64::port_manager::ReadWriteStatus::{ReadOnly, ReadWrite};
use crate::interrupts::interrupt_descriptor_table::*;
use crate::interrupts::interrupt_service_routines::*;
use crate::{println, print};

mod interrupt_descriptor_table;
mod interrupt_service_routines;

const MASTER_PIC_COMMAND: u16 = 0x20;
const MASTER_PIC_DATA: u16 = 0x21;
const SLAVE_PIC_COMMAND: u16 = 0xA0;
const SLAVE_PIC_DATA: u16 = 0xA1;

const ICW1_ICW4: u8 = 0x01;
const ICW1_8086: u8 = 0x01;
const ICW1_INIT: u8 = 0x10;


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
    map_handlers();
    remap_pic(0x20, 0x28);

    unsafe {
        asm!("sti;");
    }

}

// Create the IDT and tell the CPU where to find it
fn init_idt() {
    let idtr = InterruptDescriptorTableRegister {
        limit: 256 * 8 - 1,
        base: IDT.get_address(),
    };

    unsafe { asm!("lidt [{}]", in(reg) &idtr) }
}

fn map_handlers() {
    IDT.set_entry(IdtVector::DivisionError, GateDescriptor::new(division_error_handler as u64));
    IDT.set_entry(IdtVector::Debug, GateDescriptor::new(breakpoint_handler as u64));
    IDT.set_entry(IdtVector::NonMaskableInterrupt, GateDescriptor::new(breakpoint_handler as u64));
    IDT.set_entry(IdtVector::Breakpoint, GateDescriptor::new(breakpoint_handler as u64));
    IDT.set_entry(IdtVector::Overflow, GateDescriptor::new(overflow_handler as u64));
    IDT.set_entry(IdtVector::BoundRangeExceeded, GateDescriptor::new(bound_range_exceeded_handler as u64));
    IDT.set_entry(IdtVector::InvalidOpcode, GateDescriptor::new(invalid_opcode_handler as u64));
    IDT.set_entry(IdtVector::DeviceNotAvailable, GateDescriptor::new(device_not_available_handler as u64));
    IDT.set_entry(IdtVector::DoubleFault, GateDescriptor::new(double_fault_handler as u64));
    IDT.set_entry(IdtVector::InvalidTSS, GateDescriptor::new(invalid_tss_handler as u64));
    IDT.set_entry(IdtVector::SegmentNotPresent, GateDescriptor::new(segment_not_present_handler as u64));
    IDT.set_entry(IdtVector::StackSegmentFault, GateDescriptor::new(stack_segment_fault_handler as u64));
    IDT.set_entry(IdtVector::GeneralProtectionFault, GateDescriptor::new(general_protection_fault_handler as u64));
    IDT.set_entry(IdtVector::PageFault, GateDescriptor::new(page_fault_handler as u64));
    IDT.set_entry(IdtVector::X87FloatingPointException, GateDescriptor::new(x87_floating_point_exception_handler as u64));
    IDT.set_entry(IdtVector::AlignmentCheck, GateDescriptor::new(alignment_check_handler as u64));
    IDT.set_entry(IdtVector::MachineCheck, GateDescriptor::new(machine_check_handler as u64));
    IDT.set_entry(IdtVector::SIMDFloatingPointException, GateDescriptor::new(simd_floating_point_exception_handler as u64));
    IDT.set_entry(IdtVector::VirtualizationException, GateDescriptor::new(virtualization_exception_handler as u64));
    IDT.set_entry(IdtVector::ControlProtectionException, GateDescriptor::new(control_protection_exception_handler as u64));
    IDT.set_entry(IdtVector::HypervisorInjectionException, GateDescriptor::new(hypervisor_injection_exception_handler as u64));
    IDT.set_entry(IdtVector::VMMCommunicationException, GateDescriptor::new(vmm_communication_exception_handler as u64));
    IDT.set_entry(IdtVector::SecurityException, GateDescriptor::new(security_exception_handler as u64));

    for i in (31..255) {
        IDT.set_irq_entry(i, GateDescriptor::new(default_irq_handler as u64));
    }
}

fn remap_pic(offset_one: u8, offset_two: u8) {
    let mut master_pic_data: Port<u8> = Port::new(MASTER_PIC_DATA, ReadWrite);
    let mut slave_pic_data: Port<u8> = Port::new(SLAVE_PIC_DATA, ReadWrite);
    let mut master_pic_command: Port<u8> = Port::new(MASTER_PIC_COMMAND, ReadWrite);
    let mut slave_pic_command: Port<u8> = Port::new(SLAVE_PIC_COMMAND, ReadWrite);

    let master_pic_mask = master_pic_data.read().unwrap();
    io_wait();
    let slave_pic_mask = slave_pic_data.read().unwrap();
    io_wait();

    // Start initialization sequence
    master_pic_command.write(ICW1_INIT | ICW1_ICW4).unwrap();
    io_wait();
    slave_pic_command.write(ICW1_INIT | ICW1_ICW4).unwrap();
    io_wait();

    // PIC vector offset
    master_pic_data.write(offset_one).unwrap();
    io_wait();
    slave_pic_data.write(offset_two).unwrap();
    io_wait();

    // Tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
    master_pic_data.write(4).unwrap();
    io_wait();

    // Tell Slave PIC its cascade identity (0000 0010)
    slave_pic_data.write(2).unwrap();
    io_wait();

    // Have the PICs use 8086 mode (and not 8080 mode)
    master_pic_data.write(0x01).unwrap();
    io_wait();
    slave_pic_data.write(0x01).unwrap();
    io_wait();

    // Restore the saved masks
    master_pic_data.write(master_pic_mask).unwrap();
    slave_pic_data.write(slave_pic_mask).unwrap();
}

fn irq_mask(irq_line: u8) {
    let port: u16;
    let value: u8;

    if irq_line < 8 {
        port = MASTER_PIC_COMMAND
    }
    else {
        port = SLAVE_PIC_DATA;
        let irq_line = irq_line - 8;
    }

    let mut pic: Port<u8> = Port::new(port, ReadWrite);

    value = pic.read().unwrap() | (1 << irq_line);
    pic.write(value).unwrap();
}