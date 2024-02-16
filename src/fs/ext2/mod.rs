// https://www.nongnu.org/ext2-doc/ext2.pdf

mod block;
mod inode;
mod directory;

use crate::drivers::pci::ahci::AHCIDevice;
use crate::{println, print, serial_println};
use crate::fs::ext2::block::{Superblock};
use crate::fs::ext2::inode::{Inode, InodeMode};
use crate::memory::MemoryManagementUnit;

pub fn mount_filesystem(mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice) {
    println!("ext2: mounting file system...");

    let superblock = Superblock::read_from_disk(mmu, drive);
    let root_inode = Inode::get_from_id(mmu, drive, &superblock, 2);

    println!("Directory \"/\" contains the following files:");
    root_inode.print_content(mmu, drive, &superblock);
}