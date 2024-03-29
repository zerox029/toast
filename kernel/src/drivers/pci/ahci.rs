// Structure definitions
// https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/serial-ata-ahci-spec-rev1-3-1.pdf
// http://www.usedsite.co.kr/pds/file/SerialATA_Revision_3_0_RC11.pdf

// Command definitions:
// https://tc.gts3.org/cs3210/2016/spring/r/hardware/ATA8-ACS.pdf

#![allow(clippy::while_immutable_condition)]

use alloc::vec::Vec;
use core::arch::asm;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use crate::drivers::pci::{find_all_pci_devices, PCIDevice};
use crate::memory::{MemoryManager, PhysicalAddress};
use crate::memory::physical_memory::Frame;
use crate::memory::virtual_memory::paging::entry::EntryFlags;
use crate::utils::bitutils::is_nth_bit_set;

const SATA_SIG_ATA: u32     = 0x00000101;   // SATA drive
const SATA_SIG_ATAPI: u32   = 0xEB140101;   // SATAPI drive
const SATA_SIG_SEMB: u32    = 0xC33C0101;   // Enclosure management bridge
const SATA_SIG_PM: u32      = 0x96690101;    // Port multiplier


const FIS_TYPE_REG_H2D: u8      = 0x27;
const FIS_TYPE_REG_D2H: u8      = 0x34;
const FIS_TYPE_DMA_ACT: u8      = 0x39;
const FIS_TYPE_DMA_SETUP: u8    = 0x41;
const FIS_TYPE_DATA: u8         = 0x46;
const FIS_TYPE_BIST: u8         = 0x58;
const FIS_TYPE_PIO_SETUP: u8   = 0x5F;
const FIS_TYPE_DEV_BITS: u8     = 0xA1;


#[repr(C)]
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

#[repr(C)]
struct FisDmaSetup {
    fis_type: u8,       // FIS_TYPE_REG_DMA_SETUP

    flags: u8,         // Port multiplier

    rsvd: u16,      // Reserved

    dma_buffer_id: u64, // DMA Buffer Identifier. Used to Identify DMA buffer in host memory.
    // SATA Spec says host specific and not in Spec. Trying AHCI spec might work.

    rsvd2: u32,         // Reserved,

    dma_buf_offset: u32,// Byte offset into buffer. First 2 bits must be 0

    transfer_count: u32,// Number of bytes to transfer. Bit 0 must be 0

    rsvd3: u32,         // Reserved
}

#[repr(C)]
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

#[repr(C)]
struct FisSetDeviceBitsD2H {
    typ: u8,
    pmport: u8,
    status: u8,
    error: u8,
    protocol_specifi: u32,
}

#[repr(C)]
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
struct CommandHeader {
    flags: u16,
    prdtl: u16,
    prdbc: u32,
    ctba: u32,
    ctbau: u32,
    reserved: [u32; 4],
}

#[repr(C)]
struct CommandTable {
    cfis: [u8; 64], // Command FIS
    acmd: [u8; 16], // ATAPI command, 12 or 16 bytes
    rsv: [u8; 48],  // Reserved
    first_prdt_entry: PrdtEntry,
}

#[repr(C)]
struct PrdtEntry {
    dba: u32,
    dbau: u32,
    reserved: u32,
    dbc: u32,
}

// http://www.usedsite.co.kr/pds/file/SerialATA_Revision_3_0_RC11.pdf pp.479
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct AHCIIdentifyResponse {
    config: u16,      /* lots of obsolete bit flags */
    cyls: u16,      /* obsolete */
    reserved2: u16,   /* special config */
    heads: u16,      /* "physical_memory" heads */
    track_bytes: u16,   /* unformatted bytes per track */
    sector_bytes: u16,   /* unformatted bytes per sector */
    sectors: u16,   /* "physical_memory" sectors per track */
    vendor0: u16,   /* vendor unique */
    vendor1: u16,   /* vendor unique */
    vendor2: u16,   /* vendor unique */
    serial_no: [u8; 20],   /* 0 = not_specified */
    buf_type: u16,
    buf_size: u16,   /* 512 byte increments; 0 = not_specified */
    ecc_bytes: u16,   /* for r/w long cmds; 0 = not_specified */
    fw_rev: [u8; 8],   /* 0 = not_specified */
    model: [u8; 40],   /* 0 = not_specified */
    multi_count: u16, /* Multiple Count */
    dword_io: u16,   /* 0=not_implemented; 1=implemented */
    capability1: u16,   /* vendor unique */
    capability2: u16,   /* bits 0:DMA 1:LBA 2:IORDYsw 3:IORDYsup word: 50 */
    vendor5: u8,   /* vendor unique */
    tpio: u8,      /* 0=slow, 1=medium, 2=fast */
    vendor6: u8,   /* vendor unique */
    tdma: u8,      /* 0=slow, 1=medium, 2=fast */
    field_valid: u16,   /* bits 0:cur_ok 1:eide_ok */
    cur_cyls: u16,   /* logical cylinders */
    cur_heads: u16,   /* logical heads word 55*/
    cur_sectors: u16,   /* logical sectors per track */
    cur_capacity0: u16,   /* logical total sectors on drive */
    cur_capacity1: u16,   /*  (2 words, misaligned int)     */
    multsect: u8,   /* current multiple sector count */
    multsect_valid: u8,   /* when (bit0==1) multsect is ok */
    lba_capacity: u32,   /* total number of sectors */
    dma_1word: u16,   /* single-word dma info */
    dma_mword: u16,   /* multiple-word dma info */
    eide_pio_modes: u16, /* bits 0:mode3 1:mode4 */
    eide_dma_min: u16,   /* min mword dma cycle time (ns) */
    eide_dma_time: u16,   /* recommended mword dma cycle time (ns) */
    eide_pio: u16,       /* min cycle time (ns), no IORDY  */
    eide_pio_iordy: u16, /* min cycle time (ns), with IORDY */
    words69_70: [u16; 2],   /* reserved words 69-70 */
    words71_74: [u16; 4],   /* reserved words 71-74 */
    queue_depth: u16,   /*  */
    sata_capability: u16,   /*  SATA Capabilities word 76*/
    sata_additional: u16,   /*  Additional Capabilities */
    sata_supported: u16,   /* SATA Features supported  */
    features_enabled: u16,   /* SATA features enabled */
    major_rev_num: u16,   /*  Major rev number word 80 */
    minor_rev_num: u16,   /*  */
    command_set_1: u16,   /* bits 0:Smart 1:Security 2:Removable 3:PM */
    command_set_2: u16,   /* bits 14:Smart Enabled 13:0 zero */
    cfsse: u16,      /* command set-feature supported extensions */
    cfs_enable_1: u16,   /* command set-feature enabled */
    cfs_enable_2: u16,   /* command set-feature enabled */
    csf_default: u16,   /* command set-feature default */
    dma_ultra: u16,   /*  */
    word89: u16,      /* reserved (word 89) */
    word90: u16,      /* reserved (word 90) */
    cur_apm_values: u16,   /* current APM values */
    word92: u16,         /* reserved (word 92) */
    comreset: u16,      /* should be cleared to 0 */
    accoustic: u16,      /*  accoustic management */
    min_req_sz: u16,      /* Stream minimum required size */
    transfer_time_dma: u16,   /* Streaming Transfer Time-DMA */
    access_latency: u16,      /* Streaming access latency-DMA & PIO WORD 97*/
    perf_granularity: u32,   /* Streaming performance granularity */
    total_usr_sectors: [u32; 2], /* Total number of user addressable sectors */
    transfer_time_pio: u16,    /* Streaming Transfer time PIO */
    reserved105: u16,       /* Word 105 */
    sector_sz: u16,          /* Puysical Sector size / Logical sector size */
    inter_seek_delay: u16,   /* In microseconds */
    words108_116: [u16; 9],    /*  */
    words_per_sector: u32,    /* words per logical sectors */
    supported_settings: u16, /* continued from words 82-84 */
    command_set_3: u16,       /* continued from words 85-87 */
    words121_126: [u16; 6],   /* reserved words 121-126 */
    word127: u16,         /* reserved (word 127) */
    security_status: u16,   /* device lock function
                   * 15:9   reserved
                   * 8   security level 1:max 0:high
                   * 7:6   reserved
                   * 5   enhanced erase
                   * 4   expire
                   * 3   frozen
                   * 2   locked
                   * 1   en/disabled
                   * 0   capability
                   */
    csfo: u16,      /* current set features options
                   * 15:4   reserved
                   * 3   auto reassign
                   * 2   reverting
                   * 1   read-look-ahead
                   * 0   write cache
                   */
    words130_155: [u16; 26],/* reserved vendor words 130-155 */
    word156: u16,
    words157_159: [u16; 3],/* reserved vendor words 157-159 */
    cfa: u16, /* CFA Power mode 1 */
    words161_175: [u16; 15], /* Reserved */
    media_serial: [u8; 60], /* words 176-205 Current Media serial number */
    sct_cmd_transport: u16, /* SCT Command Transport */
    words207_208: [u16; 2], /* reserved */
    block_align: u16, /* Alignement of logical blocks in larger physical_memory blocks */
    wrv_sec_count: u32, /* Write-Read-Verify sector count mode 3 only */
    verf_sec_count: u32, /* Verify Sector count mode 2 only */
    nv_cache_capability: u16, /* NV Cache capabilities */
    nv_cache_sz: u16, /* NV Cache size in logical blocks */
    nv_cache_sz2: u16, /* NV Cache size in logical blocks */
    rotation_rate: u16, /* Nominal media rotation rate */
    reserved218: u16, /*  */
    nv_cache_options: u16, /* NV Cache options */
    words220_221: [u16; 2], /* reserved */
    transport_major_rev: u16, /*  */
    transport_minor_rev: u16, /*  */
    words224_233: [u16; 10], /* Reserved */
    min_dwnload_blocks: u16, /* Minimum number of 512byte units per DOWNLOAD MICROCODE
                                  command for mode 03h */
    max_dwnload_blocks: u16, /* Maximum number of 512byte units per DOWNLOAD MICROCODE
                                  command for mode 03h */
    words236_254: [u16; 19],   /* Reserved */
    integrity: u16,          /* Cheksum, Signature */
}

#[derive(Debug, Copy, Clone)]
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
    fn new(pci_device: PCIDevice) -> Self {
        // Memory map HBA registers as uncacheable.
        let bar5 = pci_device.bar5(0);
        let start_frame = Frame::containing_address(bar5 as PhysicalAddress);
        let end_frame = Frame::containing_address(bar5 as PhysicalAddress + 0x10FF);
        for frame in Frame::range_inclusive(start_frame, end_frame) {
            MemoryManager::instance().lock().pmm_identity_map(frame, EntryFlags::WRITABLE | EntryFlags::NO_CACHE);
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
            warn!("ahci: bios/os handoff not supported");
        }

        // TODO
    }
}

#[derive(Debug)]
pub struct AHCIDevice {
    controller: AHCIController,
    port_index: usize,

    identity: Option<AHCIIdentifyResponse>,

    pub port_registers: &'static mut PortRegisters,

    command_list: [AHCICommand; 32],
}

impl AHCIDevice {
    fn new(controller: AHCIController, port_index: usize, port_address: usize) -> Self {
        let port_registers = unsafe { &mut *(port_address as *mut PortRegisters) };

        Self {
            controller,
            port_index,

            identity: None,

            port_registers,

            command_list: [AHCICommand::new(); 32],
        }
    }

    /// Reads byte_count bytes from the device at address offset. Returns the number of bytes reads from the device
    pub fn read_from_device(&mut self, byte_offset: u64, byte_count: u64, buffer: *mut c_void) -> usize {
        let identity = &self.identity.expect("ahci: cannot read from an unidentified device");
        let sector_size = identity.sector_bytes as u64;

        let start_block = byte_offset / sector_size;
        let mut block_count = byte_count.div_ceil(sector_size);

        if byte_count % sector_size == 0 && byte_offset % sector_size != 0 {
            block_count += 1;
        }
        if byte_offset % sector_size + block_count > sector_size {
            block_count += 1;
        }

        if block_count == 0 {
            return 0;
        }

        let read_buffer_address = MemoryManager::pmm_identity(byte_count as usize, EntryFlags::WRITABLE)
            .expect("ahci: could not allocate the memory for device read");

        let read_sectors = self.issue_read(start_block, block_count, read_buffer_address as *mut c_void);

        unsafe { ptr::copy_nonoverlapping((read_buffer_address + (byte_offset % sector_size) as usize) as *const c_void, buffer, byte_count as usize); }

        MemoryManager::pmm_free(byte_count as usize, read_buffer_address);

        read_sectors - read_sectors.abs_diff(byte_count as usize)
    }

    pub fn write_to_device(&mut self, byte_offset: u64, byte_count: u64, buffer: *mut c_void) {
        let identity = &self.identity.expect("ahci: cannot write to an unidentified device");
        let sector_size = identity.sector_bytes as u64;

        let start_block = byte_offset / sector_size;
        let block_count = byte_count.div_ceil(sector_size);

        let write_buffer_address = {
            MemoryManager::pmm_identity(byte_count as usize, EntryFlags::WRITABLE)
                .expect("ahci: could not allocate the memory for device write")
        };

        if byte_offset % sector_size != 0 {
            self.read_from_device(byte_offset, sector_size, write_buffer_address as *mut c_void);
        }
        if byte_count % sector_size != 0 {
            self.read_from_device(byte_offset + block_count - 1, sector_size, write_buffer_address as *mut c_void);
        }

        unsafe { ptr::copy_nonoverlapping(buffer, (write_buffer_address + (byte_offset % sector_size) as usize) as *mut c_void, byte_count as usize)};

        self.issue_write(start_block, block_count, write_buffer_address as *mut c_void);

        MemoryManager::pmm_free(byte_count as usize, write_buffer_address);

        //written_sectors - written_sectors.abs_diff(byte_count as usize)
    }

    fn issue_identify(&mut self, identity: *mut AHCIIdentifyResponse) {
        let command_number = self.allocate_slot();

        {
            let command = &mut self.command_list[command_number];

            command.destination_address = identity as *mut c_void;
            command.data_length = 511;
            command.interrupt = false;

            let command_header = unsafe{ &mut *command.command_header };
            command_header.flags |= (size_of::<FisRegH2D>() / 4) as u16;
            command_header.prdtl = 1;
            command_header.reserved = [0; 4];

            let command_table = unsafe{ &mut *command.command_table };
            let command_pointer = &mut command_table.cfis;

            command_pointer.fill(0);
            command_pointer[0] = FIS_TYPE_REG_H2D; // FIS_TYPE
            command_pointer[1] = 1 << 7;  // flags
            command_pointer[2] = 0xEC;  // device
        }


        self.init_prdt(command_number);
        self.issue_command(command_number);
    }

    /// Reads sector_count amount of sectors from the device and writes it to buffer. Returns the amount of sectors read from the device
    fn issue_read(&mut self, sector_offset: u64, sector_count: u64, buffer: *mut c_void) -> usize {
        let command_number = self.allocate_slot();

        let command = &mut self.command_list[command_number];

        command.destination_address = buffer;
        command.data_length = (sector_count * 0x200 - 1) as usize;
        command.interrupt = false;

        let command_header = unsafe{ &mut *command.command_header };
        command_header.flags &= !(0b11111 | (1 << 6));
        command_header.flags |= (size_of::<FisRegH2D>() / 4) as u16;
        command_header.prdtl = 1;
        command_header.reserved = [0; 4];

        let command_table = unsafe{ &mut *command.command_table };
        let command_pointer = &mut command_table.cfis;

        command_pointer.fill(0);
        command_pointer[0] = FIS_TYPE_REG_H2D; // FIS_TYPE
        command_pointer[1] = 1 << 7; // flags
        command_pointer[2] = 0x25; // command
        command_pointer[7] = 1 << 6; // device

        command_pointer[4] = sector_offset as u8; // LBA0
        command_pointer[5] = (sector_offset >> 8) as u8; // LBA1
        command_pointer[6] = (sector_offset >> 16) as u8; // LBA2
        command_pointer[8] = (sector_offset >> 24) as u8; // LBA3
        command_pointer[9] = (sector_offset >> 32) as u8; // LBA4
        command_pointer[10] = (sector_offset >> 40) as u8; // LBA5

        command_pointer[12] = sector_count as u8; // countl
        command_pointer[13] = (sector_count >> 8) as u8; // counth

        self.init_prdt(command_number);
        self.issue_command(command_number);

        command_header.prdbc as usize
    }

    /// Writes sector_count amount of sectors from the buffer and writes it to the device
    fn issue_write(&mut self, sector_offset: u64, sector_count: u64, buffer: *mut c_void) {
        let command_number = self.allocate_slot();

        {
            let command = &mut self.command_list[command_number];

            let command_header = unsafe{ &mut *command.command_header };
            command_header.flags &= !(0b11111 | (1 << 6));
            command_header.flags |= (size_of::<FisRegH2D>() / 4) as u16;
            command_header.prdtl = 1;
            command_header.reserved = [0; 4];

            command.destination_address = buffer;
            command.data_length = (sector_count * 0x200 - 1) as usize;
            command.interrupt = false;

            let command_table = unsafe{ &mut *command.command_table };
            let command_pointer = &mut command_table.cfis;

            command_pointer.fill(0);
            command_pointer[0] = FIS_TYPE_REG_H2D; // FIS_TYPE
            command_pointer[1] = 1 << 7; // flags
            command_pointer[2] = 0x35; // command
            command_pointer[7] = 1 << 6; // device

            command_pointer[4] = sector_offset as u8; // LBA0
            command_pointer[5] = (sector_offset >> 8) as u8; // LBA1
            command_pointer[6] = (sector_offset >> 16) as u8; // LBA2
            command_pointer[8] = (sector_offset >> 24) as u8; // LBA3
            command_pointer[9] = (sector_offset >> 32) as u8; // LBA4
            command_pointer[10] = (sector_offset >> 40) as u8; // LBA5

            command_pointer[12] = sector_count as u8; // countl
            command_pointer[13] = (sector_count >> 8) as u8; // counth
        }

        self.init_prdt(command_number);
        self.issue_command(command_number);
    }

    fn allocate_slot(&mut self) -> usize {
        let slot_count = self.controller.slot_count;

        for i in 0..slot_count {
            // Find the first empty command slot
            if !is_nth_bit_set(self.port_registers.sact as usize, i as usize) && !is_nth_bit_set(self.port_registers.ci as usize, i as usize) {
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

    fn init_prdt(&mut self, command_number: usize) {
        let command = &self.command_list[command_number];
        let command_table = unsafe{ &mut *command.command_table };

        command_table.rsv.fill(0);
        command_table.first_prdt_entry.dba = command.destination_address as u32;
        command_table.first_prdt_entry.dbau = (command.destination_address as u64 >> 32) as u32;
        command_table.first_prdt_entry.dbc = (command.data_length | ((command.interrupt as usize) << 31)) as u32;
        command_table.first_prdt_entry.reserved = 0;
    }

    fn issue_command(&mut self, command_number: usize) {
        const PORT_TFD_BSY: u32 = 1 << 7;
        const PORT_TFD_DRQ: u32 = 1 << 3;
        const PORT_TFD_ERR: u32 = 1 << 0;
        const PORT_CMD_ST: u32 = 1 << 0;
        const PORT_CMD_CR: u32 = 1 << 15;
        const PORT_CMD_FRE: u32 = 1 << 4;
        const PORT_CMD_FR: u32 = 1 << 14;

        let command = &self.command_list[command_number];

        // Wait until busy and transfer requested flags are not set
        while self.port_registers.tfd & PORT_TFD_BSY != 0 || self.port_registers.tfd & PORT_TFD_DRQ != 0 {
            unsafe { asm!("pause;"); }
        }

        self.port_registers.cmd &= !PORT_CMD_ST;
        while self.port_registers.cmd & PORT_CMD_CR != 0 {
            unsafe { asm!("pause;"); }
        } // good

        self.port_registers.cmd |= PORT_CMD_FRE;
        while self.port_registers.cmd & PORT_CMD_FR == 0 {
            unsafe { asm!("pause;"); }
        }
        self.port_registers.cmd |= PORT_CMD_ST;

        self.port_registers.ci = 1 << command.slot;

        while self.port_registers.ci & (1 << command.slot) != 0 {
            unsafe { asm!("pause;"); }
        }

        if self.port_registers.tfd & PORT_TFD_ERR  != 0{
            panic!("ahci: an error has occured during command data transfer");
        }

        self.port_registers.cmd &= !PORT_CMD_ST;
        while self.port_registers.cmd & PORT_CMD_ST != 0 {
            unsafe { asm!("pause;"); }
        }
        self.port_registers.cmd &= !PORT_CMD_FRE;
    }
}

#[derive(Debug, Copy, Clone)]
struct AHCICommand {
    command_header: *mut CommandHeader,
    command_table: *mut CommandTable,
    ahci_device: *mut AHCIDevice,

    destination_address: *mut c_void,
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

            destination_address: ptr::null_mut(),
            data_length: 0,
            interrupt: false,

            slot: 0,
        }
    }
}

pub fn init() -> Vec<AHCIDevice> {
    info!("ahci: init...");

    let ahci_pci_device = find_all_pci_devices().into_iter().find(is_ahci_controller).expect("ahci: could not locate the ahci controller");
    let ahci_controller = AHCIController::new(ahci_pci_device);

    info!("ahci: controller version {}.{}", ahci_controller.version_maj, ahci_controller.version_min);

    // Enable interrupts, DMA, and memory space access in the PCI command register
    let updated_command = (ahci_pci_device.command(0) | 0x2) & 0b1111101111111111;
    ahci_pci_device.set_command(0, updated_command);

    // Check if 64-bit DMA is supported
    if !is_nth_bit_set(ahci_controller.hba.cap as usize, 31) {
        panic!("ahci: controller not capable of 64 bit addressing... aborting")
    }

    ahci_controller.bios_os_handoff();

    // Initialize ports
    let mut devices = Vec::new();
    for port in 0..ahci_controller.port_count as usize {
        if is_nth_bit_set(ahci_controller.hba.pi as usize, port) {
            let device = init_port(&ahci_controller, port, ahci_controller.bar5 as usize + (0x100 + port * 0x80));
            if let Some(ahci_device) = device {
                devices.push(ahci_device);
            }
        }
    }

    devices
}

fn init_port(controller: &AHCIController, port_index: usize, port_address: usize) -> Option<AHCIDevice> {
    let mut ahci_device = AHCIDevice::new(*controller, port_index, port_address); // TODO: Allocate on heap instead of cloning

    match ahci_device.port_registers.sig {
        SATA_SIG_ATA => ok!("ahci: sata drive found on port {}", port_index),
        SATA_SIG_ATAPI => ok!("ahci: satapi drive found on port {}", port_index),
        SATA_SIG_SEMB => ok!("ahci: enclosure management bridge found on port {}", port_index),
        SATA_SIG_PM => ok!("ahci: port multiplier found on port {}", port_index),
        _ => return None
    }

    // TODO: Allocate memory for these more efficiently, no need to allocate a new frame every time
    // Allocate physical memory for the command list
    let command_list_base = {
        MemoryManager::pmm_identity(1, EntryFlags::WRITABLE | EntryFlags::NO_CACHE)
            .unwrap_or_else(|| panic!("ahci: could not allocate the memory for the command list on port {}", port_index))
    };

    ahci_device.port_registers.clb = command_list_base as u32;
    ahci_device.port_registers.clbu = (command_list_base >> 32) as u32;

    // Allocate physical memory for the command tables
    for i in 0..32 {
        let header_address = command_list_base + i * size_of::<CommandHeader>();
        let command_header = unsafe{ &mut *(header_address as *mut CommandHeader) };

        let command_table_base_address = {
            MemoryManager::pmm_identity(1, EntryFlags::WRITABLE | EntryFlags::NO_CACHE)
                .unwrap_or_else(|| panic!("ahci: could not allocate the memory for the command table {} on port {}", i, port_index))
        };


        command_header.ctba = command_table_base_address as u32;
        command_header.ctbau = (command_table_base_address >> 32) as u32;
    }

    // Allocate physical memory for the received FIS
    let fis_base_base_address = {
        MemoryManager::pmm_identity(1, EntryFlags::WRITABLE | EntryFlags::NO_CACHE)
            .unwrap_or_else(|| panic!("ahci: could not allocate the memory for the FIS on port {}", port_index))
    };

    ahci_device.port_registers.fb = fis_base_base_address as u32;
    ahci_device.port_registers.fbu = (fis_base_base_address >> 32) as u32;

    // Setting start and FIS receive enable flags
    ahci_device.port_registers.cmd |= (1 << 0) | (1 << 4);

    let identity_address = {
        MemoryManager::pmm_identity(size_of::<AHCIIdentifyResponse>(), EntryFlags::WRITABLE | EntryFlags::NO_CACHE)
            .expect("ahci: could not allocate the memory for device identification")
    };

    ahci_device.issue_identify(identity_address as *mut AHCIIdentifyResponse);

    let sata_identify = unsafe{&*(identity_address as *mut AHCIIdentifyResponse)};
    ahci_device.identity = Some(*sata_identify);

    MemoryManager::pmm_free(1, identity_address);

    Some(ahci_device)
}

fn is_ahci_controller(device: &PCIDevice) -> bool {
    device.class_code(0) == 0x01 && ((device.subclass(0) == 0x06) | (device.subclass(0) == 0x01))
}