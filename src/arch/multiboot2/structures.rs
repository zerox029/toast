use bitflags::bitflags;

/// Consult https://www.gnu.org/software/grub/manual/multiboot2/html_node/Boot-information-format.html for
/// implementation details
#[repr(u32)]
#[derive(Debug, Eq, PartialEq)]
pub enum TagType {
    End = 0,
    CommandLine = 1,
    BootloaderName = 2,
    Modules = 3,
    BasicMemoryInfo = 4,
    BIOSBootDevice = 5,
    MemoryMap = 6,
    VBEInfo = 7,
    FramebufferInfo = 8,
    ELFSymbols = 9,
    APMTable = 10,
    EFI32BitSystemTablePointer = 11,
    EFI64BitSystemTablePointer = 12,
    SMBIOSTables = 13,
    ACPIoldRSDP = 14,
    ACPInewRSDP = 15,
    NetworkingInformation = 16,
    EFIMemoryMap = 17,
    EFIBootServicesNotTerminated = 18,
    EFI32BitImageHandlePointer = 19,
    EFI64BitImageHandlePointer = 20,
    ImgLoadBasePhysicalAddress = 21,
}

#[repr(C)]
pub struct Tag {
    pub typ: TagType,
    pub size: u32,
}

/// This tag provides memory map. 'entry_size' contains the size of one entry so that in future
/// new fields may be added to it. It’s guaranteed to be a multiple of 8.
/// ‘entry_version’ is currently set at ‘0’. Future versions will increment this field.
/// Future version are guaranteed to be backward compatible with older format.
#[repr(C)]
pub struct MemoryMap {
    pub typ: TagType,   // 6
    pub size: u32,
    pub entry_size: u32,
    pub entry_version: u32,
    pub first_entry: MemoryMapEntry
}
impl MemoryMap {
    pub fn entries(&self) -> MemoryMapIter {
        MemoryMapIter {
            current_entry: &(self.first_entry) as *const MemoryMapEntry,
            last_entry: ((self as *const MemoryMap as u32) + self.size - self.entry_size) as *const MemoryMapEntry,
            entry_size: self.entry_size
        }
    }
}

/// ‘size’ contains the size of current entry including this field itself.
/// It may be bigger than 24 bytes in future versions but is guaranteed to be ‘base_addr’ is the
/// starting physical address. ‘length’ is the size of the memory region in bytes. ‘type’ is the
/// variety of address range represented, where a value of 1 indicates available RAM, value of 3
/// indicates usable memory holding ACPI information, value of 4 indicates reserved memory which
/// needs to be preserved on hibernation, value of 5 indicates a memory which is occupied by
/// defective RAM modules and all other values currently indicated a reserved area. ‘reserved’
/// is set to ‘0’ by bootloader and must be ignored by the OS image.
#[repr(C)]
pub struct MemoryMapEntry {
    pub base_addr: u64,
    pub size: u64,
    pub typ: u32,
    _reserved: u32,
}
#[derive(Clone)]
pub struct MemoryMapIter {
    pub current_entry: *const MemoryMapEntry,
    pub last_entry: *const MemoryMapEntry,
    pub entry_size: u32,
}
impl Iterator for MemoryMapIter {
    type Item = &'static MemoryMapEntry;

    fn next(&mut self) -> Option<&'static MemoryMapEntry> {
        if self.current_entry > self.last_entry {
            None
        }
        else {
            let entry = unsafe { &*self.current_entry };
            self.current_entry = ((self.current_entry as u32) + self.entry_size) as *const MemoryMapEntry;

            // As specified above, a type of 1 indicated available RAM
            if entry.typ == 1 {
                Some(entry)
            }
            else {
                self.next()
            }
        }
    }
}

/// This tag contains section header table from an ELF kernel, the size of each entry, number of entries, and the
/// string table used as the index of names. They correspond to the ‘shdr_*’ entries (‘shdr_num’, etc.) in the
/// Executable and Linkable Format (ELF) specification in the program header. All sections are loaded, and the physical
/// address fields of the ELF section header then refer to where the sections are in memory (refer to the i386 ELF
/// documentation for details as to how to read the section header(s)).
#[repr(C, packed)]
pub struct ElfSymbols {
    typ: u32,   // 9
    size: u32,
    num: u32,
    entsize: u32,
    shndx: u32,
    first_section_header: SectionHeader,
}
impl ElfSymbols {
    pub fn section_headers(&'static self) -> SectionHeaderIter {
        SectionHeaderIter {
            current_header: &self.first_section_header,
            remaining_headers: self.num - 1,
            entry_size: self.entsize
        }
    }
}
#[derive(Clone)]
pub struct SectionHeaderIter {
    pub current_header: &'static SectionHeader,
    pub remaining_headers: u32,
    pub entry_size: u32
}
impl Iterator for SectionHeaderIter {
    type Item = &'static SectionHeader;

    fn next(&mut self) -> Option<&'static SectionHeader> {
        if self.remaining_headers == 0 {
            None
        }
        else {
            let header = self.current_header;
            let next_header_address = (self.current_header as *const _ as u32) + self.entry_size;
            self.current_header = unsafe { &*(next_header_address as *const SectionHeader) };
            self.remaining_headers -= 1;

            if header.typ == ElfSectionHeaderType::Null as u32 {
                self.next()
            }
            else {
                Some(header)
            }
        }
    }
}
/// Consult https://man7.org/linux/man-pages/man5/elf.5.html and https://refspecs.linuxfoundation.org/elf/elf.pdf
/// for implementation details
#[repr(C, packed)]
pub struct SectionHeader {
    name: u32,
    typ: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entsize: u64,
}

impl SectionHeader {
    pub fn start_address(&self) -> usize {
        self.addr as usize
    }

    pub fn size(&self) -> usize { self.size as usize }

    pub fn end_address(&self) -> usize {
        (self.addr + self.size) as usize
    }

    pub fn flags(&self) -> ElfSectionHeaderFlags {
        ElfSectionHeaderFlags::from_bits_truncate(self.flags)
    }

    pub fn is_allocated(&self) -> bool {
        self.flags().contains(ElfSectionHeaderFlags::ALLOCATED)
    }
}
#[derive(Eq, PartialEq)]
#[repr(u32)]
pub enum ElfSectionHeaderType {
    Null = 0,
    Progbits = 1,
    Symtab = 2,
    Strtab = 3,
    Rela = 4,
    Hash = 5,
    Dynamic = 6,
    Note = 7,
    Nobits = 8,
    Rel = 9,
    Shlib = 10,
    Dynsym = 11,
}
bitflags! {
    pub struct ElfSectionHeaderFlags: u64 {
        const WRITABLE = 0x1;
        const ALLOCATED = 0x2;
        const EXECUTABLE = 0x4;

    }
}

#[repr(C)]
pub struct NetworkingInformation {
    pub typ: TagType,   // 16
    pub size: u32,
    pub dhcp_ack: DHCPACK,
}
#[repr(C)]
pub struct DHCPACK {
    // TODO
}

/// This tag contains EFI memory map as per EFI specification.
/// This tag may not be provided by some bootloaders on EFI platforms if EFI boot services are
/// enabled and available for the loaded image (EFI boot services not terminated tag exists in
/// Multiboot2 information structure).
#[repr(C)]
pub struct EFIMemoryMap {
    pub typ: TagType,   // 17
    pub size: u32,
    pub descriptor_size: u32,
    pub descriptor_version: u32,
    pub efi_memory_map: EFIMemoryMapEntry,
}
#[repr(C)]
pub struct EFIMemoryMapEntry {
    // TODO
}

/// This tag indicates ExitBootServices wasn't called
#[repr(C)]
pub struct EFIBootServicesNotTerminated {
    pub typ: TagType,   // 18
    pub size: u32,      // 8
}

/// This tag contains pointer to EFI i386 image handle. Usually it is bootloader image handle.
#[repr(C)]
pub struct EFI32BitImageHandlePointer {
    pub typ: TagType,   // 19
    pub size: u32,      // 12
    pub pointer: u32,
}

/// This tag contains pointer to EFI amd64 image handle. Usually it is bootloader image handle.
#[repr(C)]
pub struct EFI64BitImageHandlePointer {
    pub typ: TagType,   // 20
    pub size: u32,      // 16
    pub pointer: u64,
}

/// This tag contains image load base physical address.
/// It is provided only if image has relocatable header tag.
#[repr(C)]
pub struct ImageLoadBasePhysicalAddress {
    pub typ: TagType,   // 21
    pub size: u32,      // 12
    pub load_base_addr: u32,
}