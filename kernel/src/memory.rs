pub fn get_memory_size(mmap: &mut [uefi::table::boot::MemoryDescriptor]) -> u64 {
    let mut memory_size: u64 = 0;
    for describe in mmap {
        memory_size += describe.page_count * 4096;
    }
    memory_size
}
