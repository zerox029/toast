mod root_system_descriptor_pointer;
mod acpi_tables;

use crate::acpi::root_system_descriptor_pointer::{find_rsdp, RSDP, rsdt_address};
use crate::arch::multiboot2::BootInformation;



trait RootSystemDescriptorPointer {}
impl RootSystemDescriptorPointer for RootSystemDescriptorPointerV1 {}
impl RootSystemDescriptorPointer for RootSystemDescriptorPointerV2 {}

pub fn init_acpi(boot_information: &BootInformation) {
    let rsdt_address = rsdt_address(boot_information: &BootInformation);


}