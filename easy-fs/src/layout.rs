use alloc::{sync::Arc, vec::Vec};

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, DataBlock, BLOCK_SIZE};

const EFS_MAGIC: u32 = 0x3b800001;

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

const INODE_DIRECT_COUNT: usize = 28;
#[repr(C)]
// 整体大小为128 保证一个块中放4个Inode
pub struct DiskInode {
    pub size: u32,
    // 直接索引 28 blocks 28*512=14K
    pub direct: [u32; INODE_DIRECT_COUNT],
    // 一级索引 512/4 = 128 blocks 128*512=64K
    pub indirect1: u32,
    // 二级索引 128 * 128 blocks 8MB
    pub indirect2: u32,
    type_: DiskInodeType,
}
#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}
impl DiskInode {
    pub fn initialize(&mut self, type_: DiskInodeType) {
        self.size = 0;
        self.direct.iter_mut().for_each(|v| *v = 0);
        self.indirect1 = 0;
        self.indirect2 = 0;
        self.type_ = type_;
    }
}
impl DiskInode {
    pub fn is_dir(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }
    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }
}
const INODE_INDIRECT1_COUNT: usize = BLOCK_SIZE / 4;
const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;
const INDIRECT1_BOUND: usize = DIRECT_BOUND + INODE_DIRECT_COUNT;
const INODE_INDIRET2_COUNT: usize = BLOCK_SIZE / 4 * INODE_INDIRECT1_COUNT;
type IndirectBlock = [u32; BLOCK_SIZE / 4];
impl DiskInode {
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let get = |block_id: usize, index: usize| -> u32 {
            get_block_cache(block_id, block_device.clone())
                .lock()
                .read(0, |indirect_block: &IndirectBlock| indirect_block[index])
        };
        let mut inner_id = inner_id as usize;
        if inner_id < INODE_DIRECT_COUNT {
            return self.direct[inner_id];
        }
        inner_id -= INODE_DIRECT_COUNT;
        if inner_id < INODE_INDIRECT1_COUNT {
            return get(self.indirect1 as usize, inner_id);
        }
        inner_id -= INODE_INDIRECT1_COUNT;
        assert!(inner_id < INODE_INDIRET2_COUNT);
        let indirect1 = get(self.indirect2 as usize, inner_id / INODE_INDIRECT1_COUNT);
        get(indirect1 as usize, inner_id % INODE_INDIRECT1_COUNT)
    }
}
impl DiskInode {
    fn _data_blocks(size: u32) -> u32 {
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }
    pub fn data_blocks(&self) -> u32 {
        Self::_data_blocks(self.size)
    }
    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = data_blocks;
        // indirect1
        if data_blocks > INODE_DIRECT_COUNT {
            total += 1;
        }
        // indirect2
        if data_blocks > INODE_DIRECT_COUNT + INODE_INDIRECT1_COUNT {
            total += 1;
            // (data_blocks - INODE_INDERECT1_BOUND + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT
            total += (data_blocks - INODE_DIRECT_COUNT - 1) / INODE_INDIRECT1_COUNT;
        }
        total as u32
    }
    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }
}
impl DiskInode {
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        todo!()
    }
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        todo!()
    }
}

impl DiskInode {
    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut current = offset;
        let end = (offset + buf.len()).min(self.size as usize);
        let mut cur_inner_id = current / BLOCK_SIZE;
        let mut read_size = 0usize;
        while current < end {
            let mut end_current_block = (current / BLOCK_SIZE + 1) * BLOCK_SIZE;
            end_current_block = end_current_block.min(end);

            let block_read_size = end_current_block - current;
            let dst = &mut buf[read_size..read_size + block_read_size];
            get_block_cache(
                self.get_block_id(cur_inner_id as u32, block_device) as usize,
                block_device.clone(),
            )
            .lock()
            .read(0, |data_block: &DataBlock| {
                let block_offset = current % BLOCK_SIZE;
                let src = &data_block[block_offset..block_offset + block_read_size];
                dst.copy_from_slice(src);
            });
            read_size += block_read_size;
            current = end_current_block;
            cur_inner_id += 1;
        }
        read_size
    }
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        todo!()
    }
}

const NAME_LENGTH_LIMIT: usize = 27;
#[repr(C)]
pub struct DirEntry {
    // ? 使用len保存而不是'\0'结尾应该是更好的选择
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_number: u32,
}
pub const DIRENTRY_SIZE: usize = 32;
impl DirEntry {
    pub fn empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_number: 0,
        }
    }
    pub fn new(name: &str, inode_number: u32) -> Self {
        let mut bytes = [0u8; NAME_LENGTH_LIMIT + 1];
        &mut bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: bytes,
            inode_number,
        }
    }
}
impl DirEntry {
    pub fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const _ as usize as *const u8;
        unsafe { core::slice::from_raw_parts(ptr, DIRENTRY_SIZE) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let ptr = self as *mut _ as usize as *mut u8;
        unsafe { core::slice::from_raw_parts_mut(ptr, DIRENTRY_SIZE) }
    }
}
impl DirEntry {
    pub fn name(&self) -> &str {
        let err_msg = "invalid dir name!";
        let len = (0..=DIRENTRY_SIZE)
            .find(|i| self.name[*i] == b'\0')
            .expect(err_msg);
        core::str::from_utf8(&self.name[..len]).expect(err_msg)
    }
    pub fn inode_number(&self) -> u32 {
        self.inode_number
    }
}
