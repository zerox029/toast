use spin::Mutex;

pub static IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub const IDT_MAX_DESCRIPTOR: usize = 256;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GateDescriptor {
    pub offset_low: u16,        // The lower 16 bits of the ISR's address
    selector: u16,          // The GDT segment selector that the CPU will load into CS before calling the ISR
    ist: u8,                // The IST in the TSS that the CPU will load into RSP; set to zero for now
    type_attributes: u8,    // Type and attributes; see the IDT page
    pub offset_mid: u16,        // The higher 16 bits of the lower 32 bits of the ISR's address
    pub offset_high: u32,       // The higher 32 bits of the ISR's address
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

    pub fn new(offset: u64, selector: u16, gate_type: GateType, DPL: u8) -> Self {

        Self {
            offset_low: offset as u16,
            selector,
            ist: 0x8,
            type_attributes: (gate_type as u8 & 0b00001111) | (DPL & 0b01100000) | 0b10000000,
            offset_mid: (offset >> 16) as u16,
            offset_high: (offset >> 32) as u32,
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

    pub fn set_entry(&self, index: usize, entry: GateDescriptor) {
        let mut entries = self.entries.lock();
        entries[index] = entry;
    }

    pub fn get_address(&self) -> u64 {
        self.entries.lock().as_ptr() as u64
    }
}