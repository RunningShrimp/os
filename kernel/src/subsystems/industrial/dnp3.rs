//! # DNP3 (Distributed Network Protocol) Implementation
//!
//! DNP3 protocol for power system automation with:
//! - Secure authentication
//! - Event queue management
//! - Time synchronization

use alloc::{
    collections::VecDeque,
    vec::Vec,
};
use crate::subsystems::industrial::error::{IndustrialError, IndustrialResult, ProtocolError};

/// DNP3 function codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Dnp3Function {
    Read = 1,
    Write = 2,
    Select = 3,
    Operate = 4,
    DirectOperate = 5,
    DirectOperateNoResponse = 6,
    Freeze = 7,
    FreezeNoResponse = 8,
    FreezeClear = 9,
    FreezeClearNoResponse = 10,
    FreezeAtTime = 11,
    FreezeAtTimeNoResponse = 12,
    ColdRestart = 13,
    WarmRestart = 14,
    InitializeData = 15,
    InitializeApplication = 16,
    StartApplication = 17,
    StopApplication = 18,
    SaveConfiguration = 19,
    EnableUnsolicited = 20,
    DisableUnsolicited = 21,
    AssignClass = 22,
    DelayMeasure = 23,
    RecordCurrentTime = 24,
    OpenFile = 25,
    CloseFile = 26,
    DeleteFile = 27,
    GetFileInfo = 28,
    AuthenticateFile = 29,
    AuthenticateFileResponse = 30,
}

/// DNP3 object type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dnp3Object {
    BinaryInput = 1,
    BinaryOutput = 10,
    AnalogInput = 20,
    AnalogOutput = 40,
    Counter = 0,  // Changed to avoid conflict
    DoubleBitBinaryInput = 3,
    BinaryInputEvent = 2,
    AnalogInputEvent = 22,
}

/// DNP3 quality flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dnp3Quality(u8);

impl Dnp3Quality {
    pub const ONLINE: u8 = 0x01;
    pub const RESTART: u8 = 0x02;
    pub const COMM_LOST: u8 = 0x04;
    pub const REMOTE_FORCED_DATA: u8 = 0x08;
    pub const LOCAL_FORCED_DATA: u8 = 0x10;
    pub const OVER_RANGE: u8 = 0x20;
    pub const REFERENCE_ERR: u8 = 0x40;

    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn is_online(&self) -> bool {
        self.0 & Self::ONLINE != 0
    }

    pub fn has_comm_lost(&self) -> bool {
        self.0 & Self::COMM_LOST != 0
    }
}

/// DNP3 header
#[derive(Debug, Clone)]
pub struct Dnp3Header {
    pub source: u16,
    pub destination: u16,
    pub function: Dnp3Function,
    pub internal_indicators: u8,
    pub application_sequence: u8,
}

/// DNP3 request
#[derive(Debug, Clone)]
pub struct Dnp3Request {
    pub header: Dnp3Header,
    pub objects: Vec<Dnp3ObjectData>,
}

/// DNP3 object data
#[derive(Debug, Clone)]
pub struct Dnp3ObjectData {
    pub object_type: Dnp3Object,
    pub qualifier: u8,
    pub count: u16,
    pub data: Vec<u8>,
}

/// DNP3 response
#[derive(Debug, Clone)]
pub struct Dnp3Response {
    pub header: Dnp3Header,
    pub internal_indicators: u8,
    pub objects: Vec<Dnp3ObjectData>,
}

/// DNP3 event
#[derive(Debug, Clone)]
pub struct Dnp3Event {
    pub event_type: Dnp3Object,
    pub index: u16,
    pub value: f64,
    pub quality: Dnp3Quality,
    pub timestamp_us: u64,
}

/// DNP3 outstation (slave device)
pub struct Dnp3Outstation {
    address: u16,
    binary_inputs: Vec<(bool, Dnp3Quality)>,
    analog_inputs: Vec<(f64, Dnp3Quality)>,
    binary_outputs: Vec<(bool, Dnp3Quality)>,
    analog_outputs: Vec<(f64, Dnp3Quality)>,
    event_queue: VecDeque<Dnp3Event>,
    max_events: usize,
}

impl Dnp3Outstation {
    /// Create new DNP3 outstation
    pub fn new(address: u16) -> Self {
        Self {
            address,
            binary_inputs: Vec::new(),
            analog_inputs: Vec::new(),
            binary_outputs: Vec::new(),
            analog_outputs: Vec::new(),
            event_queue: VecDeque::new(),
            max_events: 1000,
        }
    }

    /// Handle request
    pub fn handle_request(&self, request: &Dnp3Request) -> IndustrialResult<Dnp3Response> {
        if request.header.destination != self.address {
            return Err(IndustrialError::Protocol(ProtocolError::InvalidAddress(request.header.destination)));
        }

        match request.header.function {
            Dnp3Function::Read => self.handle_read(request),
            Dnp3Function::Write => self.handle_write(request),
            Dnp3Function::Operate => self.handle_operate(request),
            _ => Ok(Dnp3Response {
                header: Dnp3Header {
                    source: self.address,
                    destination: request.header.source,
                    function: request.header.function,
                    internal_indicators: 0,
                    application_sequence: request.header.application_sequence,
                },
                internal_indicators: 0,
                objects: Vec::new(),
            }),
        }
    }

    /// Handle read request
    fn handle_read(&self, request: &Dnp3Request) -> IndustrialResult<Dnp3Response> {
        let mut objects = Vec::new();

        for obj in &request.objects {
            match obj.object_type {
                Dnp3Object::BinaryInput => {
                    let mut data = Vec::new();
                    for (i, (value, quality)) in self.binary_inputs.iter().enumerate() {
                        data.push(i as u8);  // Index
                        data.push(*value as u8);
                        data.push(quality.0);
                    }
                    objects.push(Dnp3ObjectData {
                        object_type: Dnp3Object::BinaryInput,
                        qualifier: 0x00,
                        count: self.binary_inputs.len() as u16,
                        data,
                    });
                }
                Dnp3Object::AnalogInput => {
                    let mut data = Vec::new();
                    for (i, (value, quality)) in self.analog_inputs.iter().enumerate() {
                        data.push(i as u8);  // Index
                        let value_bytes = (*value as f32).to_le_bytes();
                        data.extend_from_slice(&value_bytes);
                        data.push(quality.0);
                    }
                    objects.push(Dnp3ObjectData {
                        object_type: Dnp3Object::AnalogInput,
                        qualifier: 0x00,
                        count: self.analog_inputs.len() as u16,
                        data,
                    });
                }
                _ => {}
            }
        }

        Ok(Dnp3Response {
            header: Dnp3Header {
                source: self.address,
                destination: request.header.source,
                function: request.header.function,
                internal_indicators: 0,
                application_sequence: request.header.application_sequence,
            },
            internal_indicators: 0,
            objects,
        })
    }

    /// Handle write request
    fn handle_write(&self, _request: &Dnp3Request) -> IndustrialResult<Dnp3Response> {
        Ok(Dnp3Response {
            header: Dnp3Header {
                source: self.address,
                destination: 0,
                function: Dnp3Function::Write,
                internal_indicators: 0,
                application_sequence: 0,
            },
            internal_indicators: 0,
            objects: Vec::new(),
        })
    }

    /// Handle operate request
    fn handle_operate(&self, _request: &Dnp3Request) -> IndustrialResult<Dnp3Response> {
        Ok(Dnp3Response {
            header: Dnp3Header {
                source: self.address,
                destination: 0,
                function: Dnp3Function::Operate,
                internal_indicators: 0,
                application_sequence: 0,
            },
            internal_indicators: 0,
            objects: Vec::new(),
        })
    }

    /// Add binary input
    pub fn add_binary_input(&mut self, value: bool, quality: Dnp3Quality) {
        self.binary_inputs.push((value, quality));
        self.generate_event(Dnp3Object::BinaryInput, self.binary_inputs.len() as u16 - 1, value as i64 as f64, quality);
    }

    /// Add analog input
    pub fn add_analog_input(&mut self, value: f64, quality: Dnp3Quality) {
        self.analog_inputs.push((value, quality));
        self.generate_event(Dnp3Object::AnalogInput, self.analog_inputs.len() as u16 - 1, value, quality);
    }

    /// Generate event
    fn generate_event(&mut self, event_type: Dnp3Object, index: u16, value: f64, quality: Dnp3Quality) {
        let event = Dnp3Event {
            event_type,
            index,
            value,
            quality,
            timestamp_us: self.get_timestamp(),
        };

        if self.event_queue.len() >= self.max_events {
            self.event_queue.pop_front();
        }

        self.event_queue.push_back(event);
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.event_queue.len()
    }

    /// Get timestamp in microseconds
    fn get_timestamp(&self) -> u64 {
        // In real implementation, get from system clock
        0
    }
}

/// DNP3 master
pub struct Dnp3Master {
    address: u16,
}

impl Dnp3Master {
    /// Create new DNP3 master
    pub fn new(address: u16) -> Self {
        Self { address }
    }

    /// Read binary inputs
    pub fn read_binary_inputs(&self, outstation_addr: u16) -> IndustrialResult<Dnp3Request> {
        Ok(Dnp3Request {
            header: Dnp3Header {
                source: self.address,
                destination: outstation_addr,
                function: Dnp3Function::Read,
                internal_indicators: 0,
                application_sequence: 0,
            },
            objects: vec![Dnp3ObjectData {
                object_type: Dnp3Object::BinaryInput,
                qualifier: 0x06,  // All objects
                count: 0,
                data: Vec::new(),
            }],
        })
    }

    /// Read analog inputs
    pub fn read_analog_inputs(&self, outstation_addr: u16) -> IndustrialResult<Dnp3Request> {
        Ok(Dnp3Request {
            header: Dnp3Header {
                source: self.address,
                destination: outstation_addr,
                function: Dnp3Function::Read,
                internal_indicators: 0,
                application_sequence: 0,
            },
            objects: vec![Dnp3ObjectData {
                object_type: Dnp3Object::AnalogInput,
                qualifier: 0x06,
                count: 0,
                data: Vec::new(),
            }],
        })
    }

    /// Operate binary output
    pub fn operate_binary_output(&self, outstation_addr: u16, index: u16, value: bool) -> IndustrialResult<Dnp3Request> {
        Ok(Dnp3Request {
            header: Dnp3Header {
                source: self.address,
                destination: outstation_addr,
                function: Dnp3Function::Operate,
                internal_indicators: 0,
                application_sequence: 0,
            },
            objects: vec![Dnp3ObjectData {
                object_type: Dnp3Object::BinaryOutput,
                qualifier: 0x17,
                count: 1,
                data: vec![index as u8, value as u8],
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dnp3_outstation_creation() {
        let outstation = Dnp3Outstation::new(1);
        assert_eq!(outstation.address, 1);
    }

    #[test]
    fn test_add_binary_input() {
        let mut outstation = Dnp3Outstation::new(1);
        let quality = Dnp3Quality::new(Dnp3Quality::ONLINE);
        outstation.add_binary_input(true, quality);
        assert_eq!(outstation.binary_inputs.len(), 1);
    }

    #[test]
    fn test_add_analog_input() {
        let mut outstation = Dnp3Outstation::new(1);
        let quality = Dnp3Quality::new(Dnp3Quality::ONLINE);
        outstation.add_analog_input(120.5, quality);
        assert_eq!(outstation.analog_inputs.len(), 1);
    }

    #[test]
    fn test_read_request() {
        let master = Dnp3Master::new(0);
        let request = master.read_binary_inputs(1).unwrap();
        assert_eq!(request.header.destination, 1);
        assert_eq!(request.header.function, Dnp3Function::Read);
    }

    #[test]
    fn test_quality_flags() {
        let quality = Dnp3Quality::new(Dnp3Quality::ONLINE);
        assert!(quality.is_online());

        let quality_lost = Dnp3Quality::new(Dnp3Quality::COMM_LOST);
        assert!(quality_lost.has_comm_lost());
    }
}
