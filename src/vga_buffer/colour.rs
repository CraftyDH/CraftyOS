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
    pub fn new(foreground: Colour, background: Colour) -> ColourCode {
        // The first 4 bits are background and the last 4 are foreground
        ColourCode((background as u8) << 4 | (foreground as u8))
    }
}
