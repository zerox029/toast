use core::ops::{Index, IndexMut};
use crate::memory::paging_update::entry::*;
use crate::memory::paging_update::ENTRY_COUNT;

pub struct Table {
    entries: [Entry; ENTRY_COUNT]
}

impl Table {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl Index<usize> for Table {
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

impl IndexMut<usize> for Table {
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}