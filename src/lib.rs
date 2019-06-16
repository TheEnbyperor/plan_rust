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
#[macro_use]
extern crate bitflags;
extern crate byteorder;

pub mod vga;
pub mod interrupts;
pub mod memory;
pub mod gdt;
pub mod tar;
pub mod initrd;
pub mod nine_p;
pub mod dev;
pub mod namespace;

use core::panic::PanicInfo;
use memory::heap_allocator::Allocator;
use alloc::boxed::Box;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    println!("{}", info);
    hlt_loop();
}

#[alloc_error_handler]
fn alloc_error(info: core::alloc::Layout) -> ! {
    x86_64::instructions::interrupts::disable();
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

pub fn init<'a>(multiboot_information_p: usize) -> initrd::InitRD<'a> {
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
    let init_rd_server = initrd::InitRDServer::new('/', "initrd", init_rd);

    dev::insert_dev_driver(Box::new(init_rd_server));

    let mut root_namespace = namespace::Namespace::new();

    root_namespace.bind("/", "#/");

    let root = root_namespace.open_file("#/").unwrap();

    let root_fid = root.0.fid_pool().get_fid();
    println!("{:?}", root.0.server().lock().walk(root.1, root_fid, &[]));
    println!("{:?}", root.0.server().lock().open(root_fid, &nine_p::FileMode::new(nine_p::FileAccessMode::Read, false, false)));
    println!("{:?}", root.0.server().lock().read(root_fid, 0, 1000));
    println!("{:?}", root.0.server().lock().clunk(root_fid));
    root.0.fid_pool().clunk_fid(root_fid);
    drop(root_fid);
    let test_fid = root.0.fid_pool().get_fid();
    println!("{:?}", root.0.server().lock().walk(root.1, test_fid, &["test"]));
    println!("{:?}", root.0.server().lock().open(test_fid, &nine_p::FileMode::new(nine_p::FileAccessMode::Read, false, false)));
    println!("{:?}", root.0.server().lock().read(test_fid, 0, 1000));
    println!("{:?}", root.0.server().lock().clunk(test_fid));
    root.0.fid_pool().clunk_fid(test_fid);
    drop(test_fid);

    println!("It did not crash");
    hlt_loop();
}