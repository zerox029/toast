use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::mem::{MaybeUninit, size_of};
use bitflags::bitflags;
use volatile_register::RO;
use crate::drivers::pci::ahci::AHCIDevice;
use crate::fs::ext2::block::{BlockGroupDescriptor, Superblock};
use crate::fs::ext2::directory::{DirectoryEntry};
use crate::memory::MemoryManagementUnit;
use crate::{print, println};

#[repr(C)]
pub(crate) struct Inode {
    /// 16bit value used to indicate the format of the described file and the access rights.
    pub(crate) mode: RO<InodeMode>,
    /// 16bit user id associated with the file.
    pub(crate) uid: RO<u16>,
    /// In revision 0, (signed) 32bit value indicating the size of the file in bytes. In revision 1 and later revisions,
    /// and only for regular files, this represents the lower 32-bit of the file size; the upper 32-bit is located in
    /// the dir_acl.
    pub(crate) size: RO<u32>,
    /// 32bit value representing the number of seconds since january 1st 1970 of the last time this inode was
    /// accessed.
    pub(crate) atime: RO<u32>,
    /// 32bit value representing the number of seconds since january 1st 1970, of when the inode was created.
    pub(crate) ctime: RO<u32>,
    /// 32bit value representing the number of seconds since january 1st 1970, of the last time this inode was
    /// modified.
    pub(crate) mtime: RO<u32>,
    /// 32bit value representing the number of seconds since january 1st 1970, of when the inode was deleted.
    pub(crate) dtime: RO<u32>,
    /// 16bit value of the POSIX gROup having access to this file.
    pub(crate) gid: RO<u16>,
    /// 16bit value indicating how many times this particular inode is linked (referred to). Most files will have a
    /// link count of 1. Files with hard links pointing to them will have an additional count for each hard link.
    pub(crate) links_count: RO<u16>,
    /// 32-bit value representing the total number of 512-bytes blocks reserved to contain the data of this inode,
    /// regardless if these blocks are used or not. The block numbers of these reserved blocks are contained in
    /// the i_block array.
    pub(crate) blocks: RO<u32>,
    /// 32bit value indicating how the ext2 implementation should behave when accessing the data for this inode.
    pub(crate) flags: RO<InodeFlags>,
    /// 32bit OS dependant value.
    pub(crate) osd1: RO<u32>,
    /// 15 x 32bit block numbers pointing to the blocks containing the data for this inode. The first 12 blocks
    /// are direct blocks. The 13th entry in this array is the block number of the first indirect block; which is a
    /// block containing an array of block ID containing the data. Therefore, the 13th block of the file will be the
    /// first block ID contained in the indirect block. With a 1KiB block size, blocks 13 to 268 of the file data
    /// are contained in this indirect block.
    pub(crate) block: RO<[u32; 15]>,
    /// 32bit value used to indicate the file version (used by NFS).
    pub(crate) generation: RO<u32>,
    /// 32bit value indicating the block number containing the extended attributes. In revision 0 this value is
    /// always 0.
    pub(crate) file_acl: RO<u32>,
    /// In revision 0 this 32bit value is always 0. In revision 1, for regular files this 32bit value contains the high
    /// 32 bits of the 64bit file size.
    pub(crate) dir_acl: RO<u32>,
    /// 32bit value indicating the location of the file fragment.
    pub(crate) faddr: RO<u32>,
    /// 96bit OS dependant structure.
    pub(crate) osd2: RO<[u8; 12]>,
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub(crate) struct InodeMode: u16 {
        // File format
        const SOCKET = 0xC000;
        const SYMBOLIC_LINK = 0xA000;
        const REGULAR_FILE = 0x8000;
        const BLOCK_DEVICE = 0x6000;
        const DIRECTORY = 0x4000;
        const CHARACTER_DEVICE = 0x2000;
        const FIFO = 0x1000;

        // Process execution user / Group override
        const SET_PROCESS_USER_ID = 0x0800;
        const SET_PROCESS_GROUP_ID = 0x0400;
        const STICK_BIT = 0x0200;

        // Access rights
        const USER_READ = 0x0100;
        const USER_WRITE = 0x0080;
        const USER_EXECUTE = 0x0040;
        const GROUP_READ = 0x0020;
        const GROUP_WRITE = 0x0010;
        const GROUP_EXECUTE = 0x0008;
        const OTHERS_READ = 0x0004;
        const OTHERS_WRITE = 0x0002;
        const OTHERS_EXECUTE = 0x0001;
    }

    #[derive(Copy, Clone)]
    pub(crate) struct InodeFlags: u32 {
        const SECRM = 1 << 0;
        const UNRM = 1 << 1;
        const COMPR = 1 << 2;
        const SYNC = 1 << 3;
        const IMMUTABLE = 1 << 4;
        const APPEND = 1 << 5;
        const NODUMP = 1 << 6;
        const NOATIME = 1 << 7;

        // Reserved for compression usage
        const DIRTY = 1 << 8;
        const COMPRBLK = 1 << 9;
        const NOCOMPR = 1 << 10;
        const ECOMPR = 1 << 11;

        // End of compression flags
        const BTREE = 1 << 12;
        const INDEX = 1 << 13;
        const IMAGIC = 1 << 14;
        const JOURNAL_DATA = 1 << 15;
        const RESERVED = 1 << 31;
    }
}

impl Inode {
    pub(crate) fn get_from_id(mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice, superblock: &Superblock, inode_id: usize) -> Self {
        let group_id = Inode::get_containing_block_group_id(superblock, inode_id);
        let inode_index = Self::get_local_table_index(superblock, inode_id);

        let block_group_descriptor = BlockGroupDescriptor::read_table_entry(mmu, drive, superblock, group_id);
        let table_address = block_group_descriptor.inode_table_block_address.read();

        let containing_block = inode_index * superblock.inode_size() as usize / (1024 << superblock.log_block_size.read()) as usize;

        let inode_address = table_address as usize + containing_block; // block
        let inode_address_bytes = inode_address * (1024 << superblock.log_block_size.read()) + inode_index * superblock.inode_size() as usize;

        let mut inode = MaybeUninit::<Inode>::uninit();
        drive.read_from_device(mmu, inode_address_bytes as u64, size_of::<Inode>() as u64, inode.as_mut_ptr() as *mut c_void);
        unsafe { inode.assume_init() }
    }

    pub(crate) fn print_content(&self, mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice, superblock: &Superblock) {
        let initial_address = self.block.read()[0] * (1024 << superblock.log_block_size.read());
        let mut file_address = initial_address;

        // TODO: Support multi block files
        // Read the content of the pointed block
        loop {
            let mut file = MaybeUninit::<DirectoryEntry>::uninit();
            drive.read_from_device(mmu, file_address as u64, size_of::<DirectoryEntry>() as u64, file.as_mut_ptr() as *mut c_void);
            let file = unsafe { file.assume_init() };

            file.name();
            print!(" ");

            file_address += file.rec_len.read() as u32;

            // Break if the next file is outside the current block
            if file_address - initial_address >= (1024 << superblock.log_block_size.read()) {
                break;
            }
        }

        println!("");
    }

    /// Looks for an inode with the given name in the current inode's children.
    /// Returns None if the requested Inode was not present
    pub(crate) fn find_child_inode(&self, mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice, superblock: &Superblock, name: &str) -> Option<Inode> {
        if matches!(self.mode.read(), InodeMode::DIRECTORY) {
            panic!("ext2: not a directory")
        }

        let mut inode_data = self.get_content(mmu, drive, superblock);

        let mut read_bytes = 0;
        while read_bytes < inode_data.len() {
            let directory_entry_pointer = (inode_data.as_mut_ptr() as usize + read_bytes) as *mut DirectoryEntry;
            let directory_entry = unsafe { &*directory_entry_pointer };

            if directory_entry.name() == name {
                return Some(Self::get_from_id(mmu, drive, superblock, directory_entry.inode.read() as usize));
            }

            read_bytes += directory_entry.rec_len.read() as usize;
        }


        None
    }

    pub(crate) fn get_content(&self, mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice, superblock: &Superblock) -> Vec<u8> {
        let file_start_address = self.block.read()[0] as usize * superblock.block_size_bytes();

        let mut inode_data = vec![0u8; self.size.read() as usize];
        for block_number in 0..self.adjusted_block_count(superblock) {
            // First 12 blocks, direct indexing
            if block_number < 12 {
                let write_address = (inode_data.as_mut_ptr() as usize + block_number * superblock.block_size_bytes()) as *mut c_void;
                drive.read_from_device(mmu, file_start_address as u64, size_of::<DirectoryEntry>() as u64, write_address);
            }

            // 13th block, indirect indexing
            else if block_number == 12 {
                unimplemented!();
            }

            // 14th block, doubly indirect indexing
            else if block_number == 13 {
                unimplemented!();
            }

            // 15h block, triply indirect indexing
            else if block_number == 14 {
                unimplemented!();
            }
        }

        inode_data
    }

    fn get_containing_block_group_id(superblock: &Superblock, inode_id: usize) -> usize {
        (inode_id - 1) / superblock.block_group_inode_count.read() as usize
    }

    fn get_local_table_index(superblock: &Superblock, inode_id: usize) -> usize {
        (inode_id - 1) % superblock.block_group_inode_count.read() as usize
    }

    fn adjusted_block_count(&self, superblock: &Superblock) -> usize {
        (self.blocks.read() as usize * 512) / superblock.block_size_bytes()
    }
}