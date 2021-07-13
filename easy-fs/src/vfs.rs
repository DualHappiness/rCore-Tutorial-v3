use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::Mutex;

use crate::{
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    layout::{DirEntry, DiskInode, DIRENTRY_SIZE},
    EasyFileSystem,
};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}
impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
}
impl Inode {
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, self.block_device.clone())
            .lock()
            .read(self.block_offset, f)
    }
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, self.block_device.clone())
            .lock()
            .modify(self.block_offset, f)
    }
}
impl Inode {
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENTRY_SIZE;
        (0..file_count)
            .map(|i| {
                let mut entry = DirEntry::empty();
                let size =
                    disk_inode.read_at(DIRENTRY_SIZE * i, entry.as_bytes_mut(), &self.block_device);
                assert_eq!(size, DIRENTRY_SIZE);
                entry
            })
            .find(|entry| entry.name() == name)
            .map(|entry| entry.inode_number() as u32)
    }
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock(); // just for lock
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENTRY_SIZE;
            (0..file_count)
                .map(|index| {
                    let mut entry = DirEntry::empty();
                    let size = disk_inode.read_at(
                        index * DIRENTRY_SIZE,
                        entry.as_bytes_mut(),
                        &self.block_device,
                    );
                    assert_eq!(size, DIRENTRY_SIZE);
                    entry.name().to_string()
                })
                .collect()
        })
    }
    fn increase_size(&self, size: u32, disk_inode: &mut DiskInode, fs: &mut EasyFileSystem) {
        if size <= disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(size);
        let v: Vec<_> = (0..blocks_needed).map(|_| fs.alloc_data()).collect();
        disk_inode.increase_size(size, v, &self.block_device);
    }
    pub fn create(&self, name: &str) -> Option<Arc<Self>> {
        let mut fs = self.fs.lock();
        if self
            .read_disk_inode(|root_inode| {
                assert!(root_inode.is_dir());
                self.find_inode_id(name, root_inode)
            })
            .is_some()
        {
            return None;
        }

        let new_inode_id = fs.alloc_inode();
        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(block_id as usize, self.block_device.clone())
            .lock()
            .modify(block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(crate::layout::DiskInodeType::File)
            });
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENTRY_SIZE;
            let new_size = (file_count + 1) * DIRENTRY_SIZE;
            self.increase_size(new_size as u32, root_inode, &mut fs);
            let entry = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENTRY_SIZE,
                entry.as_bytes(),
                &self.block_device,
            );
        });
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc {
                fs.dealloc_data(data_block);
            }
        })
    }
}
impl Inode {
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        })
    }
}
