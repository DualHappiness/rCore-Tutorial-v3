use alloc::sync::Arc;

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SIZE};

pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }
}

type BitmapBlock = [u64; 64]; // 64 * 64 = 4096
const BLOCK_BITS: usize = BLOCK_SIZE * 8;
impl Bitmap {
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        (0..self.blocks)
            .map(|offset| {
                (
                    offset,
                    get_block_cache(offset + self.start_block_id, block_device.clone()),
                )
            })
            .find_map(|(offset, block)| {
                block.lock().modify(0, |bitmap_block: &mut BitmapBlock| {
                    bitmap_block
                        .into_iter()
                        .enumerate()
                        .find(|(_, bits64)| **bits64 != u64::MAX)
                        .map(|(bit64_pos, bits64)| {
                            let inner_pos = bits64.trailing_ones() as usize;
                            *bits64 |= 1u64 << inner_pos;
                            offset * BLOCK_BITS + bit64_pos * 64 + inner_pos
                        })
                })
            })
    }
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        fn decomposition(bit: usize) -> (usize, usize, usize) {
            let offset = bit / BLOCK_BITS;
            let rest = bit % BLOCK_BITS;
            (offset, rest / 64, rest % 64)
        }
        let (offset, bits64_pos, inner_pos) = decomposition(bit);
        let block = get_block_cache(offset + self.start_block_id, block_device.clone());
        block.lock().modify(0, |bitmap_block: &mut BitmapBlock| {
            assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
            bitmap_block[bits64_pos] &= !(1u64 << inner_pos);
        });
    }
}
impl Bitmap {
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
