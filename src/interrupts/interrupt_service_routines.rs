use core::arch::asm;
use core::fmt;
use core::fmt::Formatter;
use crate::{println, print};

pub type HandlerFuncWithoutErrCode = extern "x86-interrupt" fn(InterruptStackFrame);
pub type HandlerFuncWithErrCode = extern "x86-interrupt" fn(InterruptStackFrame, error_code: u64);

#[repr(C)]
pub struct InterruptStackFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

impl fmt::Debug for InterruptStackFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptStackFrame")
            .field("instruction_pointer", &format_args!("0x{:X}", self.instruction_pointer))
            .field("code_segment", &format_args!("0x{:X}", self.code_segment))
            .field("cpu_flags", &format_args!("{:b}", self.cpu_flags))
            .field("stack_pointer", &format_args!("0x{:X}", self.stack_pointer))
            .field("stack_segment", &format_args!("0x{:X}", self.stack_segment))
            .finish()
    }
}

pub extern "x86-interrupt" fn division_error_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a division error interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a debug interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a non-maskable interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a breakpoint interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    println!("Caught an overflow interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a bound range exceeded interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    println!("Caught an invalid opcode interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a device not available interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a double fault! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught an invalid tss interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a segment not present interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a stack segment fault interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a general protection fault interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a page fault interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
    unsafe { asm!("hlt;"); };
}

pub extern "x86-interrupt" fn x87_floating_point_exception_handler(stack_frame: InterruptStackFrame) {
    println!("Caught an x86 floating point exception interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught an alignment check interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a machine check interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn simd_floating_point_exception_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a SIMD floating point exception interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn virtualization_exception_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a virtualization exception interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn control_protection_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a control protection exception interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn hypervisor_injection_exception_handler(stack_frame: InterruptStackFrame) {
    println!("Caught a hypervisor injection exception interrupt!");
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn vmm_communication_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a VMM communication exception interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn security_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("Caught a security exception interrupt! Error code 0x{:X}", error_code);
    println!("{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn default_irq_handler(stack_frame: InterruptStackFrame) {
    println!("Caught an IRQ!");
    println!("{:#?}", stack_frame);
}