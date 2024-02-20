// https://www.nongnu.org/ext2-doc/ext2.pdf

mod block;
mod inode;
mod directory;

use alloc::vec::Vec;
use core::ffi::c_void;
use core::ops::ControlFlow;
use crate::drivers::pci::ahci::AHCIDevice;
use crate::{println, print};
use crate::fs::ext2::block::{Superblock};
use crate::fs::ext2::inode::{Inode};
use crate::memory::MemoryManagementUnit;

const ROOT_INODE_ID: usize = 2;

pub struct Ext2FileSystem {
    pub superblock: Superblock,
    pub root_inode: Inode,
}
impl Ext2FileSystem {
    /// Checks whether a certain file is present on the current file system and returns its inode if it is.
    /// The provided path needs to be absolute relative to the current file system.
    pub fn find_file(&self, mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice, path: &str) -> Option<Inode> {
        if path.as_bytes()[0] != b'/' {
            panic!("ext2: expected an absolute path");
        }

        let mut path_iter = path[1..].split('/');

        // This manual first iteration necessary to avoid ownership issues and since Inodes cannot be cloned
        // There might be a better way though, but I haven't found it
        let first_name = path_iter.next().unwrap();
        let current_inode = self.root_inode.find_child_inode(mmu, drive, &self.superblock, first_name).unwrap();

        let inode = path_iter.try_fold(current_inode, |current_inode, current_name| {
            if let Some(found_inode) = current_inode.find_child_inode(mmu, drive, &self.superblock, current_name) {
                ControlFlow::Continue(found_inode)
            }
            else {
                ControlFlow::Break(())
            }
        });

        match inode {
            ControlFlow::Continue(inode) => Some(inode),
            ControlFlow::Break(()) => None,
        }
    }

    /// Checks whether a certain file is present on the current file system.
    /// The provided path needs to be absolute relative to the current file system.
    pub fn is_file_present(&self, mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice, path: &str) -> bool {
        self.find_file(mmu, drive, path).is_some()
    }

    /// Retrieves the given inode and returns its contents
    pub fn get_file_contents(&self, mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice, path: &str) -> Option<Vec<u8>> {
        let inode = self.find_file(mmu, drive, path);

        match inode {
            Some(inode) => Some(inode.get_content(mmu, drive, &self.superblock)),
            None => None,
        }
    }
}

pub fn mount_filesystem(mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice) -> Ext2FileSystem {
    println!("ext2: mounting file system...");

    let superblock = Superblock::read_from_disk(mmu, drive);
    let root_inode = Inode::get_from_id(mmu, drive, &superblock, ROOT_INODE_ID);

    Ext2FileSystem {
        superblock,
        root_inode
    }
}
