use crate::gdt;
use crate::println;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
// 后续我们会把 idt 放到堆上，但是我们现在是在做操作系统内核，
// 所以我们不能使用标准库的堆分配功能
// 因此我们需要使用一个静态可变变量来存储 IDT
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler); // 注册中断处理函数，处理断点异常
        // set_stack_index 函数是不安全的，要在 unsafe 中运行
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler) // 注册双重故障处理函数
                            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); // 设置双重故障使用的专属栈
        }
        idt
    };
}

// 初始化 IDT，加载中断描述符表
pub fn init_idt() {
    IDT.load();
}

// 中断处理函数
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// 双重故障处理函数，用于处理无法在中断描述符表中找到处理程序的情况或其他严重错误
// double fault 异常会在执行主要异常处理程序时触发二层异常时发生
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _err_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
