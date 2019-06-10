use x86_64::align_up;
use core::convert::TryInto;
use alloc::alloc::{Layout, GlobalAlloc, AllocErr};
use core::mem;
use core::mem::size_of;
use core::ptr::NonNull;
use core::ops::Deref;
use spin::Mutex;

pub struct Heap {
    bottom: usize,
    size: usize,
    holes: HoleList,
}

impl Heap {
    pub const fn empty() -> Heap {
        Heap {
            bottom: 0,
            size: 0,
            holes: HoleList::empty(),
        }
    }

    pub unsafe fn init(&mut self, heap_bottom: usize, heap_size: usize) {
        self.bottom = heap_bottom;
        self.size = heap_size;
        self.holes = HoleList::new(heap_bottom, heap_size);
    }

    pub unsafe fn new(heap_bottom: usize, heap_size: usize) -> Heap {
        Heap {
            bottom: heap_bottom,
            size: heap_size,
            holes: HoleList::new(heap_bottom, heap_size),
        }
    }


    pub fn allocate_first_fit(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        let mut size = layout.size();
        if size < HoleList::min_size() {
            size = HoleList::min_size();
        }
        let size = align_up(size.try_into().unwrap(),
                            mem::align_of::<Hole>().try_into().unwrap());
        let layout = Layout::from_size_align(size.try_into().unwrap(),
                                             layout.align()).unwrap();

        self.holes.allocate_first_fit(layout)
    }


    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let mut size = layout.size();
        if size < HoleList::min_size() {
            size = HoleList::min_size();
        }
        let size = align_up(size.try_into().unwrap(),
                            mem::align_of::<Hole>().try_into().unwrap());
        let layout = Layout::from_size_align(size.try_into().unwrap(),
                                             layout.align()).unwrap();

        self.holes.deallocate(ptr, layout);
    }

    pub fn bottom(&self) -> usize {
        self.bottom
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn top(&self) -> usize {
        self.bottom + self.size
    }

    pub unsafe fn extend(&mut self, by: usize) {
        let top = self.top();
        let layout = Layout::from_size_align(by, 1).unwrap();
        self.holes
            .deallocate(NonNull::new_unchecked(top as *mut u8), layout);
        self.size += by;
    }
}

pub struct HoleList {
    first: Hole,
}

impl HoleList {
    pub const fn empty() -> HoleList {
        HoleList {
            first: Hole {
                size: 0,
                next: None,
            },
        }
    }

    pub unsafe fn new(hole_addr: usize, hole_size: usize) -> HoleList {
        assert_eq!(size_of::<Hole>(), Self::min_size());

        let ptr = hole_addr as *mut Hole;
        ptr.write(Hole {
            size: hole_size,
            next: None,
        });

        HoleList {
            first: Hole {
                size: 0,
                next: Some(&mut *ptr),
            },
        }
    }

    pub fn allocate_first_fit(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        assert!(layout.size() >= Self::min_size());

        allocate_first_fit(&mut self.first, layout).map(|allocation| {
            if let Some(padding) = allocation.front_padding {
                deallocate(&mut self.first, padding.addr, padding.size);
            }
            if let Some(padding) = allocation.back_padding {
                deallocate(&mut self.first, padding.addr, padding.size);
            }
            NonNull::new(allocation.info.addr as *mut u8).unwrap()
        })
    }


    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        deallocate(&mut self.first, ptr.as_ptr() as usize, layout.size())
    }

    pub fn min_size() -> usize {
        size_of::<usize>() * 2
    }
}

pub struct Hole {
    size: usize,
    next: Option<&'static mut Hole>,
}

impl Hole {
    fn info(&self) -> HoleInfo {
        HoleInfo {
            addr: self as *const _ as usize,
            size: self.size,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct HoleInfo {
    addr: usize,
    size: usize,
}

struct Allocation {
    info: HoleInfo,
    front_padding: Option<HoleInfo>,
    back_padding: Option<HoleInfo>,
}

fn split_hole(hole: HoleInfo, required_layout: Layout) -> Option<Allocation> {
    let required_size = required_layout.size();
    let required_align = required_layout.align();

    let (aligned_addr, front_padding) = if hole.addr ==
        align_up(hole.addr.try_into().unwrap(), required_align.try_into().unwrap())
            .try_into().unwrap() {
        // hole has already the required alignment
        (hole.addr, None)
    } else {
        // the required alignment causes some padding before the allocation
        let aligned_addr: usize =
            align_up((hole.addr + HoleList::min_size()).try_into().unwrap(),
                     required_align.try_into().unwrap()).try_into().unwrap();
        (
            aligned_addr,
            Some(HoleInfo {
                addr: hole.addr,
                size: aligned_addr - hole.addr,
            }),
        )
    };

    let aligned_hole = {
        if aligned_addr + required_size > hole.addr + hole.size {
            // hole is too small
            return None;
        }
        HoleInfo {
            addr: aligned_addr,
            size: hole.size - (aligned_addr - hole.addr),
        }
    };

    let back_padding = if aligned_hole.size == required_size {
        // the aligned hole has exactly the size that's needed, no padding accrues
        None
    } else if aligned_hole.size - required_size < HoleList::min_size() {
        // we can't use this hole since its remains would form a new, too small hole
        return None;
    } else {
        // the hole is bigger than necessary, so there is some padding behind the allocation
        Some(HoleInfo {
            addr: aligned_hole.addr + required_size,
            size: aligned_hole.size - required_size,
        })
    };

    Some(Allocation {
        info: HoleInfo {
            addr: aligned_hole.addr,
            size: required_size,
        },
        front_padding,
        back_padding,
    })
}

fn allocate_first_fit(mut previous: &mut Hole, layout: Layout) -> Result<Allocation, AllocErr> {
    loop {
        let allocation: Option<Allocation> = previous
            .next
            .as_mut()
            .and_then(|current| split_hole(current.info(), layout.clone()));
        match allocation {
            Some(allocation) => {
                // hole is big enough, so remove it from the list by updating the previous pointer
                previous.next = previous.next.as_mut().unwrap().next.take();
                return Ok(allocation);
            }
            None if previous.next.is_some() => {
                // try next hole
                previous = move_helper(previous).next.as_mut().unwrap();
            }
            None => {
                // this was the last hole, so no hole is big enough -> allocation not possible
                return Err(AllocErr);
            }
        }
    }
}

fn deallocate(mut hole: &mut Hole, addr: usize, mut size: usize) {
    loop {
        assert!(size >= HoleList::min_size());

        let hole_addr = if hole.size == 0 {
            // It's the dummy hole, which is the head of the HoleList. It's somewhere on the stack,
            // so it's address is not the address of the hole. We set the addr to 0 as it's always
            // the first hole.
            0
        } else {
            // tt's a real hole in memory and its address is the address of the hole
            hole as *mut _ as usize
        };

        // Each freed block must be handled by the previous hole in memory. Thus the freed
        // address must be always behind the current hole.
        assert!(
            hole_addr + hole.size <= addr,
            "invalid deallocation (probably a double free)"
        );

        // get information about the next block
        let next_hole_info = hole.next.as_ref().map(|next| next.info());

        match next_hole_info {
            Some(next) if hole_addr + hole.size == addr && addr + size == next.addr => {
                // block fills the gap between this hole and the next hole
                // before:  ___XXX____YYYYY____    where X is this hole and Y the next hole
                // after:   ___XXXFFFFYYYYY____    where F is the freed block

                hole.size += size + next.size; // merge the F and Y blocks to this X block
                hole.next = hole.next.as_mut().unwrap().next.take(); // remove the Y block
            }
            _ if hole_addr + hole.size == addr => {
                // block is right behind this hole but there is used memory after it
                // before:  ___XXX______YYYYY____    where X is this hole and Y the next hole
                // after:   ___XXXFFFF__YYYYY____    where F is the freed block

                // or: block is right behind this hole and this is the last hole
                // before:  ___XXX_______________    where X is this hole and Y the next hole
                // after:   ___XXXFFFF___________    where F is the freed block

                hole.size += size; // merge the F block to this X block
            }
            Some(next) if addr + size == next.addr => {
                // block is right before the next hole but there is used memory before it
                // before:  ___XXX______YYYYY____    where X is this hole and Y the next hole
                // after:   ___XXX__FFFFYYYYY____    where F is the freed block

                hole.next = hole.next.as_mut().unwrap().next.take(); // remove the Y block
                size += next.size; // free the merged F/Y block in next iteration
                continue;
            }
            Some(next) if next.addr <= addr => {
                // block is behind the next hole, so we delegate it to the next hole
                // before:  ___XXX__YYYYY________    where X is this hole and Y the next hole
                // after:   ___XXX__YYYYY__FFFF__    where F is the freed block

                hole = move_helper(hole).next.as_mut().unwrap(); // start next iteration at next hole
                continue;
            }
            _ => {
                // block is between this and the next hole
                // before:  ___XXX________YYYYY_    where X is this hole and Y the next hole
                // after:   ___XXX__FFFF__YYYYY_    where F is the freed block

                // or: this is the last hole
                // before:  ___XXX_________    where X is this hole
                // after:   ___XXX__FFFF___    where F is the freed block

                let new_hole = Hole {
                    size: size,
                    next: hole.next.take(), // the reference to the Y block (if it exists)
                };
                // write the new hole to the freed memory
                let ptr = addr as *mut Hole;
                unsafe { ptr.write(new_hole) };
                // add the F block as the next block of the X block
                hole.next = Some(unsafe { &mut *ptr });
            }
        }
        break;
    }
}

fn move_helper<T>(x: T) -> T {
    x
}

pub struct Allocator(Mutex<Heap>);

impl Allocator {
    pub const fn empty() -> Allocator {
        Allocator(Mutex::new(Heap::empty()))
    }
}

impl Deref for Allocator {
    type Target = Mutex<Heap>;

    fn deref(&self) -> &Mutex<Heap> {
        &self.0
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0
            .lock()
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }
}