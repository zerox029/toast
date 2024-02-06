pub mod root_system_descriptor_pointer;
pub mod acpi_tables;

use crate::acpi::root_system_descriptor_pointer::{rsdt_address};
use crate::arch::multiboot2::BootInformation;

pub fn init_acpi(boot_information: &BootInformation) {
    let rsdt_address = rsdt_address(boot_information);
}