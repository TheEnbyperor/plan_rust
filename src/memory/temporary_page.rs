use super::ActivePageTable;
use x86_64::VirtAddr;
use x86_64::structures::paging::page::Size4KiB;
use x86_64::structures::paging::{Mapper, Page, PageTable, PhysFrame, PageTableFlags, FrameAllocator, FrameDeallocator};

pub struct TemporaryPage {
    page: Page,
    allocator: TinyAllocator,
}

impl TemporaryPage {
    pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage
        where A: FrameAllocator<Size4KiB>
    {
        TemporaryPage {
            page,
            allocator: TinyAllocator::new(allocator),
        }
    }

    /// Maps the temporary page to the given frame in the active table.
    /// Returns the start address of the temporary page.
    pub fn map(&mut self, frame: PhysFrame, active_table: &mut ActivePageTable)
        -> VirtAddr
    {
        assert!(active_table.translate_page(self.page).is_err(),
                "temporary page is already mapped");
        unsafe {
            active_table.map_to(self.page, frame, PageTableFlags::WRITABLE | PageTableFlags::PRESENT, &mut self.allocator)
                .expect("failed to map page").flush();
        }
        self.page.start_address()
    }

    /// Unmaps the temporary page in the active table.
    pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page).expect("failed to unmap page").1.flush();
    }

    /// Maps the temporary page to the given page table frame in the active
    /// table. Returns a reference to the now mapped table.
    pub fn map_table_frame(&mut self,
                        frame: PhysFrame,
                        active_table: &mut ActivePageTable)
                        -> &mut PageTable {
        unsafe { &mut *(self.map(frame, active_table).as_mut_ptr() as *mut PageTable) }
    }
}

struct TinyAllocator([Option<PhysFrame>; 3]);

impl TinyAllocator {
    fn new<A>(allocator: &mut A) -> TinyAllocator
        where A: FrameAllocator<Size4KiB>
    {
        let mut f = || allocator.allocate_frame();
        let frames = [f(), f(), f()];
        TinyAllocator(frames)
    }
}

unsafe impl FrameAllocator<Size4KiB> for TinyAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }
        None
    }
}

impl FrameDeallocator<Size4KiB> for TinyAllocator {
    fn deallocate_frame(&mut self, frame: PhysFrame) {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(frame);
                return;
            }
        }
        panic!("Tiny allocator can hold only 3 frames.");
    }
}