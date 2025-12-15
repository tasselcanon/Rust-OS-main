use crate::gdt;
use crate::{print, println};
use lazy_static::lazy_static;
use pic8259::ChainedPics; // 用于映射主副 PIC 的映射布局
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32; // 主 PIC 的中断向量偏移量
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8; // 副 PIC 的中断向量偏移量
// 初始化主副 PIC, 并将主 PIC 的 IRQ0 连接到副 PIC 的 IRQ2
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
// 我们需要把 Timer IRQ0 重映射到 PIC_1_OFFSET(一般是 32)
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,        // 定时器中断向量偏移量
    Keyboard = PIC_1_OFFSET + 1, // 键盘中断向量偏移量
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        // 转换为 u8 类型，因为硬件层面全是 u8 类型
        self as u8
    }

    fn as_usize(self) -> usize {
        // 转换为 usize 类型，因为 IDT 是一个数组，索引是 usize 类型
        usize::from(self.as_u8()) // 转换为 usize 需要先转换为 u8 类型
    }
}

// 后续我们会把 idt 放到堆上，但是我们现在是在做操作系统内核，
// 所以我们不能使用标准库的堆分配功能
// 因此我们需要使用一个静态可变变量来存储 IDT
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler); // 注册中断处理函数，处理断点异常
        // set_stack_index 函数是不安全的，要在 unsafe 中运行
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler) // 注册双重故障处理函数
                            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); // 设置双重故障使用的专属栈
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler); // 注册定时器中断处理函数
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler); // 注册键盘中断处理函数
        idt.page_fault.set_handler_fn(page_fault_handler); // 注册页故障处理函数，这样就不会触发双重故障 double fault 了

        idt
    };
}

// 初始化 IDT，加载中断描述符表
pub fn init_idt() {
    IDT.load();
}

// 中断处理函数，用于处理断点异常
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

// 定时器中断处理函数，用于处理定时器中断
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8()); // 发送定时器中断结束信号(EOI)
    }
}

// 键盘中断处理函数，用于处理键盘中断
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    // 我们需要从键盘扫描码端口读取扫描码
    // 因为键盘控制器在我们获取扫描码之前是不会发送下一个中断的
    let mut port = Port::new(0x60); // 键盘扫描码端口
    let scancode: u8 = unsafe { port.read() }; // 从键盘扫描码端口读取扫描码
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// 页故障处理函数，用于处理页故障异常
// 比如访问了一个不存在的地址，或者试图修改一个只读的地址
use crate::hlt_loop;
use x86_64::structures::idt::PageFaultErrorCode;
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    err_code: PageFaultErrorCode,
) {
    // CR2寄存器是会在 page fault 发生时，被 CPU 自动写入导致异常的虚拟地址
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read()); // 可以用 read 函数读取并打印该寄存器
    println!("Error Code: {:?}", err_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
