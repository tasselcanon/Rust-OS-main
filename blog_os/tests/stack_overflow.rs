#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use blog_os::{QemuExitCode, exit_qemu, serial_print, serial_println};
use core::panic::PanicInfo;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

// 为了测试栈溢出，我们需要一个专门的 IDT，因为栈溢出会引发双重故障异常
lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(blog_os::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");
    blog_os::gdt::init();
    init_test_idt();

    stack_overflow();

    panic!("Execution continued after stack overflow");
}

// 双重故障处理函数，用于测试栈溢出引发的双重故障异常
extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _err_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[allow(unconditional_recursion)] // 允许无限递归
fn stack_overflow() {
    stack_overflow(); // 递归调用导致栈溢出
    volatile::Volatile::new(0).read(); // 防止尾递归优化,确保每次递归都会真的压栈
}
