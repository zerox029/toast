// https://www.nongnu.org/ext2-doc/ext2.pdf

use core::ffi::c_void;
use core::mem::{MaybeUninit, size_of};
use bitflags::bitflags;
use crate::drivers::pci::ahci::AHCIDevice;
use crate::{println, print};
use crate::memory::MemoryManagementUnit;

const EXT2_SIGNATURE: u16 = 0xEF53;
const SUPERBLOCK_OFFSET: u16 = 1024;

#[repr(C)]
struct Superblock {
    inode_count: u32,
    block_count: u32,
    superuser_blocks: u32,
    unallocated_blocks: u32,
    unallocated_inodes: u32,
    superblock_block_number: u32,
    log_block_size: u32,
    log_fragment_size: u32,
    block_group_block_count: u32,
    block_group_fragment_count: u32,
    block_group_inode_count: u32,
    last_mount_time: u32,
    last_write_time: u32,
    mount_count: u16,
    allowed_mount_count: u16,
    ext2_signature: u16,
    file_system_state: FileSystemState,
    error_detection_mechanism: ErrorHandlingMethod,
    version_minor: u16,
    last_consistency_check_time: u32,
    consistency_check_interval: u32,
    creator_os_id: CreatorOSId,
    version_major: RevisionLevel,
    reserved_block_user_id: u16,
    reserved_block_group_id: u16,

    // EXT2_DYNAMIC_REV specific
    first_non_reserved_inode: u32,
    inode_byte_size: u16,
    containing_block_group: u16,
    compatible_features: CompatibleFeatures,
    incompatible_features: IncompatibleFeatures,
    read_only_compatible_features: ReadOnlyCompatibleFeatures,
    file_system_id: [u8; 16],
    volume_name: [u8; 16],
    last_mounted_path: SuperblockLastMountedPath,
    compression_algorithm: CompressionAlgorithm,

    // Performance Hints
    preallocated_block_number_file: u8,
    preallocated_block_number_directory: u8,
    _alignment: u16,

    // Journaling Support
    journal_id: [u8; 16],
    journal_inode: u32,
    journal_device: u32,
    orphan_inode_list_head: u32,

    // Directory Indexing support
    hash_seed: [u32; 4],
    hash_version: u8,
    _padding: [u8; 3],

    //Other options
    default_mount_options: u32,
    first_meta_bg: u32,
    _unused: UnusedSuperblockSpace,
}

#[repr(C)]
struct BlockGroupDescriptorTable {
    block_group_descriptors: [BlockGroupDescriptor; 1],
}

#[repr(C)]
struct BlockGroupDescriptor {
    block_usage_bitmap_address: u32,
    inode_usage_bitmap_address: u32,
    inode_table_block_address: u32,
    unallocated_block_count: u16,
    unallocated_inode_count: u16,
    directory_count: u16,
    _unused: [u8; 31 - 18 + 1],
}

struct UnusedSuperblockSpace([u8; 760]);
impl Default for UnusedSuperblockSpace {
    fn default() -> Self {
        UnusedSuperblockSpace{0: [0; 760]}
    }
}

struct SuperblockLastMountedPath([u8; 64]);
impl Default for SuperblockLastMountedPath {
    fn default() -> Self {
        SuperblockLastMountedPath {0: [0; 64]}
    }
}

#[repr(u16)]
enum FileSystemState {
    Clean = 1,
    Error = 2,
}

#[repr(u16)]
enum ErrorHandlingMethod {
    Ignore = 1,
    RemountReadOnly = 2,
    KernelPanic = 3,
}

#[repr(u32)]
enum CreatorOSId {
    Linux = 0,
    GnuHurd = 1,
    Masix = 2,
    FreeBSD = 3,
    OtherLite = 4,
}

#[repr(u32)]
enum RevisionLevel {
    GoodOldRevision = 0,
    Dynamicrevision = 1,
}

bitflags! {
    /// 32bit bitmask of compatible features. The file system implementation is free to support them or not without
    /// risk of damaging the meta-data.
    struct CompatibleFeatures: u32 {
        const DIR_PREALLOC = 1 << 0;
        const IMAGIC_INODES = 1 << 1;
        const HAS_JOURNAL = 1 << 2;
        const EXT_ATTR = 1 << 3;
        const RESIZE_INO = 1 << 4;
        const DIR_INDEX = 1 << 5;
    }

    /// 32bit bitmask of incompatible features. The file system implementation should refuse to mount the file
    /// system if any of the indicated feature is unsupported.
    struct IncompatibleFeatures: u32 {
        const COMPRESSION = 1 << 0;
        const FILETYPE = 1 << 1;
        const RECOVER = 1 << 2;
        const JOURNAL_DEV = 1 << 3;
        const META_BG = 1 << 4;
    }

    struct ReadOnlyCompatibleFeatures: u32 {
        const SPARSE_SUPER = 1 << 0;
        const LARGE_FILE = 1 << 1;
        const BTREE_DIR = 1 << 4;
    }

    struct CompressionAlgorithm: u32 {
        const LZV1 = 1 << 0;
        const LZRW3A = 1 << 1;
        const GZIP_ALG = 1 << 2;
        const BZIP2 = 1 << 3;
        const LZO = 1 << 4;
    }
}

pub fn mount_filesystem(mmu: &mut MemoryManagementUnit, drive: &mut AHCIDevice) {
    println!("ext2: mounting file system...");

    let mut superblock = MaybeUninit::<Superblock>::uninit();

    drive.read_from_device(mmu, SUPERBLOCK_OFFSET as u64, size_of::<Superblock>() as u64, superblock.as_mut_ptr() as *mut c_void);
    let superblock = unsafe { superblock.assume_init() };

    assert_eq!(superblock.ext2_signature, EXT2_SIGNATURE);
    println!("ext2: successfully identified EXT2 signature 0x{:X}", superblock.ext2_signature);
    //let block_group_descriptor_table_position = if

    // superblock should now be loaded with the contents in the drive
}