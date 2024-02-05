use core::arch::asm;
use spin::Mutex;

pub static IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub const IDT_MAX_DESCRIPTOR: usize = 256;

pub enum IdtVector {
    DivisionError = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRangeExceeded = 5,
    InvalidOpcode = 6,
    DeviceNotAvailable = 7,
    DoubleFault = 8,
    CoprocessorSegmentOverrun = 9,
    InvalidTSS = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    PageFault = 14,
    // 15 is reserved
    X87FloatingPointException = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SIMDFloatingPointException = 19,
    VirtualizationException = 20,
    ControlProtectionException = 21,
    // 22-27 are reserved
    HypervisorInjectionException = 28,
    VMMCommunicationException = 29,
    SecurityException = 30,
    // 31 is reserved
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GateDescriptor {
    pub offset_low: u16,    // The lower 16 bits of the ISR's address
    selector: u16,          // The GDT segment selector that the CPU will load into CS before calling the ISR
    ist: u8,                // The IST in the TSS that the CPU will load into RSP; set to zero for now
    type_attributes: u8,    // Type and attributes; see the IDT page
    pub offset_mid: u16,    // The higher 16 bits of the lower 32 bits of the ISR's address
    pub offset_high: u32,   // The higher 32 bits of the ISR's address
    _reserved: u32,         // Set to zero
}

#[repr(C)]
pub struct InterruptDescriptorTable {
    entries: Mutex<[GateDescriptor; IDT_MAX_DESCRIPTOR]>,
}

#[repr(u8)]
pub enum GateType {
    InterruptGate = 0xE,
    TrapGate = 0xF,
}

impl GateDescriptor {
    const fn zero() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attributes: 0,
            offset_mid: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    pub fn new(handler_address: u64) -> Self {
        let segment: u16;
        unsafe { asm!("mov {0:x}, cs", out(reg) segment, options(nostack, nomem)) };

        let dpl = 0;

        Self {
            offset_low: handler_address as u16,
            selector: segment,
            ist: 0x8,
            type_attributes: (GateType::InterruptGate as u8 & 0b00001111) | (dpl & 0b01100000) | 0b10000000,
            offset_mid: (handler_address >> 16) as u16,
            offset_high: (handler_address >> 32) as u32,
            _reserved: 0,
        }
    }
}

impl InterruptDescriptorTable {
    const fn new() -> Self {
        Self {
            entries: Mutex::new([GateDescriptor::zero(); IDT_MAX_DESCRIPTOR]),
        }
    }

    pub fn set_entry(&self, vector: IdtVector, entry: GateDescriptor) {
        let mut entries = self.entries.lock();
        entries[vector as usize] = entry;
    }

    pub fn get_address(&self) -> u64 {
        self.entries.lock().as_ptr() as u64
    }
}