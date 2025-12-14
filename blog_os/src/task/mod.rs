use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub mod executor;
pub mod keyboard;
pub mod simple_executor;

pub struct Task {
    // 任务的 Future 类型，用于驱动任务的执行
    future: Pin<Box<dyn Future<Output = ()>>>, // 任务的 Future 类型，用于驱动任务的执行
    id: TaskId,                                // 任务 ID，用于唯一标识每个任务
}
impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            // 将 Future 类型转换为 Pin<Box<T>> 类型，
            // 以便在 poll 方法中使用 Pin::as_mut 方法
            future: Box::pin(future),
            id: TaskId::new(),
        }
    }
    // 由于 Future trait 的 poll 方法期望被 Pin<&mut T> 类型调用，
    // 我们使用 Pin::as_mut 方法先转换 Pin<Box<T>> 类型的 self.future 字段
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64); // 任务 ID 类型，用于唯一标识每个任务
impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0); // 任务 ID 从 0 开始递增
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed)) // 每个任务分配一个唯一的 ID，Relaxed 顺序是为了性能考虑
    }
}
