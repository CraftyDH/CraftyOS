use core::str;

use alloc::vec::Vec;
use spin::Mutex;

use crate::vga_buffer::{
    colour::{Colour, ColourCode},
    writer,
};

use self::ata::ATA;

pub mod ata;

lazy_static! {
    // Interrupt 14
    static ref ATA_0_MASTER: Mutex<ATA> = Mutex::new(ATA::new(0x1F0, true));
    static ref ATA_0_SLAVE: Mutex<ATA> = Mutex::new(ATA::new(0x1F0, false));

    // Interrupt 15
    static ref ATA_1_MASTER: Mutex<ATA> = Mutex::new(ATA::new(0x170, true));
    static ref ATA_1_SLAVE: Mutex<ATA> = Mutex::new(ATA::new(0x170, false));
}

pub fn ata_identify() {
    let mut ata_0_master_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_master_info = ATA_0_MASTER.lock().identify(&mut ata_0_master_info);

    let mut ata_0_slave_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_slave_info = ATA_0_SLAVE.lock().identify(&mut ata_0_slave_info);

    let mut ata_1_master_info: Vec<u8> = Vec::with_capacity(512);
    let ata_1_master_info = ATA_1_MASTER.lock().identify(&mut ata_1_master_info);

    let mut ata_1_slave_info: Vec<u8> = Vec::with_capacity(512);
    let ata_1_slave_info = ATA_1_SLAVE.lock().identify(&mut ata_1_slave_info);

    for (ata_info, name) in [
        (ata_0_master_info, "ATA 0 Master (Drive 0)"),
        (ata_0_slave_info, "ATA 0 Slave (Drive 1)"),
        (ata_1_master_info, "ATA 1 Master (Drive 2)"),
        (ata_1_slave_info, "ATA 1 Slave (Drive 3)"),
    ] {
        if let Some(info) = ata_info {
            println!("Found drive on {}", name);
            println!(
                "    Serial: {}\n    Model:  {}",
                str::from_utf8(&info.serial).unwrap_or("INVALID SERIAL"),
                str::from_utf8(&info.model).unwrap_or("INVALID MODEL"),
            );
        } else {
            println!("No drive found on {}", name)
        }
    }

    // if let Some(_) = ata_0_slave_info {
    //     let rw = true;

    //     // Read
    //     if rw {
    //         ata_0_slave.read_28(10, 256);
    //     } else {
    //         let data = Vec::from("This is going to be written to the disk.");
    //         ata_0_slave.write_28(10, &data, data.len());
    //         ata_0_slave.flush();
    //     }
    // }
}

pub fn read(drive: u8, sector: u32, count: usize) {
    match drive {
        0 => ATA_0_MASTER.lock().read_28(sector, count),
        1 => ATA_0_SLAVE.lock().read_28(sector, count),
        2 => ATA_1_MASTER.lock().read_28(sector, count),
        3 => ATA_1_SLAVE.lock().read_28(sector, count),
        _ => writer::WRITER.lock().write_first_line(
            "Invalid drive number :(",
            ColourCode::from_fg(Colour::LightRed),
        ),
    }
}

pub fn read_screen(drive: u8, section: u32) {
    // We read / write in chunks of 8
    let sector = section * 8;
    for i in 0..7 {
        read(drive, sector + i, 256);
    }
    read(drive, sector + 8, 128)
}

pub fn write(drive: u8, sector: u32, data: &[u8]) {
    match drive {
        0 => {
            ATA_0_MASTER.lock().write_28(sector, data, data.len());
            ATA_0_MASTER.lock().flush();
        }
        1 => {
            ATA_0_SLAVE.lock().write_28(sector, data, data.len());
            ATA_0_SLAVE.lock().flush();
        }
        2 => {
            ATA_1_MASTER.lock().write_28(sector, data, data.len());
            ATA_1_MASTER.lock().flush();
        }
        3 => {
            ATA_1_SLAVE.lock().write_28(sector, data, data.len());
            ATA_1_SLAVE.lock().flush();
        }
        _ => writer::WRITER.lock().write_first_line(
            "Invalid drive number :(",
            ColourCode::from_fg(Colour::LightRed),
        ),
    }
}
pub fn write_screen(drive: u8, section: u32, data: Vec<u8>) {
    // We read / write in chunks of 8
    let sector = section * 8;

    for i in 0..7 {
        let small_data = &data[(i * 256)..((i * 256) + 256)];
        write(drive, sector + i as u32, small_data);
    }
    write(drive, sector + 8, &data[(7 * 256)..]);
}
