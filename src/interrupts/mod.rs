use core::arch::asm;
use core::sync::atomic::{compiler_fence, Ordering};
use spin::Mutex;
use crate::arch::x86_64::port_manager::{io_wait, Port};
use crate::arch::x86_64::port_manager::ReadWriteStatus::{ReadWrite, WriteOnly};
use crate::interrupts::interrupt_descriptor_table::*;
use crate::interrupts::interrupt_service_routines::*;
use crate::{println, print};

mod interrupt_descriptor_table;
mod interrupt_service_routines;
pub mod global_descriptor_table;

const MASTER_PIC_COMMAND_ADDRESS: u16 = 0x20;
const MASTER_PIC_DATA_ADDRESS: u16 = 0x21;
const SLAVE_PIC_COMMAND_ADDRESS: u16 = 0xA0;
const SLAVE_PIC_DATA_ADDRESS: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

static MASTER_PIC_COMMAND_PORT: Mutex<Port<u8>> = Mutex::new(Port::new(MASTER_PIC_COMMAND_ADDRESS, WriteOnly));
static MASTER_PIC_DATA_PORT: Mutex<Port<u8>> = Mutex::new(Port::new(MASTER_PIC_DATA_ADDRESS, ReadWrite));
static SLAVE_PIC_COMMAND_PORT: Mutex<Port<u8>> = Mutex::new(Port::new(SLAVE_PIC_COMMAND_ADDRESS, WriteOnly));
static SLAVE_PIC_DATA_PORT: Mutex<Port<u8>> = Mutex::new(Port::new(SLAVE_PIC_DATA_ADDRESS, ReadWrite));

pub static INTERRUPT_CONTROLLER: Mutex<InterruptController> = Mutex::new(InterruptController {
    master_pic_mask: 0xFF,
    slave_pic_mask: 0xFF,
});

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

pub struct InterruptController {
    master_pic_mask: u8,
    slave_pic_mask: u8,
}

impl InterruptController {
    pub fn init_interrupts() {
        Self::init_idt();
        Self::map_handlers();
        Self::remap_pic(0x20, 0x28);

        Self::set_irq_masks(0xFF, 0xFF);

        Self::enable_external_interrupts()
    }

    pub fn enable_keyboard_interrupts(&mut self) {
        println!("ps2: enabling keyboard input");
        self.master_pic_mask &= 0b11111101;
        Self::set_irq_masks(self.master_pic_mask, self.slave_pic_mask);
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
        IDT.set_entry(IdtVector::DivisionError, GateDescriptor::new(division_error_handler as usize));
        IDT.set_entry(IdtVector::Debug, GateDescriptor::new(breakpoint_handler as usize));
        IDT.set_entry(IdtVector::NonMaskableInterrupt, GateDescriptor::new(breakpoint_handler as usize));
        IDT.set_entry(IdtVector::Breakpoint, GateDescriptor::new(breakpoint_handler as usize));
        IDT.set_entry(IdtVector::Overflow, GateDescriptor::new(overflow_handler as usize));
        IDT.set_entry(IdtVector::BoundRangeExceeded, GateDescriptor::new(bound_range_exceeded_handler as usize));
        IDT.set_entry(IdtVector::InvalidOpcode, GateDescriptor::new(invalid_opcode_handler as usize));
        IDT.set_entry(IdtVector::DeviceNotAvailable, GateDescriptor::new(device_not_available_handler as usize));
        IDT.set_entry(IdtVector::DoubleFault, GateDescriptor::new(double_fault_handler as usize));
        IDT.set_entry(IdtVector::InvalidTSS, GateDescriptor::new(invalid_tss_handler as usize));
        IDT.set_entry(IdtVector::SegmentNotPresent, GateDescriptor::new(segment_not_present_handler as usize));
        IDT.set_entry(IdtVector::StackSegmentFault, GateDescriptor::new(stack_segment_fault_handler as usize));
        IDT.set_entry(IdtVector::GeneralProtectionFault, GateDescriptor::new(general_protection_fault_handler as usize));
        IDT.set_entry(IdtVector::PageFault, GateDescriptor::new(page_fault_handler as usize));
        IDT.set_entry(IdtVector::X87FloatingPointException, GateDescriptor::new(x87_floating_point_exception_handler as usize));
        IDT.set_entry(IdtVector::AlignmentCheck, GateDescriptor::new(alignment_check_handler as usize));
        IDT.set_entry(IdtVector::MachineCheck, GateDescriptor::new(machine_check_handler as usize));
        IDT.set_entry(IdtVector::SIMDFloatingPointException, GateDescriptor::new(simd_floating_point_exception_handler as usize));
        IDT.set_entry(IdtVector::VirtualizationException, GateDescriptor::new(virtualization_exception_handler as usize));
        IDT.set_entry(IdtVector::ControlProtectionException, GateDescriptor::new(control_protection_exception_handler as usize));
        IDT.set_entry(IdtVector::HypervisorInjectionException, GateDescriptor::new(hypervisor_injection_exception_handler as usize));
        IDT.set_entry(IdtVector::VMMCommunicationException, GateDescriptor::new(vmm_communication_exception_handler as usize));
        IDT.set_entry(IdtVector::SecurityException, GateDescriptor::new(security_exception_handler as usize));

        IDT.set_irq_entry(0x20, GateDescriptor::new(irq0_handler as usize));
        IDT.set_irq_entry(0x21, GateDescriptor::new(irq1_handler as usize));
        IDT.set_irq_entry(0x22, GateDescriptor::new(irq2_handler as usize));
        IDT.set_irq_entry(0x23, GateDescriptor::new(irq3_handler as usize));
        IDT.set_irq_entry(0x24, GateDescriptor::new(irq4_handler as usize));
        IDT.set_irq_entry(0x25, GateDescriptor::new(irq5_handler as usize));
        IDT.set_irq_entry(0x26, GateDescriptor::new(irq6_handler as usize));
        IDT.set_irq_entry(0x27, GateDescriptor::new(irq7_handler as usize));
    }

    fn remap_pic(offset_one: u8, offset_two: u8) {
        const ICW1_ICW4: u8 = 0x01;
        const ICW1_8086: u8 = 0x01;
        const ICW1_INIT: u8 = 0x10;

        let master_pic_mask = MASTER_PIC_DATA_PORT.lock().read().unwrap();
        io_wait();
        let slave_pic_mask = SLAVE_PIC_DATA_PORT.lock().read().unwrap();
        io_wait();

        // Start initialization sequence
        MASTER_PIC_COMMAND_PORT.lock().write(ICW1_INIT | ICW1_ICW4).unwrap();
        io_wait();
        SLAVE_PIC_COMMAND_PORT.lock().write(ICW1_INIT | ICW1_ICW4).unwrap();
        io_wait();

        // PIC vector offset
        MASTER_PIC_DATA_PORT.lock().write(offset_one).unwrap();
        io_wait();
        SLAVE_PIC_DATA_PORT.lock().write(offset_two).unwrap();
        io_wait();

        // Tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
        MASTER_PIC_DATA_PORT.lock().write(4).unwrap();
        io_wait();

        // Tell Slave PIC its cascade identity (0000 0010)
        SLAVE_PIC_DATA_PORT.lock().write(2).unwrap();
        io_wait();

        // Have the PICs use 8086 mode (and not 8080 mode)
        MASTER_PIC_DATA_PORT.lock().write(0x01).unwrap();
        io_wait();
        SLAVE_PIC_DATA_PORT.lock().write(0x01).unwrap();
        io_wait();

        // Restore the saved masks
        MASTER_PIC_DATA_PORT.lock().write(master_pic_mask).unwrap();
        SLAVE_PIC_DATA_PORT.lock().write(slave_pic_mask).unwrap();
    }

    fn set_irq_masks(master_mask: u8, slave_mask: u8) {
        MASTER_PIC_DATA_PORT.lock().write(master_mask).unwrap();
        SLAVE_PIC_DATA_PORT.lock().write(slave_mask).unwrap();
    }

    pub fn enable_external_interrupts() {
        compiler_fence(Ordering::Acquire);
        unsafe { asm!("sti"); }
    }

    pub fn enable_external_interrupts_and_hlt() {
        compiler_fence(Ordering::Acquire);
        unsafe { asm!("sti; hlt;"); }
    }

    pub fn disable_external_interrupts() {
        compiler_fence(Ordering::Acquire);
        unsafe { asm!("cli"); }
    }
}
