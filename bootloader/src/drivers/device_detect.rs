// Device detection and discovery during boot

#[derive(Clone, Copy, Debug)]
pub enum DeviceType {
    Serial = 0,
    Disk = 1,
    Nic = 2,
    Timer = 3,
    Pic = 4,
}

#[derive(Clone, Copy, Debug)]
pub struct Device {
    pub device_type: DeviceType,
    pub port_or_addr: u64,
    pub irq: u8,
    pub detected: bool,
}

impl Device {
    pub fn new(device_type: DeviceType) -> Self {
        Self {
            device_type,
            port_or_addr: 0,
            irq: 0,
            detected: false,
        }
    }
}

pub struct DeviceDetector {
    devices: [Device; 16],
    count: usize,
}

impl DeviceDetector {
    pub fn new() -> Self {
        Self {
            devices: [Device::new(DeviceType::Serial); 16],
            count: 0,
        }
    }

    pub fn detect_all(&mut self) {
        #[cfg(target_arch = "x86_64")]
        {
            self.detect_x86_64_devices();
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.detect_aarch64_devices();
        }

        #[cfg(target_arch = "riscv64")]
        {
            self.detect_riscv64_devices();
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn detect_x86_64_devices(&mut self) {
        // COM1 serial port at 0x3F8
        if self.count < 16 {
            let mut device = Device::new(DeviceType::Serial);
            device.port_or_addr = 0x3F8;
            device.irq = 4;
            device.detected = true;
            self.devices[self.count] = device;
            self.count += 1;
        }

        // PIT timer at I/O ports 0x40-0x43
        if self.count < 16 {
            let mut device = Device::new(DeviceType::Timer);
            device.port_or_addr = 0x40;
            device.irq = 0;
            device.detected = true;
            self.devices[self.count] = device;
            self.count += 1;
        }

        // PIC at 0x20 and 0xA0
        if self.count < 16 {
            let mut device = Device::new(DeviceType::Pic);
            device.port_or_addr = 0x20;
            device.irq = 255;
            device.detected = true;
            self.devices[self.count] = device;
            self.count += 1;
        }
    }

    #[cfg(target_arch = "aarch64")]
    fn detect_aarch64_devices(&mut self) {
        // ARM64 PL011 UART at 0x09000000
        if self.count < 16 {
            let mut device = Device::new(DeviceType::Serial);
            device.port_or_addr = 0x09000000;
            device.irq = 33;
            device.detected = true;
            self.devices[self.count] = device;
            self.count += 1;
        }

        // ARM Generic Timer
        if self.count < 16 {
            let mut device = Device::new(DeviceType::Timer);
            device.port_or_addr = 0;
            device.irq = 27;
            device.detected = true;
            self.devices[self.count] = device;
            self.count += 1;
        }
    }

    #[cfg(target_arch = "riscv64")]
    fn detect_riscv64_devices(&mut self) {
        // RISC-V UART16550 at 0x10000000
        if self.count < 16 {
            let mut device = Device::new(DeviceType::Serial);
            device.port_or_addr = 0x10000000;
            device.irq = 10;
            device.detected = true;
            self.devices[self.count] = device;
            self.count += 1;
        }

        // RISC-V CLINT (Core Local Interruptor)
        if self.count < 16 {
            let mut device = Device::new(DeviceType::Timer);
            device.port_or_addr = 0x02000000;
            device.irq = 0;
            device.detected = true;
            self.devices[self.count] = device;
            self.count += 1;
        }
    }

    pub fn print_detected(&self) {
        crate::drivers::console::write_str("Detected devices:\n");
        for i in 0..self.count {
            let device = self.devices[i];
            crate::drivers::console::write_str("  ");
            match device.device_type {
                DeviceType::Serial => {
                    crate::drivers::console::write_str("Serial @ ");
                }
                DeviceType::Disk => {
                    crate::drivers::console::write_str("Disk @ ");
                }
                DeviceType::Nic => {
                    crate::drivers::console::write_str("NIC @ ");
                }
                DeviceType::Timer => {
                    crate::drivers::console::write_str("Timer @ ");
                }
                DeviceType::Pic => {
                    crate::drivers::console::write_str("PIC @ ");
                }
            }
            crate::drivers::console::write_str("0x");
            crate::drivers::console::write_str(if device.port_or_addr > 0 {
                "..."
            } else {
                "0"
            });
            crate::drivers::console::write_str("\n");
        }
    }

    pub fn device_count(&self) -> usize {
        self.count
    }
}

impl Default for DeviceDetector {
    fn default() -> Self {
        Self::new()
    }
}
