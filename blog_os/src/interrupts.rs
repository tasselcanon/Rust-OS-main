use crate::println;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

// 后续我们会把 idt 放到堆上，但是我们现在是在做操作系统内核，
// 所以我们不能使用标准库的堆分配功能
// 因此我们需要使用一个静态可变变量来存储 IDT
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
