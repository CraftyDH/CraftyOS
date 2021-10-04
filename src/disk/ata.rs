use core::{
    convert::TryInto,
    mem::{size_of, size_of_val},
    sync::atomic::AtomicBool,
};

use alloc::{string::String, vec::Vec};
use x86_64::instructions::port::Port;

use crate::task::yield_now;

#[repr(C, align(2))]
#[derive(Debug)]
pub struct ATADiskIdentify {
    pub config: u16,
    pub cylinders: u16,
    pub specconf: u16,
    pub heads: u16,

    pub _obsolete2: [u16; 2],

    pub sectors: u16,
    pub vendor: [u16; 3],
    pub serial: [u8; 20],

    pub _retired20: [u16; 2],
    pub _obsolete22: u16,

    pub firmware_revision: [u8; 8],
    pub model: [u8; 40],
    pub sectors_per_interrupt: u16,
    pub tcg: u16, /* Trusted Computing Group */

    pub capabilities1: u16,
    pub capabilities2: u16,

    pub _retired_piomode: u16,
    pub _retired_dmamode: u16,

    pub ata_valid: u16,

    pub current_cylinders: u16,
    pub current_heads: u16,
    pub current_sectors: u16,
    pub current_size_1: u16,
    pub current_size_2: u16,
    pub multi: u16,

    pub lba_size_1: u16,
    pub lba_size_2: u16,
    pub _obsolete62: u16,

    pub multiword_dma_modes: u16,
    pub apio_modes: u16,

    pub mwdmamin: u16,
    pub mwdmarec: u16,
    pub pioblind: u16,
    pub pioiordy: u16,
    pub support3: u16,

    pub _reserved70: u16,
    pub rlsovlap: u16,
    pub rlsservice: u16,
    pub _reserved73: u16,
    pub _reserved74: u16,
    pub queue: u16,

    pub sata_capabilities: u16,
    pub sata_capabilities2: u16,
    pub sata_support: u16,
    pub sata_enabled: u16,
    pub version_major: u16,
    pub version_minor: u16,

    pub command_1: u16,
    pub command2: u16,
    pub extension: u16,

    pub ultra_dma_modes: u16,
    pub erase_time: u16,
    pub enhanced_erase_time: u16,
    pub apm_value: u16,
    pub master_passwd_revision: u16,
    pub hwres: u16,

    pub acoustic: u16,

    pub stream_min_req_size: u16,
    pub stream_transfer_time: u16,
    pub stream_access_latency: u16,
    pub stream_granularity: u32,
    pub lba_size48_1: u16,
    pub lba_size48_2: u16,
    pub lba_size48_3: u16,
    pub lba_size48_4: u16,
    pub _reserved104: u16,

    pub max_dsm_blocks: u16,
    pub pss: u16,

    pub isd: u16,
    pub wwm: [u16; 4],
    pub _reserved112: [u16; 5],
    pub lss_1: u16,
    pub lss_2: u16,
    pub support2: u16,

    pub enabled2: u16,
    pub _reserved121: [u16; 6],
    pub removable_status: u16,
    pub security_status: u16,

    pub _reserved129: [u16; 31],
    pub cfa_powermode1: u16,
    pub _reserved161: u16,
    pub cfa_kms_support: u16,
    pub cfa_trueide_modes: u16,
    pub cfa_memory_modes: u16,
    pub _reserved165: [u16; 3],
    pub form_factor: u16,

    pub support_dsm: u16,

    pub product_id: [u8; 8],
    pub _reserved174: [u16; 2],
    pub media_serial: [u8; 60],
    pub sct: u16,
    pub _reserved207: [u16; 2],
    pub lsalign: u16,

    pub wrv_sectors_m3_1: u16,
    pub wrv_sectors_m3_2: u16,
    pub wrv_sectors_m2_1: u16,
    pub wrv_sectors_m2_2: u16,

    pub nv_cache_caps: u16,
    pub nv_cache_size_1: u16,
    pub nv_cache_size_2: u16,
    pub media_rotation_rate: u16,

    pub _reserved218: u16,
    pub nv_cache_opt: u16,
    pub wrv_mode: u16,
    pub _reserved221: u16,

    pub transport_major: u16,
    pub transport_minor: u16,
    pub _reserved224: [u16; 31],
    pub integrity: u16,
}

pub struct ATA {
    data: Port<u16>,
    error: Port<u8>,
    sector_count: Port<u8>,
    lba_low: Port<u8>,
    lba_mid: Port<u8>,
    lib_hi: Port<u8>,
    device: Port<u8>,
    command: Port<u8>,
    control: Port<u8>,
    master: bool,
    bytes_per_sector: u16,
}

impl ATA {
    pub fn new(port_base: u16, master: bool) -> Self {
        Self {
            data: Port::new(port_base),
            error: Port::new(port_base + 1),
            sector_count: Port::new(port_base + 2),
            lba_low: Port::new(port_base + 3),
            lba_mid: Port::new(port_base + 4),
            lib_hi: Port::new(port_base + 5),
            device: Port::new(port_base + 6),
            command: Port::new(port_base + 7),
            control: Port::new(port_base + 0x206),

            master: master,
            bytes_per_sector: 512,
        }
    }

    pub async fn identify<'buf>(
        &mut self,
        buffer: &'buf mut Vec<u8>,
    ) -> Option<&'buf ATADiskIdentify> {
        unsafe {
            // Who are we talking to?
            if self.master {
                self.device.write(0xA0);
            } else {
                self.device.write(0xB0);
            }
            // Should do this. "WYOOS"
            self.control.write(0);

            // Read status
            self.device.write(0xA0);
            let status = self.command.read();
            if status == 0xFF {
                // There is no device here
                return None;
            }
            // Who are we talking to?
            if self.master {
                self.device.write(0xA0);
            } else {
                self.device.write(0xB0);
            }
            self.sector_count.write(0);
            self.lba_low.write(0);
            self.lba_mid.write(0);
            self.lib_hi.write(0);

            // Command for identify
            self.command.write(0xEC);

            let mut status = self.command.read();
            if status == 0x00 {
                // There is no device here
                return None;
            }

            while (status & 0x80) == 0x80 // Device is busy
             && (status & 0x01) != 0x01
            // There was an error
            {
                println!("Yielding 1");
                // yield_now().await;
                status = self.command.read();
            }

            if status & 0x01 == 1 {
                println!(
                    "ERROR reading identify from ATA, PORT: {:?}, master: {}",
                    self.command, self.master
                );
                return None;
            }

            // It is now ready, read into a buffer
            for _ in (0..512).step_by(2) {
                let data = self.data.read();
                buffer.push(((data >> 8) & 0x00FF) as u8);
                buffer.push((data & 0x00FF) as u8);
            }

            let (_, info, _) = unsafe { buffer.align_to::<ATADiskIdentify>() };
            Some(&info[0])
        }
    }

    pub fn read_28(&mut self, sector: u32, count: usize) {
        if sector & 0xF000_0000 == 1 || count > self.bytes_per_sector.into() {
            return;
        }

        // Who are we talking to?
        let device_num;
        if self.master {
            device_num = 0xE0;
        } else {
            device_num = 0xF0;
        }
        unsafe {
            self.device.write(
                (device_num | (sector & 0x0F00_0000) >> 24)
                    .try_into()
                    .unwrap(),
            );
            // Should do this. "WYOOS"
            self.error.write(0);
            self.control.write(0);
            self.sector_count.write(1);
            self.lba_low
                .write((sector & 0x0000_00FF).try_into().unwrap());
            self.lba_mid
                .write(((sector & 0x0000_FF00) >> 8).try_into().unwrap());
            self.lib_hi
                .write(((sector & 0x00FF_0000) >> 16).try_into().unwrap());

            // Command for read
            self.command.write(0x20);

            let mut status = self.command.read();
            while (status & 0x80) == 0x80 // Device is busy
                 && (status & 0x01) != 0x01
            // There was an error
            {
                status = self.command.read();
            }

            if status & 0x01 == 1 {
                println!("ERROR reading from ATA");
                return;
            }

            println!("Reading from ATA: ");

            // It is now ready
            for i in (0..count).step_by(2) {
                let wdata = self.data.read();

                print!(
                    "{}{}",
                    (wdata & 0x00FF) as u8 as char,
                    ((wdata >> 8) & 0x00FF) as u8 as char,
                );
                // data[i] = (wdata & 0x00FF).try_into().unwrap();
                // data[i + 1] = ((wdata >> 8) & 0x00FF).try_into().unwrap();
            }

            for _ in ((count + count % 2).try_into().unwrap()..self.bytes_per_sector).step_by(2) {
                // We must read entire sector
                self.data.read();
            }
        }
    }
    pub fn write_28(&mut self, sector: u32, data: &[u8], count: usize) {
        if sector & 0xF000_0000 == 1 || count > self.bytes_per_sector.into() {
            return;
        }
        // Who are we talking to?
        let device_num;
        if self.master {
            device_num = 0xE0;
        } else {
            device_num = 0xF0;
        }
        unsafe {
            self.device.write(
                (device_num | (sector & 0x0F00_0000) >> 24)
                    .try_into()
                    .unwrap(),
            );
            // Should do this. "WYOOS"
            self.control.write(0);

            self.sector_count.write(1);

            self.lba_low
                .write((sector & 0x0000_00FF).try_into().unwrap());
            self.lba_mid
                .write(((sector & 0x0000_FF00) >> 8).try_into().unwrap());
            self.lib_hi
                .write(((sector & 0x00FF_0000) >> 16).try_into().unwrap());

            // Command for write
            self.command.write(0x30);

            println!("Writing to ATA: ");

            // It is now ready
            for i in (0..count).step_by(2) {
                let mut wdata: u16 = data[i as usize].into();
                if i + 1 < count.try_into().unwrap() {
                    wdata |= (data[i as usize + 1] as u16) << 8;
                }

                print!(
                    "{}{}",
                    (wdata & 0x00FF) as u8 as char,
                    ((wdata >> 8) & 0x00FF) as u8 as char
                );
                self.data.write(wdata);
            }

            for _ in ((count + count % 2).try_into().unwrap()..self.bytes_per_sector).step_by(2) {
                self.data.write(0x0000);
            }
        }
    }
    pub fn flush(&mut self) {
        // Who are we talking to?
        let device_num;
        if self.master {
            device_num = 0xE0;
        } else {
            device_num = 0xF0;
        }
        unsafe {
            self.device.write(device_num);
            // Command for flush
            self.command.write(0xE7);

            println!("Flushing ATA");
            let mut status = self.command.read();
            if status == 0x00 {
                return;
            }
            while (status & 0x80) == 0x80 // Device is busy
                 && (status & 0x01) != 0x01
            // There was an error
            {
                status = self.command.read();
            }

            if status & 0x01 == 1 {
                println!("ERROR");
                return;
            }
        }
    }
}
