//! # Profibus Protocol Implementation
//!
//! Profibus DP (Decentralized Peripherals) and PA (Process Automation)
//! implementation with token passing and real-time communication.

use alloc::{
    collections::BTreeMap,
    vec::Vec,
};
use crate::subsystems::industrial::error::{IndustrialError, IndustrialResult, ProtocolError};

/// Profibus type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfibusType {
    Dp,  // Decentralized Peripherals
    Pa,  // Process Automation
}

/// Profibus frame format
#[derive(Debug, Clone)]
pub struct ProfibusFrame {
    pub sd: StartDelimiter,
    pub le: u8,  // Length
    pub le_r: u8,  // Length repeated
    pub da: u8,  // Destination address
    pub sa: u8,  // Source address
    pub fc: FunctionCode,
    pub data: Vec<u8>,
    pub fcs: u8,  // Frame check sequence
    pub ed: EndDelimiter,
}

/// Start delimiter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StartDelimiter {
    FrameFormatVariable = 0xDC,
    FrameFormatFixed = 0xDD,
    FrameToken = 0x3C,
}

/// End delimiter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EndDelimiter {
    Standard = 0x16,
}

/// Function code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionCode(u8);

impl FunctionCode {
    pub const REQUEST: u8 = 0x00;
    pub const RESPONSE: u8 = 0x40;
    pub const ACK: u8 = 0x80;

    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn is_request(&self) -> bool {
        self.0 & 0xC0 == 0
    }

    pub fn is_response(&self) -> bool {
        self.0 & 0xC0 == 0x40
    }
}

/// Token frame
#[derive(Debug, Clone)]
pub struct TokenFrame {
    pub sa: u8,  // Source address
    pub da: u8,  // Destination address
}

/// Profibus master
pub struct ProfibusMaster {
    master_address: u8,
    slave_addresses: Vec<u8>,
    token_holder: u8,
    profibus_type: ProfibusType,
}

impl ProfibusMaster {
    /// Create new Profibus master
    pub fn new(master_address: u8, profibus_type: ProfibusType) -> Self {
        Self {
            master_address,
            slave_addresses: Vec::new(),
            token_holder: master_address,
            profibus_type,
        }
    }

    /// Add slave device
    pub fn add_slave(&mut self, address: u8) -> IndustrialResult<()> {
        if address == self.master_address {
            return Err(IndustrialError::Protocol(ProtocolError::InvalidAddress(address as u16)));
        }

        if self.slave_addresses.contains(&address) {
            return Err(IndustrialError::Protocol(ProtocolError::InvalidAddress(address as u16)));
        }

        self.slave_addresses.push(address);
        Ok(())
    }

    /// Pass token to next master
    pub fn pass_token(&mut self, next_master: u8) -> IndustrialResult<TokenFrame> {
        if !self.slave_addresses.contains(&next_master) {
            return Err(IndustrialError::Protocol(ProtocolError::InvalidAddress(next_master as u16)));
        }

        self.token_holder = next_master;
        Ok(TokenFrame {
            sa: self.master_address,
            da: next_master,
        })
    }

    /// Send request to slave
    pub fn send_request(&self, slave_addr: u8, data: &[u8]) -> IndustrialResult<ProfibusFrame> {
        let frame = ProfibusFrame {
            sd: StartDelimiter::FrameFormatVariable,
            le: (data.len() + 3) as u8,
            le_r: (data.len() + 3) as u8,
            da: slave_addr,
            sa: self.master_address,
            fc: FunctionCode::new(FunctionCode::REQUEST),
            data: data.to_vec(),
            fcs: 0,  // Calculate FCS
            ed: EndDelimiter::Standard,
        };

        Ok(frame)
    }

    /// Calculate frame check sequence
    pub fn calculate_fcs(&self, frame: &ProfibusFrame) -> u8 {
        let mut fcs = 0u8;
        fcs ^= frame.da;
        fcs ^= frame.sa;
        fcs ^= frame.fc.0;

        for byte in &frame.data {
            fcs ^= byte;
        }

        fcs
    }

    /// Get token holder
    pub fn token_holder(&self) -> u8 {
        self.token_holder
    }
}

/// Profibus slave device
pub struct ProfibusSlave {
    slave_address: u8,
    input_data: BTreeMap<u16, u8>,
    output_data: BTreeMap<u16, u8>,
}

impl ProfibusSlave {
    /// Create new Profibus slave
    pub fn new(slave_address: u8) -> Self {
        Self {
            slave_address,
            input_data: BTreeMap::new(),
            output_data: BTreeMap::new(),
        }
    }

    /// Handle incoming request
    pub fn handle_request(&self, frame: &ProfibusFrame) -> IndustrialResult<ProfibusFrame> {
        if frame.da != self.slave_address {
            return Err(IndustrialError::Protocol(ProtocolError::InvalidAddress(frame.da as u16)));
        }

        let response = ProfibusFrame {
            sd: StartDelimiter::FrameFormatVariable,
            le: 3,
            le_r: 3,
            da: frame.sa,
            sa: self.slave_address,
            fc: FunctionCode::new(FunctionCode::RESPONSE),
            data: Vec::new(),
            fcs: 0,
            ed: EndDelimiter::Standard,
        };

        Ok(response)
    }

    /// Set input data
    pub fn set_input(&mut self, address: u16, value: u8) {
        self.input_data.insert(address, value);
    }

    /// Get output data
    pub fn get_output(&self, address: u16) -> Option<u8> {
        self.output_data.get(&address).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profibus_master_creation() {
        let master = ProfibusMaster::new(0, ProfibusType::Dp);
        assert_eq!(master.master_address, 0);
    }

    #[test]
    fn test_add_slave() {
        let mut master = ProfibusMaster::new(0, ProfibusType::Dp);
        master.add_slave(1).unwrap();
        assert_eq!(master.slave_addresses.len(), 1);
    }

    #[test]
    fn test_token_passing() {
        let mut master = ProfibusMaster::new(0, ProfibusType::Dp);
        master.add_slave(1).unwrap();
        let token = master.pass_token(1).unwrap();
        assert_eq!(token.da, 1);
    }

    #[test]
    fn test_fcs_calculation() {
        let master = ProfibusMaster::new(0, ProfibusType::Dp);
        let frame = ProfibusFrame {
            sd: StartDelimiter::FrameFormatVariable,
            le: 3,
            le_r: 3,
            da: 1,
            sa: 0,
            fc: FunctionCode::new(0x00),
            data: Vec::new(),
            fcs: 0,
            ed: EndDelimiter::Standard,
        };

        let fcs = master.calculate_fcs(&frame);
        assert_eq!(fcs, 1);  // 0 XOR 1 = 1
    }
}
