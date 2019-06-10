use core::ops::{Deref, DerefMut};
use x86_64::registers::control;
use x86_64::instructions::tlb;
use x86_64::ux::u9;
use x86_64::structures::paging::{Page, PageTable, RecursivePageTable, PhysFrame, PageTableFlags};
use super::temporary_page::TemporaryPage;

pub struct ActivePageTable<'a> {
    mapper: RecursivePageTable<'a>,
}

impl<'a> Deref for ActivePageTable<'a> {
    type Target = RecursivePageTable<'a>;

    fn deref(&self) -> &RecursivePageTable<'a> {
        &self.mapper
    }
}

impl<'a> DerefMut for ActivePageTable<'a> {
    fn deref_mut(&mut self) -> &mut RecursivePageTable<'a> {
        &mut self.mapper
    }
}

fn page_table<'a>() -> &'a mut PageTable {
    let recursive_index = u9::new(511);
    let recursive_page_table_addr = Page::from_page_table_indices(
        recursive_index,
        recursive_index,
        recursive_index,
        recursive_index,
    )
        .start_address();
    let page_table = unsafe { &mut *(recursive_page_table_addr.as_mut_ptr()) };
    page_table
}

impl<'a> ActivePageTable<'a> {
    pub unsafe fn new() -> ActivePageTable<'a> {
        let page_table = page_table();
        ActivePageTable {
            mapper: RecursivePageTable::new(page_table).expect("recursive page table creation failed"),
        }
    }

    pub fn with<F>(&mut self, table: &mut InactivePageTable, temporary_page: &mut TemporaryPage, f: F)
        where F: FnOnce(&mut RecursivePageTable<'a>)
    {
        {
            let backup = control::Cr3::read().0;

            // map temporary_page to current p4 table
            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            // overwrite recursive mapping
            page_table()[511].set_frame(table.p4_frame.clone(), PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            tlb::flush_all();

            // execute f in the new context
            f(self);

            p4_table[511].set_frame(backup, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            tlb::flush_all();
        }

        temporary_page.unmap(self);
    }


    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        let cr3 = control::Cr3::read();
        let old_table = InactivePageTable {
            p4_frame: cr3.0,
        };
        unsafe {
            control::Cr3::write(new_table.p4_frame, cr3.1);
        }
        old_table
    }
}

pub struct InactivePageTable {
    pub p4_frame: PhysFrame,
}

impl InactivePageTable {
    pub fn new(frame: PhysFrame,
               active_table: &mut ActivePageTable,
               temporary_page: &mut TemporaryPage)
               -> InactivePageTable {
        {
            let table = temporary_page.map_table_frame(frame.clone(), active_table);
            // now we are able to zero the table
            table.zero();
            // set up recursive mapping for the table
            table[511].set_frame(frame.clone(), PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        }
        temporary_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}