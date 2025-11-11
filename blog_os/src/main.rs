#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use crate::vga_buffer::WRITER;
mod serial;
mod vga_buffer;

// ==========================
//  主运行区（Kernel Entry）
// ==========================

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

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

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("hello{}world", "!4545");

    #[cfg(test)]
    test_main();

    loop {}
}

// ==========================
//  测试区（Unit Tests）
// ==========================
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

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

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("\nRunning {} tests", tests.len());
    for test in tests {
        test.run();
    }
    serial_println!();
    exit_qemu(QemuExitCode::Success);
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
