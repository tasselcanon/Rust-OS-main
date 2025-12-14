use super::{Locked, align_up};
use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};
struct ListNode {
    size: usize,
    // &'static mut 类型的语义上描述了一个由指持有的所有权对象
    // 本质上，它是一个缺少在作用域结束时释放对象的析构函数的 Box智能指针
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize // ListNode 这个头部在内存里的位置
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkListAllocator {
    head: ListNode,
}
impl LinkListAllocator {
    /// 创建一个空的 链表分配器
    pub const fn new() -> Self {
        LinkListAllocator {
            head: ListNode::new(0),
        }
    }

    /// 用给定的堆边界初始化分配器
    ///
    /// 这个函数是不安全的，因为调用者必须保证给定的堆边界是有效的并且堆是未使用的。
    /// 此方法只能调用一次
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_region(heap_start, heap_size);
        }
    }

    /// 将给定的内存区域添加到链表前端
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // 确保给定的内存区域足以存储 ListNode
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // 创建一个新的 ListNode 并将其添加到链表前端
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        // 将 addr 转换为 ListNode 指针
        let node_ptr = addr as *mut ListNode;
        unsafe {
            node_ptr.write(node); // 写入新的 ListNode
            self.head.next = Some(&mut *node_ptr);
        }
    }
    /// 查找给定大小和对齐方式的空闲区域并将其从链表中移除。
    ///
    /// 返回一个包含链表节点和分配内存区域起始地址的元组。
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // 区域适用于分配 -> 从链表中移除该节点
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // 区域不适用 -> 继续下一个区域
                current = current.next.as_mut().unwrap();
            }
        }
        // 未找到合适的区域
        None
    }
    /// 尝试将给定区域用于给定大小和对齐要求的分配。
    ///
    /// 成功时返回分配该内存区域的起始地址。
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // 区域太小，不够要分配的大小
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // 区域剩余部分太小，不足以存储 ListNode结构体
            // 必须满足此条件，因为分配将区域分为已用和空闲部分
            return Err(());
        }
        Ok(alloc_start)
    }

    /// 调整给定的内存布局，使最终分配的内存区域
    /// 足以存储一个 `ListNode` 。
    ///
    /// 将调整后的大小和对齐方式作为（size, align）元组返回
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkListAllocator::size_align(layout);
        let mut allocator = self.lock();
        // 找到第一个合适的区域
        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            // 检查是否有足够的空间分配
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            // 区域剩余部分是否足够存储 ListNode 结构体
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                unsafe {
                    // 区域剩余部分足够存储 ListNode 结构体
                    // 将剩余部分添加到空闲链表中
                    allocator.add_free_region(alloc_end, excess_size);
                }
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkListAllocator::size_align(layout);
        // 将已释放的内存区域添加到空闲链表中
        unsafe { self.lock().add_free_region(ptr as usize, size) }
    }
}
