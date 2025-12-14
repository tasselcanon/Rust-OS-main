use crate::print;
use crate::println;
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::stream::Stream;
use futures_util::stream::StreamExt;
use futures_util::task::AtomicWaker;
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};

// 我们使用了 OnceCell 类型，它能安全地实现静态值的一次性初始化。
// 除了 OnceCell 原语，我们也可以在此处使用 lazy_static 宏
// 但是 OnceCell 类型的优势在于，我们可以确保初始化操作不会发生在中断处理程序中，
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// 被中断处理程序调用
///
/// 不能阻塞或者分配
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
}

pub struct ScancodeStream {
    _private: (), // 防止从模块外部构造该结构体。这使得 new 函数成为构造该类型的唯一方式
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            // 初始化扫描码队列，大小为 100
            // 这个初始化操作只能被调用一次
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeSteam::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<u8>> {
        // 从 SCANCODE_QUEUE 中获取队列的引用
        let queue = SCANCODE_QUEUE.try_get().expect("not init");
        // 如果队列不为空，直接返回队列中的扫描码
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }
        WAKER.register(&cx.waker()); // 注册当前任务的 waker，以便在有新的扫描码时唤醒它
        match queue.pop() {
            Some(scancode) => {
                WAKER.take(); // 取走 waker，确保不会被重复唤醒
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    // 初始化键盘，这是一个全局的 键盘状态机

    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),   // 定义扫描码集为 ScancodeSet1
        layouts::Us104Key,     // 把键盘布局定为 Us104Key，因为我们的键盘是 Us104 布局
        HandleControl::Ignore, // 不要处理控制键和组合键
    );

    // 把扫描码转换成按键事件
    // Ok(Some(event)) -> 拿到按键事件（例如 A 按下）
    // Ok(None) -> 暂时没有完整事件（某些键要很多扫描码组合）
    // Err(_) -> 无法识别的扫描码
    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            // 把按键事件转换成可打印字符或特殊字符
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character), // 普通字符，如 A、1、! 等
                    DecodedKey::RawKey(key) => print!("{:?}", key), // 原始键码，如 LeftShift、F1 等，debug 打印
                }
            }
        }
    }
}
