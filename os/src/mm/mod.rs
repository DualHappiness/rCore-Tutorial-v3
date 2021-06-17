mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

// use page_table::{PageTable, PTEFlags};
// use address::{VPNRange, StepByOne};
pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum, StepByOne};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{
    translated_refmut, translated_str, translated_byte_buffer, PageTableEntry,
};

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.lock().activate();
}
