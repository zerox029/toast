pub mod ext2;

pub trait FileSystem {
    fn create_directory();
    fn delete_directory();
    fn open_directory();
    fn close_directory();
    fn read_directory();
    fn rename();
}