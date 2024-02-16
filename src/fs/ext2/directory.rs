use alloc::string::String;
use core::ffi::c_void;
use core::mem::{MaybeUninit, size_of};
use volatile_register::RO;
use crate::{print, println};

#[repr(C)]
pub(crate) struct DirectoryEntry {
    /// 32bit inode number of the file entry. A value of 0 indicate that the entry is not used.
    pub(crate) inode: RO<u32>,
    /// 16bit unsigned displacement to the next directory entry from the start of the current directory entry. This
    /// field must have a value at least equal to the length of the current record.
    /// The directory entries must be aligned on 4 bytes boundaries and there cannot be any directory entry span-
    /// ning multiple data blocks. If an entry cannot completely fit in one block, it must be pushed to the next data
    /// block and the rec_len of the previous entry properly adjusted.
    pub(crate) rec_len: RO<u16>,
    /// 8bit unsigned value indicating how many bytes of character data are contained in the name.
    pub(crate) name_len: RO<u8>,
    /// 8bit unsigned value used to indicate file type.
    pub(crate) file_type: RO<FileType>,
    /// Name of the entry. The ISO-Latin-1 character set is expected in most system. The name must be no longer
    /// than 255 bytes after encoding.
    pub(crate) name: RO<[u8; 255]>,
}
impl DirectoryEntry {
    pub(crate) fn name(&self) {
        print!(" - ");
        self.name.read()[0..(self.name_len.read() as usize)].iter().for_each(|&c| print!("{}", c as char));
        println!("");
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum FileType {
    Unknown = 0,
    RegularFile = 1,
    Directory = 2,
    CharacterDevice = 3,
    BlockDevice = 4,
    Buffer = 5,
    Socket = 6,
    SymbolicLink = 7,
}