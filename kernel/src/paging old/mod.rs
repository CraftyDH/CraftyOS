pub mod page_frame_allocator;
pub mod page_map_indexer;
pub mod page_table_manager;

// #[bitfield]
// #[derive(Debug, Copy, Clone)]
// #[repr(C, align(0x1000))]
// pub struct PageDirectoryEntry {
//     pub present: bool,
//     read_write: bool,
//     user_super: bool,
//     write_through: bool,
//     cache_disabled: bool,
//     accessed: bool,
//     ignore0: B1,
//     larger_pages: bool,
//     ignore1: B1,
//     available: B3,
//     pub address: B52,
// }

enum PtFlag {
    Present = 0,
    ReadWrite = 1,
    UserSuper = 2,
    WriteThrough = 3,
    CacheDisabled = 4,
    Accessed = 5,
    LargerPages = 7,
    Custom0 = 9,
    Custom1 = 10,
    Custom2 = 11,
    NX = 63, // only if supported
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct PageDirectoryEntry {
    value: u64,
}

impl PageDirectoryEntry {
    fn set_flag(&mut self, flag: PtFlag, state: bool) {
        let bit_selector = 1 << (flag as u64);
        self.value &= !bit_selector;
        if state {
            self.value |= bit_selector;
        }
    }

    fn get_flag(&self, flag: PtFlag) -> bool {
        let bit_selector = 1 << (flag as u64);
        if self.value & bit_selector > 0 {
            return true;
        } else {
            return false;
        }
    }

    fn get_address(&self) -> u64 {
        (self.value & 0x000ffffffffff000) >> 12
    }

    fn set_address(&mut self, address: u64) {
        let addr = address & 0x000000ffffffffff;
        self.value &= 0xfff0000000000fff;
        self.value |= addr << 12;
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct PageTable {
    pub entries: [PageDirectoryEntry; 512],
}

impl Default for PageDirectoryEntry {
    fn default() -> PageDirectoryEntry {
        PageDirectoryEntry { value: 0 }
    }
}

impl Default for PageTable {
    fn default() -> PageTable {
        let entry = PageDirectoryEntry::default();
        PageTable {
            entries: [entry; 512],
        }
    }
}
