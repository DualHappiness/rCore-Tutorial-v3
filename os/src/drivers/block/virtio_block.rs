use super::BlockDevice;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

use crate::mm::{
    frame_alloc, frame_allocator::frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr,
    PhysPageNum, StepByOne, VirtAddr,
};

const VIRTIO0: usize = 0x10001000;
pub struct VirtIOBlock(Mutex<VirtIOBlk<'static>>);

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
    fn get_dev_id(&self) -> usize {
        let header = unsafe { &*(VIRTIO0 as *const VirtIOHeader) };
        header.device_type() as usize
    }
}
impl VirtIOBlock {
    pub fn new() -> Self {
        Self(Mutex::new(
            // 需要设备寄存器 就用开头的一部分来代替
            VirtIOBlk::new(unsafe { &mut *(VIRTIO0 as *mut VirtIOHeader) }).unwrap(),
        ))
    }
}

// * 下面要求分配的内存要连续，但是由于只发生再内核初始化阶段 所以可以保证连续
#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let mut ppn_base: PhysPageNum = 0.into();
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            ppn_base = frame.ppn;
        }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.lock().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    let mut ppn: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn);
        ppn.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(pa: PhysAddr) -> VirtAddr {
    pa.0.into()
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(va: VirtAddr) -> PhysAddr {
    PageTable::from_token(kernel_token())
        .translate_va(va)
        .unwrap()
}
