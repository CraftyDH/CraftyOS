#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
// Use extern win64 so params come through correctly. Thanks Microsoft
pub extern "win64" fn _start(a: u8, b: u8) -> u8 {
    // Test math
    return a * b;
}
