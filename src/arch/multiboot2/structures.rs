use crate::arch::multiboot2::TagType;

#[repr(C)]
pub struct BootCommandLine {
    typ: TagType,
    size: u32,
    command_line: [u8],
}

#[repr(C)]
pub struct BasicMemoryInformation {
    typ: TagType,
    size: u32,
    mem_lower: u32,
    mem_upper: u32,
}

#[repr(C)]
pub struct BiosBootDevice {
    typ: TagType,
    size: u32,
    biosdev: u32,
    partition: u32,
    sub_partition: u32,
}

#[repr(C)]
pub struct MemoryMapTag {
    typ: TagType,
    size: u32,
    entry_size: u32,
    entry_version: u32,
    first_area: MemoryArea,
}

impl MemoryMapTag {
    pub fn memory_areas(&self) -> MemoryAreaIter {
        let self_ptr = self as *const MemoryMapTag;
        let start_area = (&self.first_area) as *const MemoryArea;
        MemoryAreaIter {
            current_area: start_area,
            last_area: ((self_ptr as u32) + self.size - self.entry_size) as *const MemoryArea,
            entry_size: self.entry_size,
        }
    }
}

#[repr(C)]
pub struct MemoryArea {
    pub base_addr: u64,
    pub length: u64,
    typ: u32,
    _reserved: u32,
}

#[derive(Clone)]
pub struct MemoryAreaIter {
    pub current_area: *const MemoryArea,
    pub last_area: *const MemoryArea,
    pub entry_size: u32,
}

impl Iterator for MemoryAreaIter {
    type Item = &'static MemoryArea;
    fn next(&mut self) -> Option<&'static MemoryArea> {
        if self.current_area > self.last_area {
            None
        } else {
            let area = unsafe{&*self.current_area};
            self.current_area = ((self.current_area as u32) + self.entry_size)
                as *const MemoryArea;
            if area.typ == 1 {
                Some(area)
            } else {self.next()}
        }
    }
}