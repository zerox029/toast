use core::ops::DerefMut;
use crate::{println, print};
use crate::drivers::pci::{find_all_pci_devices, PCIDevice};
use crate::memory::Frame;
use crate::memory::page_frame_allocator::PageFrameAllocator;
use crate::memory::paging::{ActivePageTable};
use crate::memory::paging::entry::EntryFlags;

enum FisType {
    RegH2D      = 0x27, // Register FIS - host to device
    RegD2H      = 0x34, // Register FIS - device to host
    DmaAct      = 0x39, // DMA activate FIS - device to host
    DmaSetup    = 0x41, // DMA setup FIS - bidirectional
    Data        = 0x46, // Data FIS - bidirectional
    Bist        = 0x58, // BIST activate FIS - bidirectional
    PioSetup    = 0x5F, // PIO setup FIS - device to host
    DevBits     = 0xA1, // Set device bits FIS - device to host
}

struct FisRegH2D {
    fis_type: u8,   // FIS_TYPE_REG_H2D

    pmport: u8,     // Port multiplier
    rsv0: u8,       // Reserved
    c: u8,          // 1: Command, 0: Control

    command: u8,    // Command register
    feature1: u8,   // Feature register, 7:0

    lba0: u8,       // LBA low register, 7:0
    lba1: u8,       // LBA mid register, 15:8
    lba2: u8,       // LBA high register, 23:16
    device: u8,     // Device register

    lba3: u8,       // LBA register, 31:24
    lba4: u8,       // LBA register, 39:32
    lba5: u8,       // LBA register, 47:40
    featureh: u8,   // Feature register, 15:8

    countl: u8,     // Count register, 7:0
    counth: u8,     // Count register, 15:8
    icc: u8,        // Isochronous command completion
    control: u8,    // Control register

    rsv1: [u8; 4],  // Reserved
}

struct FisRegD2H {
    fis_type: u8,   // FIS_TYPE_REG_D2H

    pmport: u8,     // Port multiplier
    rsv0: u8,       // Reserved
    i: u8,          // Interrupt bit
    rsv1: u8,       // Reserved

    status: u8,    // Status register
    error: u8,     // Error register

    lba0: u8,       // LBA low register, 7:0
    lba1: u8,       // LBA mid register, 15:8
    lba2: u8,       // LBA high register, 23:16
    device: u8,     // Device register

    lba3: u8,       // LBA register, 31:24
    lba4: u8,       // LBA register, 39:32
    lba5: u8,       // LBA register, 47:40
    rsv2: u8,       // Reserved

    countl: u8,     // Count register, 7:0
    counth: u8,     // Count register, 15:8
    rsv3: [u8; 2],  // Reserved

    rsv4: [u8; 4],  // Reserved
}

struct FisData {
    fis_type: u8,   // FIS_TYPE_DATA

    pmport: u8,     // Port multiplier
    rsv0: u8,       // Reserved

    rsv1: [u8; 2],  // Reserved

    data: [u32; 1], // Payload
}

struct FisPioSetup {
    fis_type: u8,   // FIS_TYPE_REG_PIO_SETUP

    pmport: u8,     // Port multiplier
    rsv0: u8,       // Reserved
    d: u8,          // Data transfer direction, 1 - device to host
    i: u8,          // Interrupt bit

    status: u8,    // Status register
    error: u8,     // Error register

    lba0: u8,       // LBA low register, 7:0
    lba1: u8,       // LBA mid register, 15:8
    lba2: u8,       // LBA high register, 23:16
    device: u8,     // Device register

    lba3: u8,       // LBA register, 31:24
    lba4: u8,       // LBA register, 39:32
    lba5: u8,       // LBA register, 47:40
    rsv2: u8,       // Reserved

    countl: u8,     // Count register, 7:0
    counth: u8,     // Count register, 15:8
    rsv3: [u8; 2],  // Reserved
    e_status: u8,   // New value of status register

    tc: u16,        // Transfer count
    rsv4: [u8; 4],  // Reserved
}

struct FisDmaSetup {
    fis_type: u8,       // FIS_TYPE_REG_DMA_SETUP

    pmport: u8,         // Port multiplier
    rsv0: u8,           // Reserved
    d: u8,              // Data transfer direction, 1 - device to host
    i: u8,              // Interrupt bit
    a: u8,              // Auto-activate. Specifies if DMA Activate FIS is needed

    rsvd: [u8; 2],      // Reserved

    dma_buffer_id: u64, // DMA Buffer Identifier. Used to Identify DMA buffer in host memory.
                        // SATA Spec says host specific and not in Spec. Trying AHCI spec might work.

    rsvd2: u32,         // Reserved,

    dma_buf_offset: u32,// Byte offset into buffer. First 2 bits must be 0

    transfer_count: u32,// Number of bytes to transfer. Bit 0 must be 0

    rsvd3: u32,         // Reserved
}

#[derive(Debug)]
struct HbaMemoryRegisters {
    test: u128,
    test2: u128
}

pub fn init(allocator: &mut PageFrameAllocator, active_page_table: &mut ActivePageTable) {
    println!("sata: init...");

    let ahci_controller = find_all_pci_devices().into_iter().find(is_ahci_controller).expect("Could not locate the AHCI controller");

    // Enable interrupts, DMA, and memory space access in the PCI command register
    let mut updated_command = ahci_controller.command(0) | 0x2;
    updated_command &= 0b1111101111111111;
    ahci_controller.set_command(0, updated_command);

    // Memory map BAR 5 register as uncacheable.
    let base_memory = ahci_controller.bar5(0);
    active_page_table.deref_mut().identity_map(Frame::containing_address(base_memory as usize), EntryFlags::NO_CACHE, allocator);

    // Perform BIOS/OS handoff (if the bit in the extended capabilities is set)
    let hba = unsafe { &*(base_memory as *const HbaMemoryRegisters) };
    println!("{:b}", hba.test);
}

fn is_ahci_controller(device: &PCIDevice) -> bool {
    device.class_code(0) == 0x01 && ((device.subclass(0) == 0x06) | (device.subclass(0) == 0x01))
}