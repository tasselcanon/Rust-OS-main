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
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

// 这个测试不能用我们通用的了，这个测试我们要定义我们自己的 test_runner 函数
pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test(); // 做所有测试时，假如有测试 panic 了就执行 panic 函数了，所以返回 Success 了
        serial_println!("[test didn't panic]");
        exit_qemu(QemuExitCode::Failed); // 万一都通过了就 Failed 
    }
    exit_qemu(QemuExitCode::Success);
}
