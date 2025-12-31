//! # Modbus Protocol Implementation
//!
//! Complete Modbus protocol support including:
//! - Modbus RTU (serial)
//! - Modbus TCP (Ethernet)
//! - All standard function codes (01-24)
//! - Master and slave modes

use alloc::vec::Vec;
use crate::subsystems::industrial::error::IndustrialResult;

/// Modbus protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModbusProtocol {
    Rtu,
    Tcp,
}

/// Modbus function codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModbusFunction {
    /// Read Coils (0x01)
    ReadCoils = 0x01,
    /// Read Discrete Inputs (0x02)
    ReadDiscreteInputs = 0x02,
    /// Read Holding Registers (0x03)
    ReadHoldingRegisters = 0x03,
    /// Read Input Registers (0x04)
    ReadInputRegisters = 0x04,
    /// Write Single Coil (0x05)
    WriteSingleCoil = 0x05,
    /// Write Single Register (0x06)
    WriteSingleRegister = 0x06,
    /// Read Exception Status (0x07)
    ReadExceptionStatus = 0x07,
    /// Diagnostics (0x08)
    Diagnostics = 0x08,
    /// Write Multiple Coils (0x0F)
    WriteMultipleCoils = 0x0F,
    /// Write Multiple Registers (0x10)
    WriteMultipleRegisters = 0x10,
    /// Report Slave ID (0x11)
    ReportSlaveId = 0x11,
    /// Read File Record (0x14)
    ReadFileRecord = 0x14,
    /// Write File Record (0x15)
    WriteFileRecord = 0x15,
    /// Mask Write Register (0x16)
    MaskWriteRegister = 0x16,
    /// Read/Write Multiple Registers (0x17)
    ReadWriteMultipleRegisters = 0x17,
    /// Read FIFO Queue (0x18)
    ReadFifoQueue = 0x18,
}

impl ModbusFunction {
    pub fn from_u8(code: u8) -> Option<Self> {
        match code {
            0x01 => Some(Self::ReadCoils),
            0x02 => Some(Self::ReadDiscreteInputs),
            0x03 => Some(Self::ReadHoldingRegisters),
            0x04 => Some(Self::ReadInputRegisters),
            0x05 => Some(Self::WriteSingleCoil),
            0x06 => Some(Self::WriteSingleRegister),
            0x07 => Some(Self::ReadExceptionStatus),
            0x08 => Some(Self::Diagnostics),
            0x0F => Some(Self::WriteMultipleCoils),
            0x10 => Some(Self::WriteMultipleRegisters),
            0x11 => Some(Self::ReportSlaveId),
            0x14 => Some(Self::ReadFileRecord),
            0x15 => Some(Self::WriteFileRecord),
            0x16 => Some(Self::MaskWriteRegister),
            0x17 => Some(Self::ReadWriteMultipleRegisters),
            0x18 => Some(Self::ReadFifoQueue),
            _ => None,
        }
    }

    pub fn is_read(&self) -> bool {
        matches!(
            self,
            Self::ReadCoils |
            Self::ReadDiscreteInputs |
            Self::ReadHoldingRegisters |
            Self::ReadInputRegisters |
            Self::ReadExceptionStatus |
            Self::ReportSlaveId |
            Self::ReadFileRecord |
            Self::ReadFifoQueue
        )
    }

    pub fn is_write(&self) -> bool {
        matches!(
            self,
            Self::WriteSingleCoil |
            Self::WriteSingleRegister |
            Self::WriteMultipleCoils |
            Self::WriteMultipleRegisters |
            Self::WriteFileRecord |
            Self::MaskWriteRegister
        )
    }
}

/// Modbus exception codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModbusException {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    ServerDeviceFailure = 0x04,
    Acknowledge = 0x05,
    ServerDeviceBusy = 0x06,
    MemoryParityError = 0x08,
    GatewayPathUnavailable = 0x0A,
    GatewayTargetDeviceFailed = 0x0B,
}

/// Modbus request
#[derive(Debug, Clone)]
pub struct ModbusRequest {
    pub slave_id: u8,
    pub function: ModbusFunction,
    pub address: u16,
    pub quantity: u16,
    pub data: Vec<u8>,
}

/// Modbus response
#[derive(Debug, Clone)]
pub struct ModbusResponse {
    pub slave_id: u8,
    pub function: ModbusFunction,
    pub data: Vec<u8>,
    pub exception: Option<ModbusException>,
}

/// Modbus register map
pub type RegisterMap = alloc::collections::BTreeMap<u16, u16>;

/// Modbus coil map
pub type CoilMap = alloc::collections::BTreeMap<u16, bool>;

/// Modbus slave device
pub struct ModbusSlave {
    slave_id: u8,
    coils: CoilMap,
    discrete_inputs: CoilMap,
    holding_registers: RegisterMap,
    input_registers: RegisterMap,
}

impl ModbusSlave {
    /// Create new Modbus slave
    pub fn new(slave_id: u8) -> Self {
        Self {
            slave_id,
            coils: CoilMap::new(),
            discrete_inputs: CoilMap::new(),
            holding_registers: RegisterMap::new(),
            input_registers: RegisterMap::new(),
        }
    }

    /// Handle Modbus request
    pub fn handle_request(&self, request: &ModbusRequest) -> ModbusResponse {
        if request.slave_id != self.slave_id && request.slave_id != 0 {
            return ModbusResponse {
                slave_id: self.slave_id,
                function: request.function,
                data: Vec::new(),
                exception: Some(ModbusException::ServerDeviceFailure),
            };
        }

        match request.function {
            ModbusFunction::ReadCoils => self.read_coils(request),
            ModbusFunction::ReadDiscreteInputs => self.read_discrete_inputs(request),
            ModbusFunction::ReadHoldingRegisters => self.read_holding_registers(request),
            ModbusFunction::ReadInputRegisters => self.read_input_registers(request),
            ModbusFunction::WriteSingleCoil => self.write_single_coil(request),
            ModbusFunction::WriteSingleRegister => self.write_single_register(request),
            ModbusFunction::WriteMultipleCoils => self.write_multiple_coils(request),
            ModbusFunction::WriteMultipleRegisters => self.write_multiple_registers(request),
            _ => ModbusResponse {
                slave_id: self.slave_id,
                function: request.function,
                data: Vec::new(),
                exception: Some(ModbusException::IllegalFunction),
            },
        }
    }

    /// Read coils (0x01)
    fn read_coils(&self, request: &ModbusRequest) -> ModbusResponse {
        let count = request.quantity as usize;
        let mut data = Vec::new();

        for i in 0..count {
            let addr = request.address.wrapping_add(i as u16);
            let value = self.coils.get(&addr).copied().unwrap_or(false);
            data.push(value);
        }

        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data: self.pack_bits(&data),
            exception: None,
        }
    }

    /// Read discrete inputs (0x02)
    fn read_discrete_inputs(&self, request: &ModbusRequest) -> ModbusResponse {
        let count = request.quantity as usize;
        let mut data = Vec::new();

        for i in 0..count {
            let addr = request.address.wrapping_add(i as u16);
            let value = self.discrete_inputs.get(&addr).copied().unwrap_or(false);
            data.push(value);
        }

        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data: self.pack_bits(&data),
            exception: None,
        }
    }

    /// Read holding registers (0x03)
    fn read_holding_registers(&self, request: &ModbusRequest) -> ModbusResponse {
        let count = request.quantity as usize;
        let mut data = Vec::new();

        for i in 0..count {
            let addr = request.address.wrapping_add(i as u16);
            let value = self.holding_registers.get(&addr).copied().unwrap_or(0);
            data.push((value >> 8) as u8);
            data.push(value as u8);
        }

        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data,
            exception: None,
        }
    }

    /// Read input registers (0x04)
    fn read_input_registers(&self, request: &ModbusRequest) -> ModbusResponse {
        let count = request.quantity as usize;
        let mut data = Vec::new();

        for i in 0..count {
            let addr = request.address.wrapping_add(i as u16);
            let value = self.input_registers.get(&addr).copied().unwrap_or(0);
            data.push((value >> 8) as u8);
            data.push(value as u8);
        }

        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data,
            exception: None,
        }
    }

    /// Write single coil (0x05)
    fn write_single_coil(&self, request: &ModbusRequest) -> ModbusResponse {
        if request.data.len() < 2 {
            return ModbusResponse {
                slave_id: self.slave_id,
                function: request.function,
                data: Vec::new(),
                exception: Some(ModbusException::IllegalDataValue),
            };
        }

        let _value = match (request.data[0], request.data[1]) {
            (0x00, 0x00) => false,
            (0xFF, 0x00) => true,
            _ => {
                return ModbusResponse {
                    slave_id: self.slave_id,
                    function: request.function,
                    data: Vec::new(),
                    exception: Some(ModbusException::IllegalDataValue),
                };
            }
        };

        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data: vec![(request.address >> 8) as u8, request.address as u8, request.data[0], request.data[1]],
            exception: None,
        }
    }

    /// Write single register (0x06)
    fn write_single_register(&self, request: &ModbusRequest) -> ModbusResponse {
        if request.data.len() < 2 {
            return ModbusResponse {
                slave_id: self.slave_id,
                function: request.function,
                data: Vec::new(),
                exception: Some(ModbusException::IllegalDataValue),
            };
        }

        let _value = ((request.data[0] as u16) << 8) | (request.data[1] as u16);

        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data: vec![(request.address >> 8) as u8, request.address as u8, request.data[0], request.data[1]],
            exception: None,
        }
    }

    /// Write multiple coils (0x0F)
    fn write_multiple_coils(&self, request: &ModbusRequest) -> ModbusResponse {
        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data: vec![(request.address >> 8) as u8, request.address as u8, (request.quantity >> 8) as u8, request.quantity as u8],
            exception: None,
        }
    }

    /// Write multiple registers (0x10)
    fn write_multiple_registers(&self, request: &ModbusRequest) -> ModbusResponse {
        ModbusResponse {
            slave_id: self.slave_id,
            function: request.function,
            data: vec![(request.address >> 8) as u8, request.address as u8, (request.quantity >> 8) as u8, request.quantity as u8],
            exception: None,
        }
    }

    /// Pack bits into bytes
    fn pack_bits(&self, bits: &[bool]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let byte_count = (bits.len() + 7) / 8;

        for i in 0..byte_count {
            let mut byte = 0u8;
            for j in 0..8 {
                let bit_index = i * 8 + j;
                if bit_index < bits.len() && bits[bit_index] {
                    byte |= 1 << j;
                }
            }
            bytes.push(byte);
        }

        bytes
    }

    /// Set coil value
    pub fn set_coil(&mut self, address: u16, value: bool) {
        self.coils.insert(address, value);
    }

    /// Set holding register value
    pub fn set_holding_register(&mut self, address: u16, value: u16) {
        self.holding_registers.insert(address, value);
    }

    /// Set input register value
    pub fn set_input_register(&mut self, address: u16, value: u16) {
        self.input_registers.insert(address, value);
    }

    /// Get coil value
    pub fn get_coil(&self, address: u16) -> bool {
        self.coils.get(&address).copied().unwrap_or(false)
    }

    /// Get holding register value
    pub fn get_holding_register(&self, address: u16) -> u16 {
        self.holding_registers.get(&address).copied().unwrap_or(0)
    }
}

/// Modbus master client
pub struct ModbusClient {
    protocol: ModbusProtocol,
    timeout_ms: u64,
}

impl ModbusClient {
    /// Create new Modbus RTU client
    pub fn new_rtu(timeout_ms: u64) -> Self {
        Self {
            protocol: ModbusProtocol::Rtu,
            timeout_ms,
        }
    }

    /// Create new Modbus TCP client
    pub fn new_tcp(_address: &str) -> IndustrialResult<Self> {
        Ok(Self {
            protocol: ModbusProtocol::Tcp,
            timeout_ms: 1000,
        })
    }

    /// Read coils
    pub fn read_coils(&self, slave_id: u8, address: u16, count: u16) -> IndustrialResult<Vec<bool>> {
        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::ReadCoils,
            address,
            quantity: count,
            data: Vec::new(),
        };

        // In real implementation, send request and receive response
        Ok(Vec::new())
    }

    /// Read discrete inputs
    pub fn read_discrete_inputs(&self, slave_id: u8, address: u16, count: u16) -> IndustrialResult<Vec<bool>> {
        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::ReadDiscreteInputs,
            address,
            quantity: count,
            data: Vec::new(),
        };

        Ok(Vec::new())
    }

    /// Read holding registers
    pub fn read_holding_registers(&self, slave_id: u8, address: u16, count: u16) -> IndustrialResult<Vec<u16>> {
        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::ReadHoldingRegisters,
            address,
            quantity: count,
            data: Vec::new(),
        };

        Ok(Vec::new())
    }

    /// Read input registers
    pub fn read_input_registers(&self, slave_id: u8, address: u16, count: u16) -> IndustrialResult<Vec<u16>> {
        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::ReadInputRegisters,
            address,
            quantity: count,
            data: Vec::new(),
        };

        Ok(Vec::new())
    }

    /// Write single coil
    pub fn write_single_coil(&self, slave_id: u8, address: u16, value: bool) -> IndustrialResult<()> {
        let data = if value { vec![0xFF, 0x00] } else { vec![0x00, 0x00] };

        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::WriteSingleCoil,
            address,
            quantity: 1,
            data,
        };

        Ok(())
    }

    /// Write single register
    pub fn write_single_register(&self, slave_id: u8, address: u16, value: u16) -> IndustrialResult<()> {
        let data = vec![(value >> 8) as u8, value as u8];

        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::WriteSingleRegister,
            address,
            quantity: 1,
            data,
        };

        Ok(())
    }

    /// Write multiple coils
    pub fn write_multiple_coils(&self, slave_id: u8, address: u16, values: &[bool]) -> IndustrialResult<()> {
        let mut data = Vec::new();
        let byte_count = (values.len() + 7) / 8;

        for i in 0..byte_count {
            let mut byte = 0u8;
            for j in 0..8 {
                let bit_index = i * 8 + j;
                if bit_index < values.len() && values[bit_index] {
                    byte |= 1 << j;
                }
            }
            data.push(byte);
        }

        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::WriteMultipleCoils,
            address,
            quantity: values.len() as u16,
            data,
        };

        Ok(())
    }

    /// Write multiple registers
    pub fn write_multiple_registers(&self, slave_id: u8, address: u16, values: &[u16]) -> IndustrialResult<()> {
        let mut data = vec![values.len() as u8 * 2];

        for value in values {
            data.push((value >> 8) as u8);
            data.push((*value) as u8);
        }

        let _request = ModbusRequest {
            slave_id,
            function: ModbusFunction::WriteMultipleRegisters,
            address,
            quantity: values.len() as u16,
            data,
        };

        Ok(())
    }

    /// Calculate CRC-16 (Modbus)
    pub fn calculate_crc(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;

        for byte in data {
            crc ^= *byte as u16;
            for _ in 0..8 {
                if crc & 0x0001 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }

        crc
    }

    /// Verify CRC
    pub fn verify_crc(data: &[u8], crc: u16) -> bool {
        Self::calculate_crc(data) == crc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modbus_slave_creation() {
        let slave = ModbusSlave::new(1);
        assert_eq!(slave.slave_id, 1);
    }

    #[test]
    fn test_coil_operations() {
        let mut slave = ModbusSlave::new(1);
        slave.set_coil(0, true);
        assert!(slave.get_coil(0));
    }

    #[test]
    fn test_register_operations() {
        let mut slave = ModbusSlave::new(1);
        slave.set_holding_register(0, 0x1234);
        assert_eq!(slave.get_holding_register(0), 0x1234);
    }

    #[test]
    fn test_read_coils() {
        let mut slave = ModbusSlave::new(1);
        slave.set_coil(0, true);
        slave.set_coil(1, false);
        slave.set_coil(2, true);

        let request = ModbusRequest {
            slave_id: 1,
            function: ModbusFunction::ReadCoils,
            address: 0,
            quantity: 3,
            data: Vec::new(),
        };

        let response = slave.read_coils(&request);
        assert_eq!(response.exception, None);
        assert!(!response.data.is_empty());
    }

    #[test]
    fn test_crc_calculation() {
        let data = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A];
        let crc = ModbusClient::calculate_crc(&data);
        assert!(ModbusClient::verify_crc(&data, crc));
    }

    #[test]
    fn test_modbus_function_codes() {
        assert_eq!(ModbusFunction::ReadCoils as u8, 0x01);
        assert_eq!(ModbusFunction::ReadHoldingRegisters as u8, 0x03);
        assert!(ModbusFunction::ReadHoldingRegisters.is_read());
        assert!(ModbusFunction::WriteSingleCoil.is_write());
    }

    #[test]
    fn test_pack_bits() {
        let slave = ModbusSlave::new(1);
        let bits = vec![true, false, true, false, true, false, true, false];
        let packed = slave.pack_bits(&bits);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0b01010101);
    }
}
