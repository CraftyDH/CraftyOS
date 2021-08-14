use alloc::vec::Vec;
use uefi::prelude::{BootServices, Status as UEFIStatus};
use uefi::proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType};
use uefi::table::boot::{AllocateType, MemoryType};
use xmas_elf::{header, program, ElfFile};

pub fn get_kernel(root: &mut Directory, path: &str) -> Vec<u8> {
    // Find the kernel and open it
    let kernel = match File::open(root, path, FileMode::Read, FileAttribute::READ_ONLY) {
        Ok(kernel) => {
            info!("Found the kernel");
            kernel.unwrap()
        }
        Err(e) => {
            info!("{:?}", e);
            loop {}
        }
    };

    // Kernal must be a file
    let mut kernel = match kernel.into_type().unwrap().expect("Failed to get kernel") {
        FileType::Regular(file) => file,
        FileType::Dir(_) => {
            info!("Kernel is a dir ???");
            loop {}
        }
    };

    // 150 Bytes for the header should be suffient
    let mut kernel_info_buffer = vec![0; 150];
    let kernel_info = match File::get_info::<FileInfo>(&mut kernel, &mut kernel_info_buffer) {
        Ok(file) => file.unwrap(),
        Err(e) if e.status() == UEFIStatus::BUFFER_TOO_SMALL => {
            // Header needs a bigger buffer :(
            let size = e.data().unwrap();
            info!("Reading kernel with size {:?}", size);
            // Increase buffer to size requested
            kernel_info_buffer.resize(size, 0);
            // This time size should be right panic otherwise.
            File::get_info::<FileInfo>(&mut kernel, &mut kernel_info_buffer)
                .expect("Incorrect size given")
                .unwrap()
        }
        Err(e) => {
            info!("{:?} : {:?}", e.status(), e.data());
            loop {}
        }
    };

    // Read the kernel
    let mut kernel_data = vec![0u8; kernel_info.file_size() as usize];
    let bytes_read = kernel.read(&mut kernel_data).unwrap().unwrap();

    // Check that we read all of the kernel
    if bytes_read as u64 != kernel_info.file_size() {
        info!(
            "Only read {} bytes out of {}",
            bytes_read,
            kernel_info.file_size()
        )
    }

    return kernel_data;
}

pub fn load_kernel(boot_services: &BootServices, mut kernel_data: Vec<u8>) -> u64 {
    // Use xmas elf becuase I don't want to implement ELF
    let kernel_elf = ElfFile::new(&mut kernel_data).unwrap();
    // Check that it is a valid ELF file
    header::sanity_check(&kernel_elf).unwrap();

    for header in kernel_elf.program_iter() {
        // Only deal with 1 type of ELF program
        if header.get_type().unwrap() == program::Type::Load {
            // Round size required to pages.
            let pages = (header.mem_size() + 0x1000 - 1) / 0x1000;
            let mut segment = header.physical_addr();

            // This errors with NOT_FOUND after the first run ???, however it still works
            // TODO: Make it not error
            let _ = match boot_services.allocate_pages(
                AllocateType::Address(segment as usize),
                MemoryType::LOADER_DATA,
                pages as usize,
            ) {
                Err(err) => {
                    info!("{:?}", err);
                }
                Ok(_) => (),
            };

            // Get all the data from the file
            let data = match header.get_data(&kernel_elf).unwrap() {
                program::SegmentData::Undefined(data) => data,
                // IDK, dont know how to handle other cases
                other => {
                    info!("ELF header returned: {:?}", other);
                    loop {}
                }
            };

            // Write each section byte by byte
            for chr in data {
                unsafe {
                    core::ptr::write(segment as *mut u8, *chr);
                }
                segment += 1;
            }
        }
    }

    kernel_elf.header.pt2.entry_point()
}
