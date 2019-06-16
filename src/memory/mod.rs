pub mod area_allocator;
pub mod paging;
pub mod temporary_page;
pub mod heap_allocator;

pub use self::area_allocator::AreaFrameAllocator;
pub use self::paging::{ActivePageTable, InactivePageTable};
pub use self::temporary_page::TemporaryPage;

use multiboot2::BootInformation;
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::page::Size4KiB;
use x86_64::structures::paging::{PhysFrame, Page, PageSize, PageTableFlags, FrameAllocator, Mapper};
use multiboot2::ElfSectionFlags;
use crate::println;
use crate::initrd;
use core::convert::TryInto;

pub fn init<'a>(boot_info: BootInformation) -> initrd::InitRD<'a> {
    assert_has_not_been_called!("memory::init must be called only once");

    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required");
    let elf_sections_tag = boot_info.elf_sections_tag()
        .expect("Elf-sections tag required");

    let kernel_start = elf_sections_tag.sections()
        .filter(|s| s.is_allocated())
        .map(|s| s.start_address())
        .min().unwrap();
    let kernel_end = elf_sections_tag.sections()
        .filter(|s| s.is_allocated())
        .map(|s| s.start_address() + s.size())
        .max().unwrap();

    let modules_start = boot_info.module_tags()
        .map(|s| s.start_address())
        .min().unwrap();
    let modules_end = boot_info.module_tags()
        .map(|s| s.end_address())
        .max().unwrap();

    println!("kernel start: {:#x}, kernel end: {:#x}",
        kernel_start, kernel_end);
    println!("modules start: {:#x}, modules end: {:#x}",
        modules_start, modules_end);
    println!("multiboot start: {:#x}, multiboot end: {:#x}",
        boot_info.start_address(), boot_info.end_address());

    let mut modules = boot_info.module_tags();
    let initrd = modules.next().expect("initrd module not in multiboot info");

    let mut frame_allocator = AreaFrameAllocator::new(
        PhysAddr::new(kernel_start), PhysAddr::new(kernel_end),
        PhysAddr::new(modules_start.into()), PhysAddr::new(modules_end.into()),
        PhysAddr::new(boot_info.start_address() as u64), PhysAddr::new(boot_info.end_address() as u64),
        memory_map_tag.memory_areas());

    let mut active_table = remap_the_kernel(&mut frame_allocator, &boot_info);

    use {HEAP_START, HEAP_SIZE, INITRD_START};

    let heap_start_page =
        Page::containing_address(VirtAddr::new(HEAP_START.try_into().unwrap()));
    let heap_end_page =
        Page::containing_address(VirtAddr::new((HEAP_START + HEAP_SIZE-1)
            .try_into().unwrap()));

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        let frame = frame_allocator.allocate_frame().expect("failed to allocate frame for heap");
        unsafe {
            active_table.map_to(page, frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE, &mut frame_allocator)
                .expect("failed to map heap page").flush();
        }
    }

    let initrd_size = (initrd.end_address() - initrd.start_address()) as usize;
    let initrd_start_page =
        Page::containing_address(VirtAddr::new(INITRD_START.try_into().unwrap()));
    let initrd_end_page =
        Page::containing_address(VirtAddr::new((INITRD_START + initrd_size-1)
            .try_into().unwrap()));

    let initrd_start_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(initrd.start_address() as u64));
    let initrd_end_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new((initrd.end_address() - 1) as u64));
    let mut initrd_phys_frames = PhysFrame::range_inclusive(initrd_start_frame, initrd_end_frame);
    for page in Page::range_inclusive(initrd_start_page, initrd_end_page) {
        let frame = initrd_phys_frames.next().expect("failed to get phys frame for initrd virt page");
        unsafe {
            active_table.map_to(page, frame, PageTableFlags::PRESENT, &mut frame_allocator)
                .expect("failed to map initrd page").flush();
        }
    }

    initrd::InitRD::new(
        VirtAddr::new(INITRD_START as u64),
        VirtAddr::new((INITRD_START + initrd_size) as u64)
    )
}

pub fn remap_the_kernel<'a, A>(allocator: &mut A, boot_info: &BootInformation)
    -> ActivePageTable<'a>
    where A: FrameAllocator<Size4KiB>
{
    let mut temporary_page = TemporaryPage::new(
        Page::from_start_address(VirtAddr::new(0xcafebabe * Size4KiB::SIZE))
            .expect("failed to make temporary page"),
        allocator);

    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("no more frames");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info.elf_sections_tag()
            .expect("Memory map tag required");

        // Kernel
        for section in elf_sections_tag.sections() {
            if !section.is_allocated() {
                // section is not loaded to memory
                continue;
            }
            assert_eq!(section.start_address() as u64 % Size4KiB::SIZE, 0, "sections need to be page aligned");

            if section.size() > 0 {
                println!("mapping section at addr: {:#x}, size: {:#x}",
                         section.start_address(), section.size());

                let mut flags = PageTableFlags::PRESENT;

                if section.flags().contains(ElfSectionFlags::WRITABLE) {
                    flags = flags | PageTableFlags::WRITABLE;
                }
                if !section.flags().contains(ElfSectionFlags::EXECUTABLE) {
                    flags = flags | PageTableFlags::NO_EXECUTE;
                }

                let start_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(section.start_address() as u64));
                let end_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new((section.end_address() - 1) as u64));
                for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                    unsafe {
                        mapper.identity_map(frame, flags, allocator)
                            .expect("failed to map kernel frame").flush();
                    }
                }
            }
        }

        // VGA
        let vga_buffer_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(0xb8000));
        unsafe {
            mapper.identity_map(vga_buffer_frame,
                                PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
                                allocator)
                .expect("failed to map VGA frame").flush();
        }

        // Multiboot
        let multiboot_start = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(boot_info.start_address() as u64));
        let multiboot_end = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new((boot_info.end_address() - 1) as u64));
        for frame in PhysFrame::range_inclusive(multiboot_start, multiboot_end) {
            unsafe {
                mapper.identity_map(frame, PageTableFlags::PRESENT, allocator)
                    .expect("failed to map multiboot frame").flush();
            }
        }
    });

    let old_table = active_table.switch(new_table);

    let old_p4_page = Page::<Size4KiB>::containing_address(
      VirtAddr::new(old_table.p4_frame.start_address().as_u64())
    );
    active_table.unmap(old_p4_page).expect("failed to set guard page").1.flush();
    println!("guard page at {:?}", old_p4_page.start_address());

    active_table
}