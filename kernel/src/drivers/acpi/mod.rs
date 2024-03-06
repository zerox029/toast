pub mod root_system_descriptor_pointer;
pub mod acpi_tables;

pub fn init_acpi() {/*
    let rsdp = find_rsdp(boot_info).expect("Error finding RSDP");

    let rsdt_address = match rsdp {
        Rsdp::V1(rsdp_v1) => rsdp_v1.rsdt_address(),
        Rsdp::V2(rsdp_v2) => rsdp_v2.rsdt_address(),
    };
    let rsdt = RootSystemDescriptorTable::from(rsdt_address);

    MemoryManager::instance().lock().pmm_identity_map(Frame::containing_address(rsdt_address as usize), EntryFlags::PRESENT);

    let fadt_address = rsdt.fadt_address().expect("Could not find FADT address");
    let _fadt = FixedACPIDescriptionTable::from(fadt_address);*/
}