use core::ffi::c_void;
use core::mem::{MaybeUninit, size_of};
use bitflags::bitflags;
use volatile_register::RO;
use crate::drivers::pci::ahci::AHCIDevice;

const EXT2_SIGNATURE: u16 = 0xEF53;
const SUPERBLOCK_OFFSET: u16 = 1024;

#[repr(C)]
pub(crate) struct Superblock {
    /// 32bit value indicating the total number of inodes, both used and free, in the file system
    pub(crate) inode_count: RO<u32>,
    /// 32bit value indicating the total number of blocks in the system including all used, free and reserved.
    pub(crate) block_count: RO<u32>,
    /// 32bit value indicating the total number of blocks reserved for the usage of the super user.
    pub(crate) superuser_blocks: RO<u32>,
    /// 32bit value indicating the total number of free blocks, including the number of reserved blocks (see
    /// s_r_blocks_count). This is a sum of all free blocks of all the block groups.
    pub(crate) unallocated_blocks: RO<u32>,
    /// 32bit value indicating the total number of free inodes. This is a sum of all free inodes of all the block groups.
    pub(crate) unallocated_inodes: RO<u32>,
    /// 32bit value identifying the first data block, in other word the id of the block containing the superblock
    /// structure.
    pub(crate) superblock_block_number: RO<u32>,
    /// The block size is computed using this 32bit value as the number of bits to shift left the value 1024. This
    /// value may only be non-negative.
    pub(crate) log_block_size: RO<u32>,
    /// The fragment size is computed using this 32bit value as the number of bits to shift left the value 1024.
    /// Note that a negative value would shift the bit right rather than left.
    pub(crate) log_fragment_size: RO<u32>,
    /// 32bit value indicating the total number of blocks per group.
    pub(crate) block_group_block_count: RO<u32>,
    /// 32bit value indicating the total number of fragments per group. It is also used to determine the size of the
    /// block bitmap of each block group.
    pub(crate) block_group_fragment_count: RO<u32>,
    /// 32bit value indicating the total number of inodes per group. This is also used to determine the size of the
    /// inode bitmap of each block group.
    pub(crate) block_group_inode_count: RO<u32>,
    /// Unix time, as defined by POSIX, of the last time the file system was mounted.
    pub(crate) last_mount_time: RO<u32>,
    /// Unix time, as defined by POSIX, of the last write access to the file system.
    pub(crate) last_write_time: RO<u32>,
    /// 16bit value indicating how many time the file system was mounted since the last time it was fully verified.
    pub(crate) mount_count: RO<u16>,
    /// 16bit value indicating the maximum number of times that the file system may be mounted before a full
    /// check is performed.
    pub(crate) allowed_mount_count: RO<u16>,
    /// 16bit value identifying the file system as Ext2. The value is currently fixed to EXT2_SUPER_MAGIC of
    /// value 0xEF53.
    pub(crate) ext2_signature: RO<u16>,
    /// 16bit value indicating the file system state. When the file system is mounted, this state is set to EXT2_ER-
    /// ROR_FS. After the file system was cleanly unmounted, this value is set to EXT2_VALID_FS.
    /// When mounting the file system, if a valid of EXT2_ERROR_FS is encountered it means the file system
    /// was not cleanly unmounted and most likely contain errors that will need to be fixed. Typically under Linux
    /// this means running fsck.
    pub(crate) file_system_state: RO<FileSystemState>,
    /// 16bit value indicating what the file system driver should do when an error is detected
    pub(crate) error_detection_mechanism: RO<ErrorHandlingMethod>,
    /// 16bit value identifying the minor revision level within its revision level
    pub(crate) version_minor: RO<u16>,
    /// Unix time, as defined by POSIX, of the last file system check.
    pub(crate) last_consistency_check_time: RO<u32>,
    /// Maximum Unix time interval, as defined by POSIX, allowed between file system checks.
    pub(crate) consistency_check_interval: RO<u32>,
    /// 32bit identifier of the os that created the file system
    pub(crate) creator_os_id: RO<CreatorOSId>,
    /// 32bit revision level value.
    pub(crate) version_major: RO<RevisionLevel>,
    /// 16bit value used as the default user id for reserved blocks.
    pub(crate) reserved_block_user_id: RO<u16>,
    /// 16bit value used as the default group id for reserved blocks.
    pub(crate) reserved_block_group_id: RO<u16>,


    // EXT2_DYNAMIC_REV specific
    /// 32bit value used as index to the first inode useable for standard files. In revision 0, the first non-reserved
    /// inode is fixed to 11 (EXT2_GOOD_OLD_FIRST_INO). In revision 1 and later this value may be set to
    /// any value.
    pub(crate) first_non_reserved_inode: RO<u32>,
    /// 16bit value indicating the size of the inode structure. In revision 0, this value is always 128 (EX-
    /// T2_GOOD_OLD_INODE_SIZE). In revision 1 and later, this value must be a perfect power of 2 and must
    /// be smaller or equal to the block size (1<<s_log_block_size).
    pub(crate) inode_byte_size: RO<u16>,
    /// 16bit value used to indicate the block group number hosting this superblock structure. This can be used
    /// to rebuild the file system from any superblock backup.
    pub(crate) containing_block_group: RO<u16>,
    /// 32bit bitmask of compatible features. The file system implementation is free to support them or not without
    /// risk of damaging the meta-data.
    pub(crate) compatible_features: RO<CompatibleFeatures>,
    /// 32bit bitmask of incompatible features. The file system implementation should refuse to mount the file
    /// system if any of the indicated feature is unsupported.
    pub(crate) incompatible_features: RO<IncompatibleFeatures>,
    /// 32bit bitmask of “read-only” features. The file system implementation should mount as read-only if any
    /// of the indicated feature is unsupported.
    pub(crate) read_only_compatible_features: RO<ReadOnlyCompatibleFeatures>,
    /// 128bit value used as the volume id. This should, as much as possible, be unique for each file system
    /// formatted.
    pub(crate) file_system_id: RO<[u8; 16]>,
    /// 16 bytes volume name, mostly unusued. A valid volume name would consist of only ISO-Latin-1
    /// characters and be 0 terminated.
    pub(crate) volume_name: RO<[u8; 16]>,
    /// 64 bytes directory path where the file system was last mounted. While not normally used, it could serve
    /// for auto-finding the mountpoint when not indicated on the command line. Again the path should be zero
    /// terminated for compatibility reasons. Valid path is constructed from ISO-Latin-1 characters.
    pub(crate) last_mounted_path: RO<[u8; 64]>,
    /// 32bit value used by compression algorithms to determine the compression method(s) used
    pub(crate) compression_algorithm: RO<CompressionAlgorithm>,

    // Performance Hints
    /// 8-bit value representing the number of blocks the implementation should attempt to pre-allocate when
    /// creating a new regular file.
    pub(crate) preallocated_block_number_file: RO<u8>,
    /// 8-bit value representing the number of blocks the implementation should attempt to pre-allocate when
    /// creating a new directory.
    pub(crate) preallocated_block_number_directory: RO<u8>,
    _alignment: RO<u16>,


    // Journaling Support
    /// 16-byte value containing the uuid of the journal superblock. See Ext3 Journaling for more information.
    pub(crate) journal_id: RO<[u8; 16]>,
    /// 32-bit inode number of the journal file. See Ext3 Journaling for more information.
    pub(crate) journal_inode: RO<u32>,
    /// 32-bit device number of the journal file. See Ext3 Journaling for more information.
    pub(crate) journal_device: RO<u32>,
    /// 32-bit inode number, pointing to the first inode in the list of inodes to delete. See Ext3 Journaling for
    /// more information.
    pub(crate) orphan_inode_list_head: RO<u32>,


    // Directory Indexing support
    /// An array of 4 32bit values containing the seeds used for the hash algorithm for directory indexing.
    pub(crate) hash_seed: RO<[u32; 4]>,
    /// An 8bit value containing the default hash version used for directory indexing.
    pub(crate) hash_version: RO<u8>,
    _padding: RO<[u8; 3]>,

    //Other options
    /// A 32bit value containing the default mount options for this file system
    pub(crate) default_mount_options: RO<u32>,
    /// A 32bit value indicating the block group ID of the first meta block group
    pub(crate) first_meta_bg: RO<u32>,
    _unused: RO<[u8; 760]>,
}
impl Superblock {
    pub(crate) fn read_from_disk(drive: &mut AHCIDevice) -> Superblock {
        let mut superblock = MaybeUninit::<Superblock>::uninit();

        drive.read_from_device(SUPERBLOCK_OFFSET as u64, size_of::<Superblock>() as u64, superblock.as_mut_ptr() as *mut c_void);
        let superblock = unsafe { superblock.assume_init() };

        assert_eq!(superblock.ext2_signature.read(), EXT2_SIGNATURE);

        superblock
    }

    pub(crate) fn block_group_count(&self) -> usize {
        let count_from_blocks = self.block_count.read().div_ceil(self.block_group_block_count.read()) as usize;
        let count_from_inodes = self.inode_count.read().div_ceil(self.block_group_inode_count.read()) as usize;

        assert_eq!(count_from_blocks, count_from_inodes);

        count_from_blocks
    }

    pub(crate) fn inode_size(&self) -> u16 {
        match self.version_major.read() {
            RevisionLevel::GoodOldRevision => 128,
            RevisionLevel::Dynamicrevision => self.inode_byte_size.read()
        }
    }

    pub(crate) fn block_size_bytes(&self) -> usize {
        1024 << self.log_block_size.read()
    }
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub(crate) enum FileSystemState {
    Clean = 1,
    Error = 2,
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub(crate) enum ErrorHandlingMethod {
    Ignore = 1,
    RemountReadOnly = 2,
    KernelPanic = 3,
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub(crate) enum CreatorOSId {
    Linux = 0,
    GnuHurd = 1,
    Masix = 2,
    FreeBSD = 3,
    OtherLite = 4,
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub(crate) enum RevisionLevel {
    GoodOldRevision = 0,
    Dynamicrevision = 1,
}

bitflags! {
    #[derive(Copy, Clone)]
    pub(crate) struct CompatibleFeatures: u32 {
        const DIR_PREALLOC = 1 << 0;
        const IMAGIC_INODES = 1 << 1;
        const HAS_JOURNAL = 1 << 2;
        const EXT_ATTR = 1 << 3;
        const RESIZE_INO = 1 << 4;
        const DIR_INDEX = 1 << 5;
    }

    #[derive(Copy, Clone)]
    pub(crate) struct IncompatibleFeatures: u32 {
        const COMPRESSION = 1 << 0;
        const FILETYPE = 1 << 1;
        const RECOVER = 1 << 2;
        const JOURNAL_DEV = 1 << 3;
        const META_BG = 1 << 4;
    }

    #[derive(Copy, Clone)]
    pub(crate) struct ReadOnlyCompatibleFeatures: u32 {
        const SPARSE_SUPER = 1 << 0;
        const LARGE_FILE = 1 << 1;
        const BTREE_DIR = 1 << 4;
    }

    #[derive(Copy, Clone)]
    pub(crate) struct CompressionAlgorithm: u32 {
        const LZV1 = 1 << 0;
        const LZRW3A = 1 << 1;
        const GZIP_ALG = 1 << 2;
        const BZIP2 = 1 << 3;
        const LZO = 1 << 4;
    }
}

#[repr(C)]
pub(crate) struct BlockGroupDescriptor {
    /// 32bit block id of the first block of the “block bitmap” for the group represented.
    /// The actual block bitmap is located within its own allocated blocks starting at the block ID specified by
    /// this value.
    pub(crate) block_bitmap: RO<u32>,
    /// 32bit block id of the first block of the “inode bitmap” for the group represented.
    pub(crate) inode_usage_bitmap_address: RO<u32>,
    /// 32bit block id of the first block of the “inode table” for the group represented
    pub(crate) inode_table_block_address: RO<u32>,
    /// 16bit value indicating the total number of free blocks for the represented group.
    pub(crate) unallocated_block_count: RO<u16>,
    /// 16bit value indicating the total number of free inodes for the represented group.
    pub(crate) unallocated_inode_count: RO<u16>,
    /// 16bit value indicating the number of inodes allocated to directories for the represented group.
    pub(crate) directory_count: RO<u16>,

    /// 16bit value used for padding the structure on a 32bit boundary.
    _pad: RO<u16>,
    /// 12 bytes of reserved space for future revisions
    _reserved: RO<[u8; 12]>,
}

impl BlockGroupDescriptor {
    pub(crate) fn read_table_entry(drive: &mut AHCIDevice, superblock: &Superblock, index: usize) -> Self {
        let first_entry_address = (1024 << superblock.log_block_size.read()) * if superblock.log_block_size.read() == 0 { 2 } else { 1 };
        let offset = first_entry_address + index * size_of::<BlockGroupDescriptor>();

        let mut entry = MaybeUninit::<BlockGroupDescriptor>::uninit();
        drive.read_from_device(offset as u64, size_of::<BlockGroupDescriptor>() as u64, entry.as_mut_ptr() as *mut c_void);
        unsafe { entry.assume_init() }
    }
}