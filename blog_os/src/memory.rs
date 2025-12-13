use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

/// 初始化一个新的OffsetPageTable。
///
/// 这个函数是不安全的
/// 因为调用者必须保证完整的物理内存能在传递的 `physical_memory_offset` 被映射到虚拟内存
/// 必须保证只被调用一次，以避免 &mut 引用的别名问题
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        // 创建一个新的 OffsetPageTable 实例
        // 用于将虚拟地址转换为物理地址
        // 并返回一个新的 OffsetPageTable 实例
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// 返回一个对活动的4级页表的可变引用
///
/// 这个函数是不安全的
/// 因为调用者必须保证完整的物理内存能在传递的 `physical_memory_offset` 被映射到虚拟内存
/// 必须保证只被调用一次，以避免 &mut 引用的别名问题
/// 私有
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    // 从 CR3 寄存器中读取活动的 4 级页表帧
    let (level_4_table_frame, _) = Cr3::read();
    // 从页表帧中获取物理地址
    let phys = level_4_table_frame.start_address();
    // 计算虚拟地址，也就是物理地址加上偏移量
    let virt = physical_memory_offset + phys.as_u64();
    // 将虚拟地址转换为页表指针
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    // 解引用指针并返回页表引用
    unsafe { &mut *page_table_ptr }
}

/// 为给定的页面创建一个实例映射到框架`0xb8000`
pub fn create_example_mapping(
    page: Page,                                          // 要映射的虚拟页面
    mapper: &mut OffsetPageTable,                        // 能够安全地修改页表
    frame_allocator: &mut impl FrameAllocator<Size4KiB>, // 帧分配器，用于分配物理帧
) {
    use x86_64::structures::paging::PageTableFlags as Flags;
    // 要映射的物理框架
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    // 映射标志，这里设置为存在和可写
    let flags = Flags::PRESENT | Flags::WRITABLE;
    // 执行映射操作，将虚拟页面映射到物理框架，让 page -> frame
    let map_to_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    // 检查映射是否成功，失败则 panic
    map_to_result.expect("map_to failed").flush();
}

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
/// 一个FrameAllocator，从bootloader的内存地图中返回可用的 frames
/// 该分配器会返回所有在内存地图中被标记为 "可用 "的帧
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap, // 内存地图引用
    next: usize,                    // 下一个可用帧的索引
}

impl BootInfoFrameAllocator {
    /// 从传递的内存 map 中创建一个FrameAllocator。
    ///
    /// 这个函数是不安全的，因为调用者必须保证传递的内存 map 是有效的。
    /// 主要的要求是，所有在其中被标记为 "可用 "的帧都是真正未使用的
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// 返回所有可用的物理帧
    ///
    /// 这个函数是不安全的，因为调用者必须保证传递的内存 map 是有效的
    /// 返回一个迭代器，返回所有在内存地图中被标记为 "可用 "的帧
    /// 这个函数通俗来说就是，从内存地图中提取所有可用的物理帧，遍历所有可用的内存区域，
    /// 并返回所有在这些区域中的4096字节对齐的物理帧
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter(); // 遍历内存地图中的所有区域
        // 过滤出所有可用的内存区域
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // 从可用的内存区域中提取地址范围
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        // 从地址范围中提取所有4096字节对齐的地址
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // 从起始地址创建 `PhysFrame` 类型
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

// 实现 `FrameAllocator<Size4KiB>` trait 用于 BootInfoFrameAllocator
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next); // 获取第 `next` 个可用帧
        self.next += 1; // 增加 `next` 索引，准备返回下一个可用帧
        frame
    }
}

//////////////////////////////////////////////////////////////////// 可删 👇
/// 将给定的虚拟地址转换为映射的物理地址，如果地址没有被映射，则为`None'。
///
/// 这个函数是不安全的，因为调用者必须保证完整的物理内存在传递的`physical_memory_offset`处被映射到虚拟内存。
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

/// 由 `translate_addr`调用的私有函数。
///
/// 这个函数是安全的，可以限制`unsafe`的范围，
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::page_table::FrameError;
    // 从 CR3 寄存器中读取活动的 4 级页表帧
    let (level_4_table_frame, _) = Cr3::read();
    // 把虚拟地址分成 4 个索引，分别对应 4 级页表
    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    // 初始`frame`为4级页表帧，意为从4级页表开始遍历
    let mut frame = level_4_table_frame;
    // 遍历 4 个索引
    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        // 把虚拟地址转换为页表指针
        let table_ptr: *const PageTable = virt.as_ptr();
        // 解引用指针并获取页表引用
        let table = unsafe { &*table_ptr };
        // 获取页表引用中的页表项
        let entry = &table[index];
        // 读取页表条目并更新`frame`
        // 如果页表项没有映射到物理帧，则返回 None
        // 因为我们不支持大页，所以如果页表项映射到大页，就直接恐慌
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }
    // 最后返回物理帧的起始地址加上页内偏移量
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
//////////////////////////////////////////////////////////////////// 可删 👆
