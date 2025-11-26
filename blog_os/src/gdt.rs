use lazy_static::lazy_static;
use x86_64::VirtAddr;

// 把一个指针转成 x86_64 crate 里的虚拟地址类型
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment; // TSS 结构体

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0; // 选取第 0 个IST作为 double fault 的专属栈
lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new(); // 创建一个新的 TSS
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5; // 栈内存大小为 5 页大小的原始内存数组
            // 一定要 static mut 而不是 static
            // 要把这个 STACK 放到可写的内存段中
            // 否则 bootloader 会将其分配到只读页中
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK); // 把数组地址转换成虚拟地址
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

// 全局描述符表 GDT
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new(); // 注册一个全局描述符表 GDT
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment()); // 添加内核代码段描述符
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS)); // 添加 TSS 段描述符
        (gdt, Selectors {code_selector,tss_selector}) // 返回 GDT 和 选择子
    };
}

// 段选择子结构体
struct Selectors {
    code_selector: SegmentSelector, // 代码段选择子
    tss_selector: SegmentSelector,  // TSS 段选择子
}

pub fn init() {
    use x86_64::instructions::segmentation::{CS, Segment};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load(); // 加载 GDT 表
    unsafe {
        CS::set_reg(GDT.1.code_selector); // 更新代码段寄存器
        load_tss(GDT.1.tss_selector); // 加载 TSS 段选择子
    }
}
