// https://www.nongnu.org/ext2-doc/ext2.pdf

mod block;
mod inode;

use crate::drivers::pci::ahci::AHCIDevice;
use crate::{println, print, serial_println};
use crate::fs::ext2::block::{BlockGroupDescriptorTable, Superblock};
use crate::memory::MemoryManagementUnit;

const EXT2_SIGNATURE: u16 = 0xEF53;

#[repr(C)]
struct InodeTable {
    mode: u16,
    uid: u16,
    size: u32,
    atime: u32,
    ctime: u32,
    mtime: u32,
    dtime: u32,
    gid: u16,
    links_count: u16,
    blocks: u32,
    flags: u32,
    osd1: u32,
    block: [u32; 15],
    generation: u32,
    file_acl: u32,
    dir_acl: u32,
    faddr: u32,
    osd2: [u8; 12],
}


#[repr(u16)]
enum InodeMode {
    // File format
    Ifsock = 0xC000,
    Iflnk = 0xA000,
    Ifreg = 0x8000,
    Ifblk = 0x6000,
    Ifdir = 0x4000,
    Ifchr = 0x2000,
    Ififo = 0x1000,

    // Process execution user / group override
    Isuid = 0x0800,
    Isgid = 0x0400,
    Isvtx = 0x0200,

    // Access rights
    Irusr = 0x0100,
    Iwusr = 0x0080,
    Ixusr = 0x0040,
    Irgrp = 0x0020,
    Iwgrp = 0x0010,
    Ixgrp = 0x0008,
    Iroth = 0x0004,
    Iwoth = 0x0002,
    Ixoth = 0x0001,
}

pub fn mount_filesystem(mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice) {
    println!("ext2: mounting file system...");

    let superblock = Superblock::read_from_disk(mmu, drive);
    let block_group_descriptor_table = BlockGroupDescriptorTable::read_from_disk(mmu, drive, &superblock);

    serial_println!("ext2: {} free blocks", block_group_descriptor_table.0[0].unallocated_block_count);
}