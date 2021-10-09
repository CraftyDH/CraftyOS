use core::convert::TryInto;

use x86_64::instructions::port::Port;

use crate::executor::{spawner::Spawner, yield_now};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum BaseAddressRegisterType {
    MemoryMapping = 0,
    InputOutput = 1,
}

pub struct BaseAddressRegister {
    prefetchable: bool,
    address: u8,
    size: u32,
    register_type: BaseAddressRegisterType,
}

pub struct PCIDevice {
    port_base: u32,
    interrupt: u32,

    bus: u16,
    device: u16,
    function: u16,

    vendor_id: u16,
    device_id: u16,

    class_id: u8,
    subclass_id: u8,
    interface_id: u8,
    revision: u8,
}

// #[repr(C, packed)]
// pub struct PCIDevice {
//     device_id:
// }

impl PCIDevice {}
pub struct PCI {
    data_port: Port<u32>,
    command_port: Port<u32>,
}

impl PCI {
    pub fn new() -> Self {
        Self {
            data_port: Port::new(0xCFC),
            command_port: Port::new(0xCF8),
        }
    }

    fn make_id(bus: u16, device: u16, function: u16, register_offset: u32) -> u32 {
        // Shift the 16 bit number first
        let id = 0x1 << 31
            | u32::from(bus) << 16
            | u32::from(device) << 11
            | u32::from(function) << 8
            | (register_offset & 0xFC);
        id
    }

    pub fn read(&mut self, bus: u16, device: u16, function: u16, register_offset: u32) -> u32 {
        let id = PCI::make_id(bus, device, function, register_offset);
        // Send the device ID to the PCI
        unsafe { self.command_port.write(id) };
        // Read the result send back to us
        let result = unsafe { self.data_port.read() };

        // println!("Result: {}", result);
        // println!("{}", (result >> (8 * (register_offset % 4))));
        // println!("{}", (result >> ((8 * (register_offset & 2)) & 0xFFFF)));

        // The PCI will return in chunks of 4 so discard unwanted bits
        return result >> (8 * (register_offset % 4));
    }

    pub fn write(
        &mut self,
        bus: u16,
        device: u16,
        function: u16,
        register_offset: u32,
        value: u32,
    ) {
        let id = PCI::make_id(bus, device, function, register_offset);
        // Send the device ID to the PCI
        unsafe {
            self.command_port.write(id);
            self.data_port.write(value)
        };
    }

    pub fn get_device_functions(&mut self, bus: u16, device: u16) -> bool {
        // Check if device has functions by checking if function 0 exists
        let function0 = self.read(bus, device, 0, 0x0E) & (1 << 7);
        // println!("Function0: {}", function0);
        if function0 >= 1 {
            return true;
        } else if function0 == 0 {
            return false;
        } else {
            panic!("Result of get_device functions is: {}", function0);
        }
    }

    pub fn get_device_descriptor(&mut self, bus: u16, device: u16, function: u16) -> PCIDevice {
        // Why do C people overflow their numbers :/
        PCIDevice {
            port_base: 0,
            bus: bus,
            device: device,
            function: function,

            vendor_id: (self.read(bus, device, function, 0x00) & 0xFFFF)
                .try_into()
                .unwrap(),
            device_id: (self.read(bus, device, function, 0x02) & 0xFFFF)
                .try_into()
                .unwrap(),

            class_id: (self.read(bus, device, function, 0x0B) & 0xFF)
                .try_into()
                .unwrap(),
            subclass_id: (self.read(bus, device, function, 0x0A) & 0xFF)
                .try_into()
                .unwrap(),
            interface_id: (self.read(bus, device, function, 0x09) & 0xFF)
                .try_into()
                .unwrap(),
            revision: (self.read(bus, device, function, 0x08) & 0xFF)
                .try_into()
                .unwrap(),
            interrupt: self.read(bus, device, function, 0x3C),
        }
    }

    pub fn get_driver(&mut self, dev: &mut PCIDevice, spawner: Spawner) -> u8 {
        match dev.vendor_id {
            // AMD
            0x1022 => {
                match dev.device_id {
                    // AM79C973 - AKA Ethernet
                    0x2000 => {
                        println!("Device: AM79C973");
                    }
                    _ => {
                        println!("Device: Other AMD");
                    }
                }
            }
            // Intel
            0x8086 => {
                println!("Device: Other Intel");
            }
            _ => {
                println!("Device: Unknown");
            }
        };

        match dev.class_id {
            // Graphics card
            0x03 => {
                match dev.subclass_id {
                    // VGA
                    0x00 => {
                        println!("Device: VGA");
                    }
                    _ => {
                        println!("Other Graphics");
                    }
                }
            }
            _ => {
                println!("Device: Unknown");
            }
        };
        0
    }

    pub fn get_base_address_register(
        &mut self,
        bus: u16,
        device: u16,
        function: u16,
        bar_num: u16,
    ) -> Option<BaseAddressRegister> {
        let header_type = self.read(bus, device, function, 0x0E) & 0x7F;
        let max_bars = 6 - (4 * header_type);

        if bar_num >= max_bars.try_into().unwrap() {
            return None;
        }

        let bar_value = self.read(bus, device, function, 0x10 + 4 * bar_num as u32);
        let register_type = if bar_value & 0x1 == 1 {
            BaseAddressRegisterType::InputOutput
        } else {
            BaseAddressRegisterType::MemoryMapping
        };

        let address;
        let prefectchable;
        if register_type == BaseAddressRegisterType::MemoryMapping {
            prefectchable = ((bar_value >> 3) & 0x1) == 0x1;
            address = 0;
            // match (bar_value >> 1) & 0x3 {
            //     0 => // 32 Bit Mode
            //     1 => // 20 Bit mode
            //     2 => // 64 Bit Mode
            // }
            // return None;
        } else {
            address = ((bar_value & !0x3) & 0xFF).try_into().unwrap();
            prefectchable = false;
        }

        Some(BaseAddressRegister {
            prefetchable: prefectchable,
            address: address,
            size: 0,
            register_type: register_type,
        })
    }

    pub async fn select_drivers(&mut self, spawner: Spawner) {
        for bus in 0..8 {
            for device in 0..32 {
                let num_functions = if self.get_device_functions(bus, device) {
                    8
                } else {
                    1
                };
                for function in 0..num_functions {
                    let mut dev = self.get_device_descriptor(bus, device, function);
                    if dev.vendor_id == 0x0000 || dev.vendor_id == 0xFFFF {
                        // No more functions after this
                        continue;
                    }

                    for bar_num in 0..6 {
                        let bar =
                            match self.get_base_address_register(bus, device, function, bar_num) {
                                Some(bar) => bar,
                                None => continue,
                            };

                        // Ensure there is an address
                        // We currently only have support for IO not mmap
                        if bar.address != 0
                            && (bar.register_type == BaseAddressRegisterType::InputOutput)
                        {
                            dev.port_base = bar.address.into()
                        }

                        let driver = self.get_driver(&mut dev, spawner.clone());
                        if driver != 0 {
                            // Add driver
                        }
                    }

                    println!(
                        "PCI BUS {}, DEVICE {}, FUNCTION {} = VENDOR {:#X}{:X}, DEVICE {:#X}{:02X}",
                        bus & 0xFF,
                        device & 0xFF,
                        function & 0xFF,
                        (dev.vendor_id & 0xFF00) >> 8,
                        dev.vendor_id & 0xFF,
                        (dev.device_id & 0xFF00) >> 8,
                        dev.device_id & 0xFF
                    );
                }
                // Pass control over after each device scan
                yield_now().await;
            }
        }
    }
}
