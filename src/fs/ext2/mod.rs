// https://www.nongnu.org/ext2-doc/ext2.pdf

mod block;
mod inode;

use crate::drivers::pci::ahci::AHCIDevice;
use crate::{println, print, serial_println};
use crate::fs::ext2::block::{BlockGroupDescriptor, Superblock};
use crate::fs::ext2::inode::Inode;
use crate::memory::MemoryManagementUnit;

pub fn mount_filesystem(mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice) {
    println!("ext2: mounting file system...");

    let superblock = Superblock::read_from_disk(mmu, drive);
    let root_inode = Inode::read_from_disk(mmu, drive, &superblock, 2);

    serial_println!("{}", root_inode.mtime.read());
    serial_println!("{}", root_inode.atime.read());
}