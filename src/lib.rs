#![no_std]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(const_fn)]

extern crate volatile;
extern crate lazy_static;
extern crate spin;
extern crate x86_64;
extern crate multiboot2;
extern crate pic8259_simple;
extern crate pc_keyboard;
extern crate alloc;
#[macro_use]
extern crate once;

pub mod vga;
pub mod interrupts;
pub mod memory;
pub mod gdt;
pub mod tar;
pub mod initrd;

use core::panic::PanicInfo;
use memory::heap_allocator::Allocator;
use initrd::InitRD;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

#[alloc_error_handler]
fn alloc_error(info: core::alloc::Layout) -> ! {
    println!("Allocation failed: {:?}", info);
    hlt_loop();
}

pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB
pub const INITRD_START: usize = 0o_000_002_000_000_0000;

#[global_allocator]
static HEAP_ALLOCATOR: Allocator = Allocator::empty();

fn enable_nxe_bit() {
    use x86_64::registers::model_specific::{Efer, EferFlags};

    unsafe {
        let mut efer = Efer::read();
        efer.set(EferFlags::NO_EXECUTE_ENABLE, true);
        Efer::write(efer);
    }
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::{Cr0, Cr0Flags};

    unsafe {
        let mut cr0 = Cr0::read();
        cr0.set(Cr0Flags::WRITE_PROTECT, true);
        Cr0::write(cr0);
    };
}

pub fn init(multiboot_information_p: usize) -> initrd::InitRD {
    vga::WRITER.lock().clear_screen();
    println!("Starting planRust");
    let boot_info = unsafe { multiboot2::load(multiboot_information_p) };
    let init_rd = memory::init(boot_info);
    unsafe {
        HEAP_ALLOCATOR.lock().init(HEAP_START, HEAP_START + HEAP_SIZE);
    }
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    init_rd
}

#[no_mangle]
pub extern "C" fn rust_start(multiboot_information_p: usize) -> ! {
    unsafe {
        asm!("mov ax, 0
        mov ss, ax
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax" :::: "intel");
    }

    enable_nxe_bit();
    enable_write_protect_bit();

    let init_rd = init(multiboot_information_p);
    init_rd.dump();


    println!("It did not crash");
    hlt_loop();
}