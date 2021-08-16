// pub struct Bitmap<'a> {
//     pub size: usize,
//     pub buffer: &'a mut [u8],
// }

// const BIT_INDEXER: u32 = 0b10_000_000;

// impl Bitmap<'_> {
//     pub fn set(&mut self, index: u64, value: bool) {
//         let byte_index = index / 8;
//         let bit_index = BIT_INDEXER >> (index % 8);
//         self.buffer[byte_index as usize] &= !bit_index as u8;
//         if value {
//             self.buffer[byte_index as usize] |= bit_index as u8;
//         }
//     }
// }

// impl core::ops::Index<usize> for Bitmap<'_> {
//     type Output = bool;

//     fn index(&self, index: usize) -> &bool {
//         let byte_index = index / 8;
//         let bit_index = BIT_INDEXER >> (index % 8);

//         if (self.buffer[byte_index as usize] & bit_index as u8) > 0 {
//             return &true;
//         }
//         return &false;
//     }
// }

pub struct Bitmap {
    pub size: usize,
    pub buffer: *mut u8,
}

const BIT_INDEXER: u32 = 0b10_000_000;

impl Bitmap {
    pub fn new(size: usize, buf_ptr: *mut u8) -> Bitmap {
        for i in 0..size {
            unsafe {
                core::ptr::write(buf_ptr.offset(i as isize), 0);
            }
        }
        Bitmap {
            size,
            buffer: buf_ptr,
        }
    }
    pub fn set(&mut self, index: u64, value: bool) -> bool {
        if index as usize > self.size * 8 {
            return false;
        }
        let byte_index = (index / 8) as isize;
        let bit_index = BIT_INDEXER >> (index % 8);
        let mut byte = unsafe { core::ptr::read(self.buffer.offset(byte_index)) };
        byte &= !bit_index as u8;
        if value {
            byte |= bit_index as u8;
        }
        unsafe { core::ptr::write(self.buffer.offset(byte_index), byte) };

        return true;
    }
}

impl core::ops::Index<usize> for Bitmap {
    type Output = bool;

    fn index(&self, index: usize) -> &bool {
        if index > self.size * 8 {
            return &false;
        }

        let byte_index = (index / 8) as isize;
        let bit_index = BIT_INDEXER >> (index % 8);
        let mut byte = unsafe { core::ptr::read(self.buffer.offset(byte_index)) };

        if (byte & bit_index as u8) > 0 {
            return &true;
        }
        return &false;
    }
}
