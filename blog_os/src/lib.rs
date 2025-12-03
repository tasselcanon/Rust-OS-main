#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)] // x86-interrupt 并不是稳定特性，需要手动启用
pub mod gdt;
pub mod interrupts;
pub mod serial;
pub mod vga_buffer;

use core::panic::PanicInfo;

// ==================
//      EXIT QEMU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10, // 纯个人风格，可切换成任何数字
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
//      EXIT QEMU END
// ==================

pub fn init() {
    gdt::init(); // gdt: 定义 CPU 如何执行程序 (段、权限、TSS)
    interrupts::init_idt(); // idt: 定义 CPU 遇到事件后该跳去哪 (中断与异常处理函数)
    unsafe {
        interrupts::PICS.lock().initialize(); // 初始化主副 PIC
    }
    x86_64::instructions::interrupts::enable(); // 启用中断
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt(); // 使 CPU 进入 HLT 休眠状态，等待中断
    }
}

// ==================
//      TASTABLE
pub trait Testable {
    fn run(&self) -> ();
}
impl<T> Testable for T
where
    T: Fn(), // T 必须要满足可像函数一样被调用
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>()); // 打印函数名
        self(); // 调用测试函数本身
        serial_println!("[ok]"); // 打印 [ok] 测试完成
    }
}
//        END
// ==================

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("\nRunning {} tests", tests.len());
    for test in tests {
        test.run();
    }
    serial_println!();
    exit_qemu(QemuExitCode::Success);
}

// 测试函数失败通用
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

// ==================
//      测试区

#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
