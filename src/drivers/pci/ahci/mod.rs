// https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/serial-ata-ahci-spec-rev1-3-1.pdf
// http://www.usedsite.co.kr/pds/file/SerialATA_Revision_3_0_RC11.pdf

mod structures;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem::size_of;
use core::ops::DerefMut;
use core::ptr;
use crate::{println, print, panic};
use crate::drivers::pci::{find_all_pci_devices, PCIDevice};
use crate::memory::Frame;
use crate::memory::page_frame_allocator::PageFrameAllocator;
use crate::memory::paging::{ActivePageTable};
use crate::memory::paging::entry::EntryFlags;
use crate::utils::bitutils::is_nth_bit_set;

const SATA_SIG_ATA: u32     = 0x00000101;   // SATA drive
const SATA_SIG_ATAPI: u32   = 0xEB140101;   // SATAPI drive
const SATA_SIG_SEMB: u32    = 0xC33C0101;   // Enclosure management bridge
const SATA_SIG_PM: u32      = 0x96690101;    // Port multiplier

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

#[repr(C, packed)]
struct FisRegH2D {
    fis_type: u8,   // FIS_TYPE_REG_H2D

    flags: u8,     // Port multiplier

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

#[repr(C, packed)]
struct FisData {
    fis_type: u8,   // FIS_TYPE_DATA

    pmport: u8,     // Port multiplier
    rsv0: u8,       // Reserved

    rsv1: [u8; 2],  // Reserved

    data: [u32; 1], // Payload
}

#[repr(C, packed)]
struct FisDmaSetup {
    fis_type: u8,       // FIS_TYPE_REG_DMA_SETUP

    pmport_byte: u8,         // Port multiplier

    rsvd: u16,      // Reserved

    dma_buffer_id: u64, // DMA Buffer Identifier. Used to Identify DMA buffer in host memory.
    // SATA Spec says host specific and not in Spec. Trying AHCI spec might work.

    rsvd2: u32,         // Reserved,

    dma_buf_offset: u32,// Byte offset into buffer. First 2 bits must be 0

    transfer_count: u32,// Number of bytes to transfer. Bit 0 must be 0

    rsvd3: u32,         // Reserved
}

impl FisDmaSetup {
    pub fn pmport(&self) -> u8 {
        self.pmport_byte & 0b1111
    }

    /// Data transfer direction, 1 - device to host
    pub fn d(&self) -> bool {
        is_nth_bit_set(self.pmport_byte as usize, 5)
    }

    /// Interrupt bit
    pub fn i(&self) -> bool {
        is_nth_bit_set(self.pmport_byte as usize, 6)
    }

    /// Auto-activate. Specifies if DMA Activate FIS is needed
    pub fn a(&self) -> bool {
        is_nth_bit_set(self.pmport_byte as usize, 7)
    }
}

#[repr(C, packed)]
struct FisPioSetup {
    fis_type: u8,   // FIS_TYPE_REG_PIO_SETUP

    pmport: u8,     // Port multiplier

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
    rsv3: [u8; 1],  // Reserved
    e_status: u8,   // New value of status register

    tc: u16,        // Transfer count
    rsv4: u16,  // Reserved
}

#[repr(C)]
struct FisRegD2H {
    fis_type: u8,   // FIS_TYPE_REG_D2H

    pmport: u8,     // Port multiplier

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

#[repr(C, packed)]
struct FisSetDeviceBitsD2H {
    typ: u8,
    pmport: u8,
    status: u8,
    error: u8,
    protocol_specifi: u32,
}

#[repr(C, packed)]
struct ReceivedFis {
    dsfis: FisDmaSetup,
    rsv1: [u8; 0x20 - 0x1C],
    psfis: FisPioSetup,
    rsv2: [u8; 0x40 - 0x34],
    rfis: FisRegD2H,
    rsv3: u32,
    sdbfis: FisSetDeviceBitsD2H,
    ufis: [u8; 0xA0 - 0x60],
    rsv4: [u8; 0xFF - 0xA0],
}

#[repr(C)]
#[derive(Debug)]
struct HbaMemoryRegisters {
    // 0x00 - 0x2B, Generic Host Control
    cap: u32,
    ghc: u32,
    is: u32,
    pi: u32,
    vs: u32,
    ccc_ctl: u32,
    ccc_pts: u32,
    em_loc: u32,
    em_ctl: u32,
    cap2: u32,
    bohc: u32,

    // 0x2C - 0x9F, Reserved
    rsv: [u8; 0xA0-0x2C],

    // 0xA0 - 0xFF, Vendor specific registers
    vendor: [u8; 0x100-0xA1],
}

#[repr(C)]
#[derive(Debug)]
pub struct PortRegisters {
    clb: u32,
    clbu: u32,
    fb: u32,
    fbu: u32,
    is: u32,
    ie: u32,
    cmd: u32,

    rsv: u32,

    tfd: u32,
    sig: u32,
    ssts: u32,
    sctl: u32,
    serr: u32,
    sact: u32,
    ci: u32,
    sntf: u32,
    fbs: u32,
    devslp: u32,

    // 0x48 - 6F, Reserved
    rsv2: [u8; 0x70-0x48],

    // 0x70 - 7F, Vendor Specific
    vendor: [u8; 0x80-0x71],
}


type CommandList = [CommandHeader; 32];
#[repr(C)]
#[derive(Debug)]
struct CommandHeader {
    flags: u16,
    prdtl: u16,
    prdbc: u32,
    ctba: u32,
    ctbau: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Debug)]
struct CommandTable {
    cfis: [u8; 64], // Command FIS
    acmd: [u8; 16], // ATAPI command, 12 or 16 bytes
    rsv: [u8; 48],  // Reserved
    first_prdt_entry: PrdtEntry,
}

#[repr(C)]
#[derive(Debug)]
struct PrdtEntry {
    dba: u32,
    dbau: u32,
    reserved: u32,
    dbc: u32,
}


#[derive(Debug)]
struct AHCIController {
    pci_device: PCIDevice,

    bar5: u32,
    version_maj: u32,
    version_min: u32,
    port_count: u32,
    slot_count: u32,

    hba: &'static HbaMemoryRegisters,
}

impl AHCIController {
    fn new(allocator: &mut PageFrameAllocator, active_page_table: &mut ActivePageTable, pci_device: PCIDevice) -> Self {
        // Memory map HBA registers as uncacheable.
        let bar5 = pci_device.bar5(0);
        let start_frame = Frame::containing_address(bar5 as usize);
        let end_frame = Frame::containing_address(bar5 as usize + 0x10FF);
        for frame in Frame::range_inclusive(start_frame, end_frame) {
            active_page_table.deref_mut().identity_map(frame, EntryFlags::WRITABLE | EntryFlags::NO_CACHE, allocator);
        }

        let hba = unsafe { &*(bar5 as *mut HbaMemoryRegisters) };

        let version_maj = (hba.vs >> 16) & 0xFFFF;
        let version_min = hba.vs & 0xFFFF;
        let port_count = hba.cap & 0b11111;
        let slot_count = (hba.cap >> 8) & 0b11111;

        Self {
            pci_device,

            bar5,
            version_maj,
            version_min,
            port_count,
            slot_count,

            hba
        }
    }

    fn bios_os_handoff(&self) {
        if !is_nth_bit_set(self.hba.cap2 as usize, 0) {
            println!("ahci: bios/os handoff not supported");
            return;
        }

        // TODO

        /*
        let mut bohc_address = self.bar5 + 0x28;
        let bohc_pointer = bohc_address as *mut u32;

        unsafe { core::ptr::write(bohc_pointer, self.hba.bohc | 2) };*/
    }
}

#[derive(Debug)]
struct AHCIDevice {
    controller: *const AHCIController,
    port_index: usize,

    serial_number: [u8; 21],
    firmware_revision: [u8; 9],
    model_number: [u8; 41],

    port_registers: &'static mut PortRegisters,

    command_list: [AHCICommand; 32],
}

impl AHCIDevice {
    fn new(controller: *const AHCIController, port_index: usize, port_address: usize) -> Self {
        let mut port_registers = unsafe { &mut *(port_address as *mut PortRegisters) };

        Self {
            controller,
            port_index,

            serial_number: [0; 21],
            firmware_revision: [0; 9],
            model_number: [0; 41],

            port_registers,

            command_list: [AHCICommand::new(); 32],
        }
    }

    fn issue_identity(&mut self, identity: *mut u32) {
        let mut command = &mut self.command_list[self.allocate_slot()];

        command.data_base = identity;
        command.data_length = 511;
        command.interrupt = false;

        unsafe{ &mut *command.command_header }.flags = (size_of::<FisRegH2D>() / 4) as u16;
        unsafe{ &mut *command.command_header }.prdtl = 1;

        // init prdt
        let command_table = unsafe{ &mut *command.command_table };
        command_table.first_prdt_entry.dba = identity as u32;
        command_table.first_prdt_entry.dbau = (identity as u32 >> 32);
        command_table.first_prdt_entry.dbc = 511 | (0 << 31);

        let command_pointer = &mut command_table.cfis;
        command_pointer.fill(0);

        command_pointer[0] = 0x27;
        command_pointer[1] = (1 << 7);
        command_pointer[2] = 0xEC;

        // Issue command
    }

    fn allocate_slot(&mut self) -> usize {
        let controller = unsafe{ &*self.controller };
        let slot_count = controller.slot_count;

        for i in 0..slot_count {
            if self.port_registers.sact & (1 << i) == 0 && self.port_registers.ci & (1 << i) == 0 {
                let command_header_address = (self.port_registers.clb as usize | ((self.port_registers.clbu as usize) << 32)) + i as usize * size_of::<CommandHeader>();
                let command_header = unsafe { &*(command_header_address as *const CommandHeader )};

                let command_table_address = (command_header.ctba as usize | ((command_header.ctbau as usize) << 32)) + i as usize * size_of::<CommandTable>();

                self.command_list[i as usize].ahci_device = self as *mut AHCIDevice;
                self.command_list[i as usize].command_header = command_header_address as *mut CommandHeader;
                self.command_list[i as usize].command_table = command_table_address as *mut CommandTable;
                self.command_list[i as usize].slot = i;

                return i as usize
            }
        }

        panic!("ahci: unable to allocate command slot");
    }
}

#[derive(Debug, Copy, Clone)]
struct AHCICommand {
    command_header: *mut CommandHeader,
    command_table: *mut CommandTable,
    ahci_device: *mut AHCIDevice,

    data_base: *mut u32,
    data_length: usize,
    interrupt: bool,

    slot: u32,
}


impl AHCICommand {
    fn new() -> Self {
        Self {
            command_header: ptr::null_mut(),
            command_table: ptr::null_mut(),
            ahci_device: ptr::null_mut(),

            data_base: ptr::null_mut(),
            data_length: 0,
            interrupt: false,

            slot: 0,
        }
    }
}

pub fn init(allocator: &mut PageFrameAllocator, active_page_table: &mut ActivePageTable) {
    println!("ahci: init...");

    let ahci_pci_device = find_all_pci_devices().into_iter().find(is_ahci_controller).expect("ahci: could not locate the ahci controller");
    let ahci_controller = AHCIController::new(allocator, active_page_table, ahci_pci_device);

    println!("ahci: controller version {}.{}", ahci_controller.version_maj, ahci_controller.version_min);

    // Enable interrupts, DMA, and memory space access in the PCI command register
    let updated_command = (ahci_pci_device.command(0) | 0x2) & 0b1111101111111111;
    ahci_pci_device.set_command(0, updated_command);

    // Check if 64-bit DMA is supported
    if !is_nth_bit_set(ahci_controller.hba.cap as usize, 31) {
        panic!("ahci: controller not capable of 64 bit addressing... aborting")
    }

    ahci_controller.bios_os_handoff();

    // Initialize ports
    for port in 0..ahci_controller.port_count as usize {
        if is_nth_bit_set(ahci_controller.hba.pi as usize, port) {
            init_port(&ahci_controller, port, ahci_controller.bar5 as usize + 0x100 + port * 0x80);
        }
    }

    /*
    // Reset controller
    let mut ghc_address = base_memory + 0x4;
    let ghc_pointer = ghc_address as *mut u32;

    //unsafe { core::ptr::write(ghc_pointer, hba.ghc | 1) };

    // Register IRQ handler, using interrupt line given in the PCI register.
    println!("ahci: connected to IRQ{}", ahci_controller.interrupt_line(0));

    // Enable AHCI mode and interrupts in global host control register.
    unsafe { core::ptr::write(ghc_pointer, hba.ghc | 0x80000002) };
    */
}

fn init_port(controller: &AHCIController, port_index: usize, port_address: usize) {
    let mut ahci_device = AHCIDevice::new(controller as *const AHCIController, port_index, port_address);

    match ahci_device.port_registers.sig {
        SATA_SIG_ATA => println!("ahci: sata drive found on port {}", port_index),
        SATA_SIG_ATAPI => println!("ahci: satapi drive found on port {}", port_index),
        SATA_SIG_SEMB => println!("ahci: enclosure management bridge found on port {}", port_index),
        SATA_SIG_PM => println!("ahci: port multiplier found on port {}", port_index),
        _ => return
    }

    // TODO: Allocate somewhere else to map them as uncacheable
    // Allocate physical memory for the command list
    let mut command_list_base = Box::into_raw(Box::<CommandList>::new_uninit()) as usize;
    ahci_device.port_registers.clb = command_list_base as u32;
    ahci_device.port_registers.clbu = (command_list_base >> 32) as u32;

    // Allocate physical memory for the command tables
    for i in 0..32 {
        let header_address = command_list_base + i * core::mem::size_of::<CommandHeader>();
        let command_header = unsafe{ &mut *(header_address as *mut CommandHeader) };

        let command_table_base_address = Box::into_raw(Box::<CommandTable>::new_uninit()) as usize;

        command_header.ctba = command_table_base_address as u32;
        command_header.ctbau = (command_table_base_address >> 32) as u32;
    }

    // Allocate physical memory for the received FIS
    let mut command_list_base = Box::into_raw(Box::<ReceivedFis>::new_uninit()) as usize;
    ahci_device.port_registers.clb = command_list_base as u32;
    ahci_device.port_registers.clbu = (command_list_base >> 32) as u32;

    ahci_device.port_registers.cmd |= (1 << 0) | (1 << 4);

    ahci_device.issue_identity();


    // , the received FIS, and its command tables. Make sure the command tables are 128 byte aligned.
    // Memory map these as uncacheable.

/*
    let command_list = unsafe { &*(command_list_address as *const CommandList) };
    command_list.iter().for_each(|command_header| {
        let command_table_address = (command_header.dw2 as u64) | ((command_header.dw3 as u64) << 32);
        active_page_table.deref_mut().identity_map_if_unmapped(Frame::containing_address(command_table_address as usize), EntryFlags::WRITABLE | EntryFlags::NO_CACHE, allocator);
    });

    let fis_address = (port_registers.fb as u64) | ((port_registers.fbu as u64) << 32);
    active_page_table.deref_mut().identity_map_if_unmapped(Frame::containing_address(fis_address as usize), EntryFlags::WRITABLE | EntryFlags::NO_CACHE, allocator);
*/
}

fn is_ahci_controller(device: &PCIDevice) -> bool {
    device.class_code(0) == 0x01 && ((device.subclass(0) == 0x06) | (device.subclass(0) == 0x01))
}