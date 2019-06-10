use x86_64::VirtAddr;
use core::slice;
use crate::tar;

#[derive(Debug)]
pub struct InitRD {
    start: VirtAddr,
    end: VirtAddr,
}

impl InitRD {
    pub fn new(start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            start,
            end
        }
    }

    fn size(&self) -> u64 {
        return self.end - self.start
    }

    pub fn dump(&self) {
        let data = unsafe { slice::from_raw_parts(self.start.as_ptr(), self.size() as usize) };
        tar::find_headers(data);
    }
}