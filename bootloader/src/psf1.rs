use alloc::vec::Vec;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileMode, FileType};

const PSF1_MAGIC: [u8; 2] = [0x36, 0x04];

#[derive(Debug, Clone, Copy)]
pub struct PSF1FontHeader {
    pub magic: [u8; 2],
    pub mode: u8,
    pub charsize: u8,
}

pub struct PSF1Font {
    pub psf1_header: PSF1FontHeader,
    pub glyph_buffer: Vec<u8>,
}

pub fn load_psf1_font(root: &mut Directory, path: &str) -> PSF1Font {
    // Find the kernel and open it
    let psf1 = match File::open(root, path, FileMode::Read, FileAttribute::READ_ONLY) {
        Ok(psf1) => psf1.unwrap(),
        Err(e) => {
            info!("Cant find {:?}", e);
            loop {}
        }
    };

    // Kernal must be a file
    let mut psf1 = match psf1.into_type().unwrap().expect("Failed to get psf1 font") {
        FileType::Regular(file) => file,
        FileType::Dir(_) => {
            info!("psf1 is a dir ???");
            loop {}
        }
    };
    let mut psf1_font = vec![0; core::mem::size_of::<PSF1FontHeader>()];
    let _bytes_read = psf1.read(&mut psf1_font).unwrap().unwrap();

    let psf1_font_header = unsafe { psf1_font.align_to::<PSF1FontHeader>().1[0] };

    if psf1_font_header.magic != PSF1_MAGIC {
        error!("PSF1 FONT not valid");
        loop {}
    }

    let mut glyph_buffer_size = (psf1_font_header.charsize as usize) * 256;
    if psf1_font_header.mode == 1 {
        // 512 glyph mode
        glyph_buffer_size *= 2;
    }

    let mut psf1_font = vec![0; glyph_buffer_size];
    let _bytes_read = psf1.read(&mut psf1_font).unwrap().unwrap();

    return PSF1Font {
        psf1_header: psf1_font_header,
        glyph_buffer: psf1_font,
    };
}
