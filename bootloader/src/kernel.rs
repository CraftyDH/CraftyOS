use alloc::vec::Vec;
use uefi::prelude::{BootServices, Status as UEFIStatus};
use uefi::proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType};
use uefi::table::boot::{AllocateType, MemoryType};

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

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

// For the ELF Header https://refspecs.linuxfoundation.org/elf/gabi4+/ch4.eheader.html
const ELFCLASS64: u8 = 2; // 64 BIT
const ELFDATA2LSB: u8 = 1; // LSB not MSB

const ET_EXEC: u16 = 2; // Executable file
const EM_X86_64: u16 = 62; // AMD x86-64 architecture

// For the ELF Program Header https://refspecs.linuxbase.org/elf/gabi4+/ch5.pheader.html
const PT_LOAD: u32 = 1; // A loadable segment

pub fn load_kernel(boot_services: &BootServices, mut kernel_data: Vec<u8>) -> u64 {
    let elf_header = unsafe { *(kernel_data.as_mut_ptr() as *const Elf64Ehdr) };
    if &elf_header.e_ident[0..6]
        == [
            0x7F,
            'E' as u8,
            'L' as u8,
            'F' as u8,
            ELFCLASS64,
            ELFDATA2LSB,
        ]
        && elf_header.e_type == ET_EXEC
        && elf_header.e_machine == EM_X86_64
        && elf_header.e_version == 1
    {
        info!("Kernel Header Verified");
    } else {
        panic!("Kernel Header Invalid")
    }

    for program_header_ptr in (elf_header.e_phoff
        ..((elf_header.e_phnum * elf_header.e_phentsize) as u64))
        .step_by(elf_header.e_phentsize as usize)
    {
        let program_header = unsafe {
            *(kernel_data.as_mut_ptr().offset(program_header_ptr as isize) as *const Elf64Phdr)
        };
        if program_header.p_type == PT_LOAD {
            // We need to load to section
            // Round size needed to the next page
            let pages = (program_header.p_memsz + 0x1000 - 1) / 0x1000;
            // Round start address to start of a page
            let addr = (program_header.p_paddr / 0x1000) * 0x1000;

            // Allocate page
            let _ = match boot_services.allocate_pages(
                AllocateType::Address(addr as usize),
                MemoryType::LOADER_DATA,
                pages as usize,
            ) {
                Err(err) => {
                    panic!("Couldn't allocate page {:?}", err);
                }
                Ok(_) => (),
            };

            info!("{:?}", program_header);

            unsafe {
                core::ptr::copy::<u8>(
                    kernel_data
                        .as_mut_ptr()
                        .offset(program_header.p_offset as isize),
                    program_header.p_paddr as *mut u8,
                    program_header.p_filesz as usize,
                )
            }
        }
    }

    elf_header.e_entry
}
