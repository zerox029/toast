use core::arch::asm;
use crate::arch::x86_64::port_manager::Port;
use crate::interrupts::interrupt_descriptor_table::*;
use crate::interrupts::interrupt_service_routines::*;

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
}

fn remap_pic(offset_one: u8, offset_two: u8) {
    let mut master_pic: Port<u8> = Port::new(MASTER_PIC_DATA);
    let mut slave_pic: Port<u8> = Port::new(SLAVE_PIC_DATA);

    let master_pic_mask = master_pic.read();
    let slave_pic_mask = slave_pic.read();

    // Start initialization sequence
    master_pic.write(ICW1_INIT | ICW1_ICW4);
    slave_pic.write(ICW1_INIT | ICW1_ICW4);

    // PIC vector offset
    master_pic.write(offset_one);
    slave_pic.write(offset_two);

    // Tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
    master_pic.write(4);

    // Tell Slave PIC its cascade identity (0000 0010)
    slave_pic.write(2);

    // Have the PICs use 8086 mode (and not 8080 mode)
    master_pic.write(ICW1_8086);
    slave_pic.write(ICW1_8086);

    // Restore the saved masks
    master_pic.write(master_pic_mask);
    slave_pic.write(slave_pic_mask);
}