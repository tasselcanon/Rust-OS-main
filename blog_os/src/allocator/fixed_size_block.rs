use super::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr, ptr::NonNull};
struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// 要使用的块大小
///
/// 各块大小必须为2的幂，因为它们同时被
/// 用作块内存对齐（对齐方式必须始终为2的幂）
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// 创建一个空的 固定大小块分配器
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()], // 每个块大小对应一个链表头
            fallback_allocator: linked_list_allocator::Heap::empty(), // 后备分配器，用于分配无法满足的请求
        }
    }
    /// 用给定的堆边界初始化分配器
    ///
    /// 此函数是不安全的，因为调用者必须保证给定的堆边界是有效的且堆是
    /// 未使用的。此方法只能调用一次。
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.fallback_allocator.init(heap_start, heap_size);
        }
    }
    /// 使用后备分配器分配
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        // allocate_first_fit的作用是在后备分配器中查找第一个合适的空闲块
        // 并将其分配给请求的布局。如果没有合适的空闲块，返回null指针。
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// 为给定布局选择适当的块大小
///
/// 返回 `BLOCK_SIZES` 数组中的索引
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        // 查找适当的块大小索引
        match list_index(&layout) {
            // 如果找到合适的块大小索引，从对应链表中弹出一个节点
            Some(index) => match allocator.list_heads[index].take() {
                // 如果链表不为空，弹出一个节点并返回其指针
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    node as *mut ListNode as *mut u8
                }
                // 如果链表为空，尝试从后备分配器分配一个块
                None => {
                    let block_size = BLOCK_SIZES[index];
                    let block_align = block_size; // 块对齐方式与块大小相同
                    let layout = Layout::from_size_align(block_size, block_align).unwrap();
                    allocator.fallback_alloc(layout) // 如果后备分配器也无法满足请求，返回null指针
                }
            },
            None => allocator.fallback_alloc(layout), // 如果没有合适的块大小索引，尝试从后备分配器分配
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                // 验证块是否满足存储节点所需的大小和对齐方式要求
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node); // 将新节点写入块内存
                    allocator.list_heads[index] = Some(&mut *new_node_ptr); // 将新节点设置为链表头
                }
            }
            None => {
                // 将指针转换为NonNull类型，unwrap()确保指针不为null
                let ptr = NonNull::new(ptr).unwrap();
                unsafe {
                    // deallocate 从后备分配器中释放指针
                    allocator.fallback_allocator.deallocate(ptr, layout);
                }
            }
        }
    }
}
