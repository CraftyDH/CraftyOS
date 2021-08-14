use alloc::vec::Vec;
use uefi::proto::console::gop::{GraphicsOutput, Mode};

pub fn initialize_gop(bt: &uefi::table::boot::BootServices) -> &mut GraphicsOutput {
    let gop = match bt.locate_protocol::<GraphicsOutput>() {
        Ok(status) => unsafe { &mut *status.unwrap().get() },
        Err(e) => {
            error!("Cannot locate GOP: {:?}", e);
            loop {}
        }
    };

    // The max resolution to choose
    let maxx = 1600;
    let maxy = 1400;

    let mut modes: Vec<Mode> = Vec::new();

    for mode in gop.modes() {
        let mode = mode.unwrap();
        let info = mode.info();
        let (x, y) = info.resolution();
        if x <= maxx && y <= maxy {
            modes.push(mode)
        }
    }

    if modes.len() >= 1 {
        let mode = modes.last().unwrap();
        info!("{:?}", mode.info());

        let gop2 = match bt.locate_protocol::<GraphicsOutput>() {
            Ok(status) => unsafe { &mut *status.unwrap().get() },
            Err(e) => {
                error!("Cannot locate GOP: {:?}", e);
                loop {}
            }
        };

        gop2.set_mode(&mode).unwrap().unwrap();
    }

    info!("GOP found");
    gop
}
