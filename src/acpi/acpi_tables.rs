use crate::utils::any_as_u8_slice;

#[repr(C, packed)]
pub struct ACPISDTHeader {
    signature: [char; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oemid: [char; 6],
    oemt_table_id: [char; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

impl ACPISDTHeader {
    pub fn length(&self) -> u32 {
        self.length
    }
}

#[repr(C, packed)]
pub struct RootSystemDescriptorTable {
    header: ACPISDTHeader,
    std_pointers: [u32; header.length]
}

impl RootSystemDescriptorTable {

}

#[repr(C, packed)]
pub struct FixedACPIDescriptionTable {
    header: ACPISDTHeader,
    firmware_ctrl: u32,
    dsdt: u32,

    // field used in ACPI 1.0; no longer in use, for compatibility only
    _reserved: u8,

    preferred_power_management_profile: PreferredPowerManagementProfile,
    sci_interrupt: u16,
    smi_command_port: u32,
    acpi_enable: u8,
    acpi_disable: u8,
    s4bios_req: u8,
    pstate_control: u8,
    pm1a_event_block: u32,
    pm1b_event_block: u32,
    pm1a_control_block: u32,
    pm1b_control_block: u32,
    pm2_control_block: u32,
    pm_timer_block: u32,
    gpe0_block: u32,
    gpe1_block: u32,
    pm1_event_length: u8,
    pm1_control_length: u8,
    pm2_control_length: u8,
    pm_timer_length: u8,
    gpe0_length: u8,
    gpe1_length: u8,
    gpe1_base: u8,
    c_state_control: u8,
    worst_c2_latency: u16,
    worst_c3_latency: u16,
    flush_size: u16,
    flush_stride: u16,
    duty_offset: u8,
    duty_width: u8,
    day_alarm: u8,
    month_alarm: u8,
    century: u8,

    // reserved in ACPI 1.0; used since ACPI 2.0+
    boot_architecture_flags: u16,

    _reserved2: u8,
    flags: u32,

    reset_reg: GenericAddressStructure,

    reset_value: u8,
    reserved3: [u8; 3],

    // 64bit pointers - Available on ACPI 2.0+
    x_firmware_control: u64,
    x_dsdt: u64,

    x_pm1a_event_block: GenericAddressStructure,
    x_pm1b_event_block: GenericAddressStructure,
    x_pm1a_control_block: GenericAddressStructure,
    x_pm1b_control_block: GenericAddressStructure,
    x_pm2_control_block: GenericAddressStructure,
    x_pm_timer_block: GenericAddressStructure,
    x_gpe0_block: GenericAddressStructure,
    x_gpe1_block: GenericAddressStructure,
}

#[repr(C, packed)]
pub struct GenericAddressStructure {
    address_space: GASAddressSpace,
    bit_width: u8,
    bit_offset: u8,
    access_size: GASAccessSize,
    address: u64,
}

#[repr(u8)]
enum GASAddressSpace {
    SystemMemory = 0,
    SystemIO = 1,
    PCIConfigurationSpace = 2,
    EmbeddedController = 3,
    SystemManagementBus = 4,
    SystemCMOS = 5,
    PCIDeviceTarget = 6,
    IntelligentPlatformManagementInfrastructure = 7,
    GeneralPurposeIO = 8,
    GenericSerialBus = 9,
    PlatformCommunicationChannel = 10,
}

#[repr(u8)]
enum GASAccessSize {
    Undefined = 0,
    ByteAccess = 1,
    WordAccess = 2,
    DWordAccess = 3,
    QWordAccess = 4,
}

#[repr(u8)]
enum PreferredPowerManagementProfile {
    Unspecified = 0,
    Desktop = 1,
    Mobile = 2,
    Workstation = 3,
    EnterpriseServer = 4,
    SOHOServer = 5,
    AplliancePC = 6,
    PerformanceServer = 7
}

fn validate_rsdp_checksum(fadt: &FixedACPIDescriptionTable)-> bool {
    // Add up every byte, the lowest byte of the result should be zero
    let mut fadt_bytes: &[u8];
    unsafe {
        fadt_bytes = any_as_u8_slice(fadt);
    }

    let sum: u64 = fadt_bytes.iter().map(|&n| n as u64).sum();

    sum % 2 == 0
}