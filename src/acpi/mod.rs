pub mod root_system_descriptor_pointer;
pub mod acpi_tables;

use core::ops::DerefMut;
use crate::acpi::root_system_descriptor_pointer::{find_rsdp, RSDP};
use crate::arch::multiboot2::BootInformation;
use crate::{println, print};
use crate::acpi::acpi_tables::RootSystemDescriptorTable;
use crate::memory::Frame;
use crate::memory::page_frame_allocator::PageFrameAllocator;
use crate::memory::paging::ActivePageTable;
use crate::memory::paging::entry::EntryFlags;

pub fn init_acpi(boot_info: &BootInformation, allocator: &mut PageFrameAllocator, page_table: &mut ActivePageTable) {
    let rsdp = find_rsdp(boot_info).expect("Error finding RSDP");

    let rsdt_address = match rsdp {
        RSDP::V1(rsdp_v1) => rsdp_v1.rsdt_address(),
        RSDP::V2(rsdp_v2) => rsdp_v2.rsdt_address(),
    };
    let rsdt = RootSystemDescriptorTable::from(rsdt_address);
    page_table.deref_mut().identity_map(Frame::containing_address(rsdt_address as usize), EntryFlags::PRESENT, allocator);

    let fadt_address = rsdt.fadt_address();

    println!("{:X}", fadt_address.unwrap());
}