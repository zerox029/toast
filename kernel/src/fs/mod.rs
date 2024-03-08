pub mod ext2;
pub mod vfs;

const MAX_FILENAME_LENGTH: usize = 256;
const MAX_PATH_LENGTH: usize = 4096;

pub trait FileSystem {
    fn create_directory();
    fn delete_directory();
    fn open_directory();
    fn close_directory();
    fn read_directory();
    fn rename();
}