// Read https://nongnu.askapache.com/grub/phcoder/multiboot.pdf

pub mod structures;

const END_TAG_SIZE: u32 = 8;

#[repr(C)]
pub struct BootInformation {
    pub total_size: u32,
    pub _reserved: u32,
    pub first_tag: Tag,
}

#[repr(C)]
pub struct Tag {
    pub tag_type: TagType,
    pub size: u32,
}

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
}

pub unsafe fn load_boot_information(address: usize) -> &'static BootInformation {
    let multiboot = &*(address as *const BootInformation);
    multiboot
}

impl BootInformation {
    pub fn start_address(&self) -> usize {
        self as *const _ as usize
    }
    pub fn end_address(&self) -> usize {
        self.start_address() + self.total_size as usize
    }

    pub fn memory_map_tag(&self) -> Option<&'static structures::memory_map::MemoryMapTag> {
        self.get_tag(TagType::MemoryMap).map(|tag| unsafe{&*(tag as *const Tag as *const structures::memory_map::MemoryMapTag)})
    }

    fn get_tag(&self, typ: TagType) -> Option<&'static Tag> {
        self.tags().find(|tag| tag.tag_type == typ)
    }

    fn tags(&self) -> TagIterator {
        TagIterator{current: &self.first_tag as *const _}
    }
}

pub struct TagIterator {
    current: *const Tag,
}

impl Iterator for TagIterator {
    type Item = &'static Tag;

    fn next(&mut self) -> Option<&'static Tag> {
        match unsafe{&*self.current} {
            &Tag{ tag_type: TagType::End, size: END_TAG_SIZE} => None, // end tag
            tag => {
                // go to next tag
                let mut tag_addr = self.current as usize;
                tag_addr += tag.size as usize;
                tag_addr = ((tag_addr-1) & !0x7) + 0x8; //align at 8 byte
                self.current = tag_addr as *const _;

                Some(tag)
            },
        }
    }
}
