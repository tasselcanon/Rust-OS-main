#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use blog_os::println;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("hello world{}", "!!!");
    blog_os::init();

    unsafe {
        *(0xdeadbeef as *mut u8) = 42;
    }

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    loop {}
}
