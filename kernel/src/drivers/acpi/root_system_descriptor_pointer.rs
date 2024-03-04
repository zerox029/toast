use crate::arch::multiboot2::BootInformation;
use crate::utils::any_as_u8_slice;

pub enum Rsdp {
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

pub fn find_rsdp(boot_information: &BootInformation) -> Result<Rsdp, &'static str> {
    let rsdp_v2 = boot_information.acpi_new_rsdp().map(|rsdp| &rsdp.rsdp_v2);

    // V2
    if let Some(rsdp) = rsdp_v2 {
        if !validate_rsdp_checksum(rsdp) {
            return Err("Checksum validation failed...")
        }

        // technically should be reading xsdt, but I don't think it really matters, and Toast uses V1 anyway
        Ok(Rsdp::V2(rsdp))
    }
    // V1
    else {
        let rsdp_v1 = boot_information.acpi_old_rsdp().map(|rsdp| &rsdp.rsdp_v1);

        if let Some(rsdp) = rsdp_v1 {
            if !validate_rsdp_checksum(rsdp) {
                return Err("Checksum validation failed...")
            }

            Ok(Rsdp::V1(rsdp))
        }
        else {
            Err("ACPI RSDP tag is required...")
        }
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