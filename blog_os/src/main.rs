#![no_std]
#![no_main]
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

mod vga_buffer;
use vga_buffer::Writer;

static HELLO: &[u8] = b"Tassel!";
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    Writer::print_something();

    loop {}
}
