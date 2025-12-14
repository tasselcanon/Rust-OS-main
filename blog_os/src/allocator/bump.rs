use super::{Locked, align_up};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}
impl BumpAllocator {
    /// 创建一个新的空的 bump 分配器
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,  // 堆的起始位置
            heap_end: 0,    // 堆的结束位置
            next: 0,        // 下一个分配的地址
            allocations: 0, // 已分配的内存块数量
        }
    }

    /// 用给定的堆边界初始化 bump 分配器
    /// 这个方法是不安全的，因为调用者必须确保给定的内存范围没有被使用
    /// 同样，这个方法只能被调用一次
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start; // 下一个分配的地址从 堆的起始位置 开始
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();
        // 计算对齐后的分配起始地址
        // layout.align() 要分配的这块内存，最少要满足的对齐要求
        let alloc_start = align_up(bump.next, layout.align());
        // 计算对齐后的分配结束地址
        // alloc_start.checked_add(layout.size()) 防止整数溢出的加法
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump.heap_end {
            ptr::null_mut() // 内存不足
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start; // 重置下一个分配的地址到堆的起始位置
        }
    }
}
