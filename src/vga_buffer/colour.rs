//* All the vga text buffer colours
#[allow(dead_code)] // Ignore if some colours aren't used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Colour {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColourCode(u8);

impl ColourCode {
    pub fn from_fg(foreground: Colour) -> ColourCode {
        ColourCode::new(foreground, Colour::Black)
    }

    pub fn from_u8(foreground: u8, background: u8) -> ColourCode {
        assert!(foreground <= 15);
        assert!(background <= 15);
        ColourCode(background << 4 | foreground)
    }

    pub fn new(foreground: Colour, background: Colour) -> ColourCode {
        // The first 4 bits are background and the last 4 are foreground
        ColourCode((background as u8) << 4 | (foreground as u8))
    }

    pub fn flip(&mut self) {
        // println!("C: {}", self.0);
        self.0 = (self.0 & 0x0F) << 4 | (self.0 & 0xF0) >> 4;
    }
}
