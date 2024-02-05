use core::arch::asm;
use crate::interrupts::interrupt_descriptor_table::*;
use crate::interrupts::interrupt_service_routines::*;

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
    map_handlers();
}

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