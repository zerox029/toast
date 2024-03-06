use bitflags::bitflags;
use crate::memory::{Frame, VirtualAddress};

pub struct Entry(pub(crate) VirtualAddress);

impl Entry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(EntryFlags::PRESENT) {
            Some(Frame::containing_address(
                self.0 & 0x000fffff_fffff000
            ))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        assert!(frame.start_address() & !0x000fffff_fffff000 == 0);
        self.0 = frame.start_address() | flags.bits();
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct EntryFlags: usize {
        const PRESENT =         1 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const NO_EXECUTE =      1 << 63;
    }
}

impl EntryFlags {
    /*
    pub fn from_elf_section_flags(section: &ElfSectionHeader) -> EntryFlags {
        let mut flags = EntryFlags::empty();

        if section.flags().contains(ElfSectionHeaderFlags::ALLOCATED) {
            flags |= EntryFlags::PRESENT;
        }
        if section.flags().contains(ElfSectionHeaderFlags::WRITABLE) {
            flags |= EntryFlags::WRITABLE;
        }
        if !section.flags().contains(ElfSectionHeaderFlags::EXECUTABLE) {
            flags |= EntryFlags::NO_EXECUTE;
        }

        flags |= EntryFlags::USER_ACCESSIBLE;
        flags |= EntryFlags::WRITABLE;

        flags
    }*/
}