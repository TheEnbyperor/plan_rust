use multiboot2::{MemoryAreaIter, MemoryArea};
use x86_64::structures::paging::{PhysFrame, FrameAllocator};
use x86_64::structures::paging::page::Size4KiB;
use x86_64::PhysAddr;

pub struct AreaFrameAllocator<'a> {
    next_free_frame: PhysFrame,
    current_area: Option<&'a MemoryArea>,
    areas: MemoryAreaIter<'a>,
    kernel_start: PhysFrame,
    kernel_end: PhysFrame,
    multiboot_start: PhysFrame,
    multiboot_end: PhysFrame,
}

impl AreaFrameAllocator<'_> {
    pub fn new(kernel_start: PhysAddr, kernel_end: PhysAddr,
               multiboot_start: PhysAddr, multiboot_end: PhysAddr,
               memory_areas: MemoryAreaIter) -> AreaFrameAllocator
    {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: PhysFrame::containing_address(PhysAddr::new(0)),
            current_area: None,
            areas: memory_areas,
            kernel_start: PhysFrame::containing_address(kernel_start),
            kernel_end: PhysFrame::containing_address(kernel_end),
            multiboot_start: PhysFrame::containing_address(multiboot_start),
            multiboot_end: PhysFrame::containing_address(multiboot_end),
        };
        allocator.choose_next_area();
        allocator
    }

    fn choose_next_area(&mut self) {
        self.current_area = self.areas.clone().filter(|area| {
            let address = area.end_address() - 1;
            PhysFrame::containing_address(PhysAddr::new(address)) >= self.next_free_frame
        }).min_by_key(|area| area.start_address());

        match self.current_area {
            Some(area) => {
                let start_frame =
                    PhysFrame::containing_address(PhysAddr::new(area.start_address()));
                if self.next_free_frame < start_frame {
                    self.next_free_frame = start_frame;
                }
            }
            None => {}
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for AreaFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        match self.current_area {
            Some(area) => {
                let frame = self.next_free_frame.clone();

                // the last frame of the current area
                let current_area_last_frame = {
                    let address = area.end_address() - 1;
                    PhysFrame::containing_address(PhysAddr::new(address))
                };

                if frame > current_area_last_frame {
                    // all frames of current area are used, switch to next area
                    self.choose_next_area();
                } else if frame >= self.kernel_start && frame <= self.kernel_end {
                    // `frame` is used by the kernel
                    self.next_free_frame = self.kernel_end.clone() + 1;
                } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                    // `frame` is used by the multiboot information structure
                    self.next_free_frame = self.multiboot_end.clone() + 1;
                } else {
                    // frame is unused, increment `next_free_frame` and return it
                    self.next_free_frame += 1;
                    return Some(frame);
                }
                // `frame` was not valid, try it again with the updated `next_free_frame`
                self.allocate_frame()
            }
            None => {
                None // no free frames left
            }
        }
    }
}