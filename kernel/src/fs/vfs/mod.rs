use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ops::ControlFlow;
use conquer_once::spin::OnceCell;
use spin::Mutex;

type VfsNodeRef = Arc<Mutex<Box<dyn VfsNode + Send>>>;
type VfsNodeWeakRef = Weak<Mutex<Box<dyn VfsNode + Send>>>;

static ROOT_DIRECTORY: OnceCell<VfsNodeRef> = OnceCell::uninit();

pub trait VfsNode {
    fn name(&self) -> &String;
    fn parent(&self) -> &Option<VfsNodeWeakRef>;
    fn children(&mut self) -> &mut Vec<VfsNodeRef>;

    fn open(&self, );
    fn close(&self, );

    fn read(&self, buffer: *mut c_void, byte_count: usize, offset: usize);
    fn write(&self, buffer: *mut c_void, byte_count: usize, offset: usize);
}

pub struct Vfs {
    mount_points: Vec<RamfsNode>
}

impl Vfs {
    pub fn init() {
        ROOT_DIRECTORY.init_once(|| {
            let root_node = Arc::new(Mutex::new(Box::new(RamfsNode {
                name: String::from("/"),
                parent: None,
                children: Vec::new(),
            }) as Box<dyn VfsNode + Send>));

            let current_directory = Arc::new(Mutex::new(Box::new(RamfsNode {
                name: String::from("."),
                parent: Some(Arc::downgrade(&root_node)),
                children: Vec::new(),
            }) as Box<dyn VfsNode + Send> ));

            let previous_directory = Arc::new(Mutex::new(Box::new(RamfsNode {
                name: String::from(".."),
                parent: Some(Arc::downgrade(&root_node)),
                children: Vec::new(),
            }) as Box<dyn VfsNode + Send> ));

            {
                let mut root_node = root_node.lock();

                root_node.children().push(current_directory);
                root_node.children().push(previous_directory);
            }

            root_node
        });
    }

    pub fn root_directory() -> &'static VfsNodeRef {
        ROOT_DIRECTORY.try_get().expect("fs: virtual file system not initialized")
    }

    pub fn add_child_node(parent: VfsNodeRef, name: &str) {
        let child = Arc::new(Mutex::new(Box::new(RamfsNode {
            name: String::from(name),
            parent: Some(Arc::downgrade(&parent)),
            children: Vec::new(),
        }) as Box<dyn VfsNode + Send> ));

        parent.lock().children().push(child);
    }

    /// Finds a node with the specified name in the children of the given node.
    pub fn find_child(node: VfsNodeRef, name: &str) -> Option<VfsNodeRef> {
        node.lock().children().iter().find(|child| child.lock().name() == name).map(|child| child.clone())
    }

    /// Finds a node at the specified path starting at the given node.
    /// Given the path "Desktop/someFolder" and the node "/home/user", it will return the node at
    /// "/home/user/Desktop/someFolder"
    pub fn find_from_absolute_path(node: VfsNodeRef, path: &str) -> Option<VfsNodeRef> {
        let mut path_iter = path[1..].split('/');

        let current_node = node;
        let node = path_iter.try_fold(current_node, |current_node, current_name| {
            if let Some(found_node) = Self::find_child(current_node, current_name) {
                ControlFlow::Continue(found_node)
            }
            else {
                ControlFlow::Break(())
            }
        });

        match node {
            ControlFlow::Continue(node) => Some(node),
            ControlFlow::Break(()) => None,
        }
    }

    /// Returns the parent of a given node
    pub fn parent(node: VfsNodeRef) -> Option<VfsNodeWeakRef> {
        node.lock().parent().clone()
    }

    /// Returns the absolute path of the given node
    pub fn get_absolute_path(node: VfsNodeRef) -> String {
        if node.lock().name() == "/" {
            return String::from("/");
        }

        let mut current_node = node.clone();
        let mut directory_entries: Vec<String> = Vec::new();
        directory_entries.push({
            let current_node = current_node.lock();

            format!("{}", current_node.name())
        });

        while let Some(parent) = current_node.clone().lock().parent().clone() {
            current_node = parent.upgrade().expect("lol get fucked");

            directory_entries.insert(0, format!("{}", current_node.lock().name()));
        }

        directory_entries.iter().skip(1).map(|entry| format!("/{}", entry) ).collect()
    }
}

pub struct RamfsNode {
    pub name: String,
    pub parent: Option<VfsNodeWeakRef>,
    pub children: Vec<VfsNodeRef>,
}

impl VfsNode for RamfsNode {
    fn name(&self) -> &String {
        &self.name
    }

    fn parent(&self) -> &Option<VfsNodeWeakRef> {
        &self.parent
    }

    fn children(&mut self) -> &mut Vec<VfsNodeRef> {
        &mut self.children
    }

    fn open(&self) {
        panic!("fs: cannot invoke method 'open' a ramfs node");
    }

    fn close(&self) {
        panic!("fs: cannot invoke method 'close' on a ramfs node");
    }

    fn read(&self, buffer: *mut c_void, byte_count: usize, offset: usize) {
        todo!()
    }

    fn write(&self, buffer: *mut c_void, byte_count: usize, offset: usize) {
        todo!()
    }
}