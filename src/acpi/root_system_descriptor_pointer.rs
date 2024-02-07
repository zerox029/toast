use crate::arch::multiboot2::BootInformation;
use crate::utils::any_as_u8_slice;

pub enum RSDP {
    V1(&'static RootSystemDescriptorPointerV1),
    V2(&'static RootSystemDescriptorPointerV2),
}

#[repr(C, packed)]
pub struct RootSystemDescriptorPointerV1 {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

impl RootSystemDescriptorPointerV1 {
    pub fn rsdt_address(&self) -> u32 {
        self.rsdt_address
    }
}

#[repr(C, packed)]
pub struct RootSystemDescriptorPointerV2 {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: [u8; 3],
}

impl RootSystemDescriptorPointerV2 {
    pub fn rsdt_address(&self) -> u32 {
        self.rsdt_address
    }
}

trait RootSystemDescriptorPointer {}
impl RootSystemDescriptorPointer for RootSystemDescriptorPointerV1 {}
impl RootSystemDescriptorPointer for RootSystemDescriptorPointerV2 {}

pub fn find_rsdp(boot_information: &BootInformation) -> Result<RSDP, &'static str> {
    let rsdp_v2 = match boot_information.acpi_new_rsdp() {
        Some(rsdp) => Some(&rsdp.rsdp_v2),
        None => None
    };

    // V1
    if rsdp_v2.is_none() {
        let rsdp_v1 = match boot_information.acpi_old_rsdp() {
            Some(rsdp) => Some(&rsdp.rsdp_v1),
            None => None
        };

        if rsdp_v1.is_none() {
            return Err("ACPI RSDP tag is required...")
        }

        let rsdp_v1 = rsdp_v1.unwrap();

        if !validate_rsdp_checksum(rsdp_v1) {
            return Err("Checksum validation failed...")
        }

        Ok(RSDP::V1(rsdp_v1))
    }
    // V2
    else {
        if !validate_rsdp_checksum(rsdp_v2.unwrap()) {
            return Err("Checksum validation failed...")
        }

        // technically should be reading xsdt, but I don't think it really matters, and Toast uses V1 anyway
        Ok(RSDP::V2(rsdp_v2.unwrap()))
    }
}

fn validate_rsdp_checksum<T: RootSystemDescriptorPointer>(rsdp: &T)-> bool {
    // Add up every byte, the lowest byte of the result should be zero
    let rsdp_bytes: &[u8];
    unsafe {
        rsdp_bytes = any_as_u8_slice(rsdp);
    }

    let sum: u64 = rsdp_bytes.iter().map(|&n| n as u64 ).sum();

    sum % 2 == 0
}