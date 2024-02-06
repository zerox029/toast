use crate::arch::multiboot2::BootInformation;
use crate::utils::any_as_u8_slice;

pub enum RSDP {
    V1(&'static RootSystemDescriptorPointerV1),
    V2(&'static RootSystemDescriptorPointerV2),
}

#[repr(C)]
pub struct RootSystemDescriptorPointerV1 {
    signature: [char; 8],
    checksum: u8,
    oemid: [char; 6],
    revision: u8,
    rsdt_address: u32,
}

impl RootSystemDescriptorPointerV1 {
}

#[repr(C)]
pub struct RootSystemDescriptorPointerV2 {
    signature: [char; 8],
    checksum: u8,
    oemid: [char; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: [u8; 3],
}

trait RootSystemDescriptorPointer {}
impl RootSystemDescriptorPointer for RootSystemDescriptorPointerV1 {}
impl RootSystemDescriptorPointer for RootSystemDescriptorPointerV2 {}

pub fn rsdt_address(boot_information: &BootInformation) -> u32 {
    let rsdp_v2 = find_rsdp_v2(boot_information);

    // V1
    if rsdp_v2.is_none() {
        let rsdp_v1 = find_rsdp_v1(boot_information);

        if rsdp_v1.is_none() {
            panic!("ACPI RSDP tag is required...");
        }

        if !validate_rsdp_checksum(rsdp_v1.unwrap()) {
            panic!("Checksum validation failed...");
        }

        rsdp_v1.unwrap().rsdt_address
    }
    // V2
    else {
        if !validate_rsdp_checksum(rsdp_v2.unwrap()) {
            panic!("Checksum validation failed...");
        }

        rsdp_v2.unwrap().rsdt_address
    }
}

fn find_rsdp_v2(boot_information: &BootInformation) -> Option<&RootSystemDescriptorPointerV2> {
    let acpi_new_rsdp = boot_information.acpi_new_rsdp();

    match acpi_new_rsdp {
        Some(rsdp) => Some(&rsdp.rsdp_v2),
        None => None
    }
}

fn find_rsdp_v1(boot_information: &BootInformation) -> Option<&RootSystemDescriptorPointerV1> {
    let acpi_old_rsdp = boot_information.acpi_old_rsdp();

    match acpi_old_rsdp {
        Some(rsdp) => Some(&rsdp.rsdp_v1),
        None => None
    }
}

fn validate_rsdp_checksum<T: RootSystemDescriptorPointer>(rsdp: &T)-> bool {
    // Add up every byte, the lowest byte of the result should be zero
    let mut rsdp_bytes: &[u8];
    unsafe {
        rsdp_bytes = any_as_u8_slice(rsdp);
    }

    let sum: u64 = rsdp_bytes.iter().map(|&n| n as u64).sum();

    sum % 2 == 0
}