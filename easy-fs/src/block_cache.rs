use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{block_dev::BlockDevice, BLOCK_SIZE};

pub struct BlockCache {
    cache: [u8; BLOCK_SIZE],
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
    modified: bool,
}
impl BlockCache {
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0; BLOCK_SIZE];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }
}
impl BlockCache {
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }
    fn get<T>(&self, offset: usize) -> usize
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        self.addr_of_offset(offset)
    }
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let addr = self.get::<T>(offset);
        unsafe { &*(addr as *const T) }
    }
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_sized = core::mem::size_of::<T>();
        assert!(offset + type_sized <= BLOCK_SIZE);
        self.modified = true;
        let addr = self.get::<T>(offset);
        unsafe { &mut *(addr as *mut T) }
    }
}
impl BlockCache {
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }
}
impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}
impl BlockCache {
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }
}

const BLOCK_CACHE_SIZE: usize = 16;

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}
impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}
impl BlockCacheManager {
    fn check_queue_size(&mut self) -> Result<(), ()> {
        if self.queue.len() == BLOCK_CACHE_SIZE {
            match self
                .queue
                .iter()
                .enumerate()
                .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
            {
                None => return Err(()),
                Some((index, _)) => self.queue.remove(index),
            };
        }
        Ok(())
    }
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        match self.queue.iter().find(|pair| pair.0 == block_id) {
            Some((_, block)) => block.clone(),
            None => {
                self.check_queue_size().expect("run out of block cache!");
                let block = Arc::new(Mutex::new(BlockCache::new(block_id, block_device.clone())));
                self.queue.push_back((block_id, block.clone()));
                block
            }
        }
    }
}

lazy_static! {
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}
