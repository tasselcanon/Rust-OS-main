#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use blog_os::{QemuExitCode, exit_qemu, serial_println};
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[unsafe(no_mangle)]
// 集成测试所有的代码全部放到 start 中执行
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test didn't panic]");
    exit_qemu(QemuExitCode::Failed); // 万一都通过了就 Failed 

    loop {}
}

fn should_fail() {
    serial_println!("should failed... ");
    assert_eq!(0, 2);
}
