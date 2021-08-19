use super::{Frame, PhysicalAddress, VirtualAddress, PAGE_SIZE};

pub mod table;

const ENTRY_COUNT: usize = 512;

pub struct Page {
    number: usize,
}

bitflags! {
    pub struct EntryFlags: u64 {
        const PRESENT =         1 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const NO_EXECUTE =      1 << 63;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Entry(u64);

impl Entry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(EntryFlags::PRESENT) {
            // Bits 0-12 are for Entry Flags, the other bits are for the address
            return Some(Frame::containing_address(
                self.0 as usize & 0x000fffff_fffff000,
            ));
        }
        None
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        // Start of frame must be page aligned.
        assert_eq!(frame.start_address() & !0x000fffff_fffff000, 0);
        self.0 = (frame.start_address() as u64) | flags.bits;
    }
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        assert!(
            address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
            "invalid address: 0x{:x}",
            address
        );
        Page {
            number: address / PAGE_SIZE,
        }
    }

    fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    // Values from Phil Opp
    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }
    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }
    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }
    fn p1_index(&self) -> usize {
        (self.number >> 0) & 0o777
    }
}

use super::FrameAllocator;
use core::ptr::Unique;
use table::{Level4, Table};

pub struct ActivePageTable {
    p4: Unique<Table<Level4>>,
}

impl ActivePageTable {
    pub unsafe fn new(frame: Frame) -> ActivePageTable {
        let mut p4 = ActivePageTable {
            p4: Unique::new_unchecked(frame.start_address() as *mut Table<Level4>),
        };

        p4.get_p4_mut()[511].set(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);

        // let
        // asm!("mov eax, {0}",
        //     "or eax, 0b11",
        //     "mov [{1}], eax",
        //     in(reg) ptr,
        //     in(reg) ptr + 511 * 8,
        // );
        return p4;
    }
    pub fn get_p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }
    pub fn get_p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }
    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::containing_address(virtual_address))
            .map(|frame| frame.number * PAGE_SIZE + offset)
    }

    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = self.get_p4().next_table(page.p4_index());

        // let p3 = unsafe { &*table::P4 }.next_table(page.p4_index());

        let huge_page = || todo!("No support for huge pages");

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(huge_page)
    }

    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let mut p4 = self.get_p4_mut();
        let p3 = p4.next_table_create(page.p4_index(), allocator);
        let p2 = p3.next_table_create(page.p3_index(), allocator);
        let p1 = p2.next_table_create(page.p2_index(), allocator);

        // Check entry is unused
        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
    }

    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let frame = allocator.allocate_frame().expect("No Frames Available :(");
        self.map_to(page, frame, flags, allocator)
    }

    pub fn identity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    fn unmap<A>(&mut self, page: Page, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        assert!(self.translate(page.start_address()).is_some());

        let p1 = self
            .get_p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("mapping code does not support huge pages");
        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();

        allocator.deallocate_frame(frame);
    }
}

pub fn test_paging<A>(allocator: &mut A)
where
    A: FrameAllocator,
{
    let mut page_table = unsafe { ActivePageTable::new(allocator.allocate_frame().unwrap()) };
    let addr = 42 * 512 * 512 * 4096; // 42th P3 entry
    let page = Page::containing_address(addr);
    let frame = allocator.allocate_frame().expect("no more frames");
    let frame = allocator.allocate_frame().expect("no more frames");
    println!("hello");

    println!(
        "None = {:?}, map to {:?}",
        page_table.translate(addr),
        frame
    );
    page_table.map_to(page, frame, EntryFlags::empty(), allocator);
    println!("Some = {:?}", page_table.translate(addr));
    println!("next free frame: {:?}", allocator.allocate_frame());
}
