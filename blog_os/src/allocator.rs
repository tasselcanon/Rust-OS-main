// use bump::BumpAllocator;
// use linked_list::LinkListAllocator;
use fixed_size_block::FixedSizeBlockAllocator;
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB, mapper::MapToError,
    },
};
pub mod bump;
pub mod fixed_size_block;
pub mod linked_list;

pub const HEAP_START: usize = 0x_4444_4444_0000; // 可随意任取，只要它尚未用于其他内存区域
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        // 堆的开始地址
        let heap_start = VirtAddr::new(HEAP_START as u64);
        // 堆的结束地址， -1 是为了确保堆的结束地址是一个页的边界
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        // 定义堆的开始页和结束页
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        // 定义堆的页范围
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame() // 从帧分配器中分配一个物理帧
            .ok_or(MapToError::FrameAllocationFailed)?; // 如果分配失败，返回错误
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() }; // 映射到页表中并刷新 TLB
        unsafe {
            // 初始化堆，设置堆的开始地址和大小，.lock() 是为了获取锁，确保线程安全
            ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
        }
    }
    Ok(())
}

/// 在 spin 外添加一个包装器，用于确保分配器是线程安全的
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}
impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<'_, A> {
        self.inner.lock()
    }
}
/// 向上对齐给定地址 `addr` 到对齐值 `align`
fn align_up(addr: usize, align: usize) -> usize {
    // 分步骤
    /* let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }*/
    // 一步到位
    (addr + align - 1) & !(align - 1)
}

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());
