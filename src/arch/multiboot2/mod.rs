mod tags;

const END_TAG_SIZE: u32 = 8;

#[repr(C)]
pub struct BootInformation {
    pub total_size: u32,
    pub reserved: u32,
    pub first_tag: tags::Tag,
}

pub unsafe fn load(multiboot_information_address: usize) -> &'static BootInformation {
    let boot_info = &*(multiboot_information_address as *const BootInformation);
    boot_info
}

impl BootInformation {
    pub fn memory_map(&self) -> Option<&'static tags::MemoryMap> {
        self.get_tag(tags::TagType::MemoryMap)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::MemoryMap )})
    }

    pub fn elf_symbols(&self) -> Option<&'static tags::ElfSymbols> {
        self.get_tag(tags::TagType::ELFSymbols)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::ElfSymbols )})
    }

    pub fn networking_information(&self) -> Option<&'static tags::NetworkingInformation> {
        self.get_tag(tags::TagType::NetworkingInformation)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::NetworkingInformation )})
    }

    pub fn efi_memory_map(&self) -> Option<&'static tags::EFIMemoryMap> {
        self.get_tag(tags::TagType::EFIMemoryMap)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::EFIMemoryMap )})
    }

    pub fn efi_boot_services_not_terminated(&self) -> Option<&'static tags::EFIBootServicesNotTerminated> {
        self.get_tag(tags::TagType::EFIBootServicesNotTerminated)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::EFIBootServicesNotTerminated )})
    }
    pub fn efi_32bit_image_handle_pointer(&self) -> Option<&'static tags::EFI32BitImageHandlePointer> {
        self.get_tag(tags::TagType::EFI32BitImageHandlePointer)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::EFI32BitImageHandlePointer )})
    }

    pub fn efi_64bit_image_handle_pointer(&self) -> Option<&'static tags::EFI64BitImageHandlePointer> {
        self.get_tag(tags::TagType::EFI64BitImageHandlePointer)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::EFI64BitImageHandlePointer )})
    }

    pub fn image_load_base_physical_address(&self) -> Option<&'static tags::ImageLoadBasePhysicalAddress> {
        self.get_tag(tags::TagType::ImgLoadBasePhysicalAddress)
            .map(|tag| unsafe{ &*(tag as *const tags::Tag as *const tags::ImageLoadBasePhysicalAddress )})
    }

    pub fn tags(&self) -> TagIterator {
        TagIterator{ current: &self.first_tag as *const _ }
    }

    pub fn get_tag(&self, typ: tags::TagType) -> Option<&'static tags::Tag> {
        self.tags().find(|tag| tag.typ == typ)
    }
}

pub struct TagIterator {
    current: *const tags::Tag,
}

impl Iterator for TagIterator {
    type Item = &'static tags::Tag;

    fn next(&mut self) -> Option<&'static tags::Tag> {
        match unsafe{ &*self.current } {
            &tags::Tag{ typ: tags::TagType::End, size: END_TAG_SIZE } => None,
            tag => {
                let mut tag_addr = self.current as usize;
                tag_addr += tag.size as usize;
                tag_addr = ((tag_addr - 1) & !0x7) + 0x8; // 8-bytes alignment
                self.current = tag_addr as *const tags::Tag;

                Some(tag)
            }
        }
    }
}