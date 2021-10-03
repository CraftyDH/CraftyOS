use core::convert::TryInto;

use x86_64::instructions::port::Port;

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

    pub fn select_drivers(&mut self) {
        for bus in 0..8 {
            for device in 0..32 {
                let num_functions = if self.get_device_functions(bus, device) {
                    8
                } else {
                    1
                };
                for function in 0..num_functions {
                    let dev = self.get_device_descriptor(bus, device, function);
                    if dev.vendor_id == 0x0000 || dev.vendor_id == 0xFFFF {
                        // No more functions after this
                        break;
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
            }
        }
    }
}
