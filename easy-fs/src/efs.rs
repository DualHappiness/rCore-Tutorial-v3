use core::u32;

use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    bitmap::Bitmap,
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    layout::{DiskInode, SuperBlock},
    vfs::Inode,
    DataBlock, BLOCK_SIZE,
};

pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inode_area_start_block: u32,
    data_area_start_block: u32,
}
impl EasyFileSystem {
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        let inode_bitmap_blocks = inode_bitmap_blocks as usize;
        let total_blocks = total_blocks as usize;
        let mut offset = 1; // super

        let inode_bitmap = Bitmap::new(offset, inode_bitmap_blocks);
        let inode_num = inode_bitmap.maximum();
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = BLOCK_SIZE / inode_size;
        let inode_area_blocks = (inode_num + inodes_per_block - 1) / inodes_per_block;

        offset += inode_bitmap_blocks + inode_area_blocks;
        let data_blocks = total_blocks - offset; // total - super - inode
        let data_bitmap_blocks = (data_blocks + 4097 - 1) / 4097; // 1 data bitmap index 4096 block
        let data_bitmap = Bitmap::new(offset, data_bitmap_blocks);
        let data_area_blocks = data_blocks - data_bitmap_blocks;

        let mut efs = Self {
            block_device: block_device.clone(),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks as u32,
            data_area_start_block: (offset + data_bitmap_blocks) as u32,
        };

        // clear blocks
        for i in 0..total_blocks {
            get_block_cache(i, block_device.clone())
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    data_block.iter_mut().for_each(|byte| *byte = 0)
                });
        }
        // super block
        get_block_cache(0, block_device.clone()).lock().modify(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks as u32,
                    inode_bitmap_blocks as u32,
                    inode_area_blocks as u32,
                    data_bitmap_blocks as u32,
                    data_area_blocks as u32,
                )
            },
        );
        // flush super node
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, block_device.clone())
            .lock()
            .modify(root_inode_offset as usize, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(crate::layout::DiskInodeType::Directory)
            });

        Arc::new(Mutex::new(efs))
    }
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        get_block_cache(0, block_device.clone())
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        1 + inode_total_blocks as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }
}
impl EasyFileSystem {
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SIZE / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (
            block_id,
            (inode_id % inodes_per_block) as usize * inode_size,
        )
    }
    pub fn get_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, self.block_device.clone())
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| *p = 0)
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }
}
impl EasyFileSystem {
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = efs.lock().block_device.clone();
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        Inode::new(0, block_id, block_offset, efs.clone(), block_device)
    }
}
