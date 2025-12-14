#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use blog_os::println;
use blog_os::task::Task;
use blog_os::task::executor::Executor;
use blog_os::task::keyboard;
use core::panic::PanicInfo;
extern crate alloc;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    blog_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

use bootloader::{BootInfo, entry_point};
entry_point!(kernel_main);

async fn async_num() -> u32 {
    42
}

async fn example_task() {
    let num = async_num().await;
    println!("num: {}", num);
}

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use blog_os::allocator;
    use blog_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    println!("Hello World{}\n", "!");
    blog_os::init();
    // 测试框架会在测试完成后调用 `test_main`
    #[cfg(test)]
    test_main();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset); // 物理内存偏移量
    // 初始化页表映射器，用于后续的内存映射操作
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // 初始化帧分配器，用于后续的物理内存分配操作
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap init failed");

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();

    // 程序执行到这里说明没有崩溃，打印一条消息
    println!("\nIt did not crash!");
    blog_os::hlt_loop();
}
