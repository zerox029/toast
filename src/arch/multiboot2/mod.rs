pub mod structures;

const END_TAG_SIZE: u32 = 8;

#[repr(C)]
pub struct BootInformation {
    pub total_size: u32,
    pub reserved: u32,
    pub first_tag: structures::Tag,
}

pub unsafe fn load(multiboot_information_address: usize) -> &'static BootInformation {
    let boot_info = &*(multiboot_information_address as *const BootInformation);
    boot_info
}

impl BootInformation {
    pub fn start_address(&self) -> usize {
        self as *const _ as usize
    }

    pub fn end_address(&self) -> usize {
        self.start_address() + self.total_size as usize
    }

    pub fn memory_map(&self) -> Option<&'static structures::MemoryMap> {
        self.get_tag(structures::TagType::MemoryMap)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::MemoryMap )})
    }

    pub fn elf_symbols(&self) -> Option<&'static structures::ElfSymbols> {
        self.get_tag(structures::TagType::ELFSymbols)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::ElfSymbols )})
    }

    pub fn acpi_old_rsdp(&self) -> Option<&'static structures::ACPIOldRSDP> {
        self.get_tag(structures::TagType::ACPIOldRSDP)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::ACPIOldRSDP )})
    }

    pub fn acpi_new_rsdp(&self) -> Option<&'static structures::ACPINewRSDP> {
        self.get_tag(structures::TagType::ACPINewRSDP)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::ACPINewRSDP )})
    }

    pub fn networking_information(&self) -> Option<&'static structures::NetworkingInformation> {
        self.get_tag(structures::TagType::NetworkingInformation)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::NetworkingInformation )})
    }

    pub fn efi_memory_map(&self) -> Option<&'static structures::EFIMemoryMap> {
        self.get_tag(structures::TagType::EFIMemoryMap)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::EFIMemoryMap )})
    }

    pub fn efi_boot_services_not_terminated(&self) -> Option<&'static structures::EFIBootServicesNotTerminated> {
        self.get_tag(structures::TagType::EFIBootServicesNotTerminated)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::EFIBootServicesNotTerminated )})
    }
    pub fn efi_32bit_image_handle_pointer(&self) -> Option<&'static structures::EFI32BitImageHandlePointer> {
        self.get_tag(structures::TagType::EFI32BitImageHandlePointer)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::EFI32BitImageHandlePointer )})
    }

    pub fn efi_64bit_image_handle_pointer(&self) -> Option<&'static structures::EFI64BitImageHandlePointer> {
        self.get_tag(structures::TagType::EFI64BitImageHandlePointer)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::EFI64BitImageHandlePointer )})
    }

    pub fn image_load_base_physical_address(&self) -> Option<&'static structures::ImageLoadBasePhysicalAddress> {
        self.get_tag(structures::TagType::ImgLoadBasePhysicalAddress)
            .map(|tag| unsafe{ &*(tag as *const structures::Tag as *const structures::ImageLoadBasePhysicalAddress )})
    }

    pub fn tags(&self) -> TagIterator {
        TagIterator{ current: &self.first_tag as *const _ }
    }

    pub fn get_tag(&self, typ: structures::TagType) -> Option<&'static structures::Tag> {
        self.tags().find(|tag| tag.typ == typ)
    }
}

pub struct TagIterator {
    current: *const structures::Tag,
}

impl Iterator for TagIterator {
    type Item = &'static structures::Tag;

    fn next(&mut self) -> Option<&'static structures::Tag> {
        match unsafe{ &*self.current } {
            &structures::Tag{ typ: structures::TagType::End, size: END_TAG_SIZE } => None,
            tag => {
                let mut tag_addr = self.current as usize;
                tag_addr += tag.size as usize;
                tag_addr = ((tag_addr - 1) & !0x7) + 0x8; // 8-bytes alignment
                self.current = tag_addr as *const structures::Tag;

                Some(tag)
            }
        }
    }
}