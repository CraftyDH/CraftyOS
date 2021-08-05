#![no_std]
#![no_main]
#![feature(asm)]


use core::panic::PanicInfo;
use uefi::proto::console::gop::*;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
// Use extern win64 so params come through correctly. Thanks Microsoft
pub extern "win64" fn _start(gop: &mut GraphicsOutput) -> u8 {
    let gopmode = &gop.current_mode_info();

    let res = gopmode.resolution();

    // It crashes with a full size so remove the bottom pixel line.
    let res = (res.0, res.1 - 1);

    // RGB Loop using modulus
    for color in 0..0xff {
        let mut pix = 0xff_00_00;
        if color % 3 == 0 {
            pix = 0x00_ff_00;
        } else if color % 2 == 0 {
            pix = 0x00_00_ff;
        }
        gop.blt(BltOp::VideoFill {
            color: BltPixel::from(pix),
            dest: (0, 0),
            dims: res
        }).unwrap().unwrap();

        // Stall time
        for _i in 0..48 {
            unsafe {
                asm!("hlt");
            }
        }
    }

    loop {}
}
