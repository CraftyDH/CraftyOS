use super::{PageDirectoryEntry, PageTable, PtFlag};

// use crate::GlobalAllocator;

pub struct PageTableManager {
    pub pml4: *mut PageTable,
}

impl PageTableManager {
    pub fn new(pml4: *mut PageTable) -> PageTableManager {
        return PageTableManager { pml4: pml4 };
    }

    // pub unsafe fn map_memory(&mut self, GlobalAllocator: &mut super::page_frame_allocator::PageFrameAllocator, logical: u64) {
    //     let mut pml4 = *(self.pml4 as *mut PageTable);

    //     let pml4_idx = ((logical >> 39) & 0x1ff) as usize;
    //     let pdp_idx = ((logical >> 30) & 0x1ff) as usize;
    //     let pd_idx = ((logical >> 21) & 0x1ff) as usize;
    //     let pt_idx = ((logical >> 12) & 0x1ff) as usize;
    //     let p_idx = (logical & 0x7ff) as usize;

    //     if !pml4.entries[pml4_idx].present() {
    //         let mut pdpt_alloc = GlobalAllocator.request_page() as *mut PageTable;
    //         core::ptr::write_bytes(pdpt_alloc, 0, 0x1000);

    //         *pdpt_alloc.set_address(pdpt_alloc as u64 & 0x000ffffffffff000);
    //         *pdpt_alloc.set_present(true);
    //         *pdpt_alloc.set_read_write(true);

    //         pml4.entries[pml4_idx] =
    //     }

    //     return ()
    // }

    pub unsafe fn map_memory(
        &mut self,
        GlobalAllocator: &mut super::page_frame_allocator::PageFrameAllocator,
        virtual_memory: *const u8,
        physical_memory: *const u8,
    ) {
        let indexer = super::page_map_indexer::PageMapIndexer::new(virtual_memory as u64);
        // println!("Index: {:?}", indexer);
        let mut pml4 = *self.pml4;

        let mut pde = pml4.entries[indexer.pdp_i as usize];
        let mut pdp: *mut PageTable = 0 as *mut PageTable;
        if !pde.get_flag(PtFlag::Present) {
            pdp = GlobalAllocator.request_page() as *mut PageTable;
            core::ptr::write_bytes::<u8>(pdp as *mut u8, 0, 0x1000);

            pde.set_address(pdp as u64 >> 12);
            pde.set_flag(PtFlag::Present, true);
            pde.set_flag(PtFlag::ReadWrite, true);

            (*self.pml4).entries[indexer.pdp_i as usize] = pde;
        } else {
            // println!("Old 1");
            pdp = (pde.get_address() << 12) as *mut PageTable;
        }

        if pdp as u64 == 0 {
            panic!("PDP null");
        }

        let mut pde = (*pdp).entries[indexer.pd_i as usize];
        let mut pd: *mut PageTable = 0 as *mut PageTable;
        if !pde.get_flag(PtFlag::Present) {
            let pd = GlobalAllocator.request_page() as *mut PageTable;
            core::ptr::write_bytes(pd as *mut u8, 0, 0x1000);

            pde.set_address(pd as u64 >> 12);
            pde.set_flag(PtFlag::Present, true);
            pde.set_flag(PtFlag::ReadWrite, true);

            println!("New 2");
            // println!("New PD {:?}", pde);
            (*pdp).entries[indexer.pd_i as usize] = pde;
            // core::ptr::write_volatile(&mut pdp as *mut PageTable, pdp);
        } else {
            pd = (pde.get_address() << 12) as *mut PageTable;
        }
        if pdp as u64 == 0 {
            panic!("PDP null");
        }

        let mut pde = (*pd).entries[indexer.pt_i as usize];
        let mut pt: *mut PageTable = 0 as *mut PageTable;
        if !pde.get_flag(PtFlag::Present) {
            let pt = GlobalAllocator.request_page() as *mut PageTable;
            core::ptr::write_bytes(pt as *mut u8, 0, 0x1000);

            pde.set_address(pt as u64 >> 12);
            pde.set_flag(PtFlag::Present, true);
            pde.set_flag(PtFlag::ReadWrite, true);

            println!("New 3");
            // println!("New PT {:?}", pde);
            (*pd).entries[indexer.pt_i as usize] = pde;
            // core::ptr::write(&mut pd as *mut PageTable, pd);
        } else {
            // println!("Old 3");
            pt = (pde.get_address() << 12) as *mut PageTable;
        }

        if pdp as u64 == 0 {
            panic!("PDP null");
        }

        let mut pde = (*pt).entries[indexer.p_i as usize];
        pde.set_address(physical_memory as u64 >> 12);
        pde.set_flag(PtFlag::Present, true);
        pde.set_flag(PtFlag::ReadWrite, true);
        (*pt).entries[indexer.p_i as usize] = pde;
    }
}
