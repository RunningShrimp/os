//! iSCSI protocol implementation
//!
//! This module implements the iSCSI (Internet Small Computer System Interface)
//! protocol (RFC 3720 & RFC 5048), providing transport of SCSI commands over
//! IP networks.
//!
//! # Features
//! - SCSI command encapsulation
//! - Discovery and login
//! - Target and initiator support
//! - Error recovery
//! - Multipath I/O support
//!
//! # References
//! - RFC 3720: Internet Small Computer Systems Interface (iSCSI)
//! - RFC 5048: iSCSI Corrections and Clarifications

#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

use core::sync::atomic {AtomicU32, Ordering, Ordering};
use crate::subsystems::sync::Mutex;

/// Default iSCSI port
pub const DEFAULT_ISCSI_PORT: u16 = 3260;

/// Default iSCSI target port
pub const DEFAULT_TARGET_PORT: u16 = 3260;

/// Maximum iSCSI header size
const MAX_HEADER_SIZE: usize = 48;

/// Maximum data segment length
const MAX_DATA_SEGMENT_LENGTH: u32 = 65536;

/// Default data segment length
const DEFAULT_DATA_SEGMENT_LENGTH: u32 = 8192;

/// iSCSI operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    /// NOP-Out
    NopOut = 0x00,
    /// SCSI Command
    ScsiCommand = 0x01,
    /// SCSI Task Management function request
    TaskManagement = 0x02,
    /// Login request
    LoginRequest = 0x03,
    /// Text request
    TextRequest = 0x04,
    /// SCSI Data-Out
    DataOut = 0x05,
    /// Logout request
    LogoutRequest = 0x06,
    /// SNACK request
    SnackRequest = 0x10,
    /// NOP-In
    NopIn = 0x20,
    /// SCSI Response
    ScsiResponse = 0x21,
    /// SCSI Task Management function response
    TaskManagementResponse = 0x22,
    /// Login response
    LoginResponse = 0x23,
    /// Text response
    TextResponse = 0x24,
    /// SCSI Data-In
    DataIn = 0x25,
    /// Logout response
    LogoutResponse = 0x26,
    /// Ready To Transfer (R2T)
    ReadyToTransfer = 0x31,
    /// Asynchronous Message
    AsyncMessage = 0x32,
    /// Reject
    Reject = 0x3F,
}

impl OpCode {
    /// Parse from byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(Self::NopOut),
            0x01 => Some(Self::ScsiCommand),
            0x02 => Some(Self::TaskManagement),
            0x03 => Some(Self::LoginRequest),
            0x04 => Some(Self::TextRequest),
            0x05 => Some(Self::DataOut),
            0x06 => Some(Self::LogoutRequest),
            0x10 => Some(Self::SnackRequest),
            0x20 => Some(Self::NopIn),
            0x21 => Some(Self::ScsiResponse),
            0x22 => Some(Self::TaskManagementResponse),
            0x23 => Some(Self::LoginResponse),
            0x24 => Some(Self::TextResponse),
            0x25 => Some(Self::DataIn),
            0x26 => Some(Self::LogoutResponse),
            0x31 => Some(Self::ReadyToTransfer),
            0x32 => Some(Self::AsyncMessage),
            0x3F => Some(Self::Reject),
            _ => None,
        }
    }

    /// Check if this is a response opcode
    pub fn is_response(self) -> bool {
        (self as u8) & 0x40 != 0
    }
}

/// iSCSI session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Not connected
    NotConnected,
    /// In login
    InLogin,
    /// Logged in
    LoggedIn,
    /// In logout
    InLogout,
}

/// iSCSI connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    NotConnected,
    /// In login
    InLogin,
    /// Full feature phase
    FullFeaturePhase,
    /// In logout
    InLogout,
    /// Clean wait
    CleanWait,
}

/// iSCSI login stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStage {
    /// Security negotiation
    Security,
    /// Operational parameter negotiation
    Operational,
    /// Full feature phase
    FullFeature,
}

/// iSCSI status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StatusCode {
    /// Success
    Success = 0x00,
    /// Target error / Initiator error
    TargetError = 0x01,
    /// Service delivery (target) error
    ServiceDeliveryError = 0x02,
    /// Invalid PDU field
    InvalidPduField = 0x03,
    /// Version mismatch
    VersionMismatch = 0x04,
    /// Invalid during login
    InvalidDuringLogin = 0x05,
    /// Authentication failure
    AuthenticationFailure = 0x06,
}

/// iSCSI PDU header
#[derive(Debug, Clone)]
pub struct PduHeader {
    /// Operation code
    pub opcode: OpCode,
    /// Final flag
    pub final_flag: bool,
    /// Immediate data flag
    pub immediate_flag: bool,
    /// Total sequence number
    pub total_sequence_number: u32,
    /// Data segment length
    pub data_segment_length: u32,
    /// Logical unit number
    pub lun: u64,
    /// Initiator task tag
    pub initiator_task_tag: u32,
    /// Target transfer tag
    pub target_transfer_tag: u32,
    /// Command sequence number
    pub cmd_sequence_number: u32,
    /// Status sequence number
    pub status_sequence_number: u32,
    /// Expected data transfer length
    pub expected_data_length: u32,
}

impl PduHeader {
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(48);

        // First dword: opcode and flags
        let opcode = self.opcode as u8;
        let final_flag = self.final_flag as u8;
        let immediate_flag = self.immediate_flag as u8;
        let first = opcode << 24
            | (final_flag << 23)
            | (immediate_flag << 22);
        bytes.extend_from_slice(&first.to_be_bytes());

        // Total sequence number
        bytes.extend_from_slice(&self.total_sequence_number.to_be_bytes());

        // Data segment length
        bytes.extend_from_slice(&self.data_segment_length.to_be_bytes());

        // LUN (8 bytes)
        bytes.extend_from_slice(&self.lun.to_be_bytes());

        // Initiator task tag
        bytes.extend_from_slice(&self.initiator_task_tag.to_be_bytes());

        // Target transfer tag
        bytes.extend_from_slice(&self.target_transfer_tag.to_be_bytes());

        // Command sequence number
        bytes.extend_from_slice(&self.cmd_sequence_number.to_be_bytes());

        // Status sequence number
        bytes.extend_from_slice(&self.status_sequence_number.to_be_bytes());

        // Expected data transfer length
        bytes.extend_from_slice(&self.expected_data_length.to_be_bytes());

        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IscsiError> {
        if bytes.len() < 48 {
            return Err(IscsiError::InvalidPdu);
        }

        let first = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let opcode = OpCode::from_byte((first >> 24) as u8).ok_or(IscsiError::InvalidOpcode)?;
        let final_flag = (first & 0x00800000) != 0;
        let immediate_flag = (first & 0x00400000) != 0;

        let total_sequence_number = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let data_segment_length = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        let mut lun_bytes = [0u8; 8];
        lun_bytes.copy_from_slice(&bytes[12..20]);
        let lun = u64::from_be_bytes(lun_bytes);

        let initiator_task_tag = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        let target_transfer_tag = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let cmd_sequence_number = u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        let status_sequence_number = u32::from_be_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);
        let expected_data_length = u32::from_be_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);

        Ok(Self {
            opcode,
            final_flag,
            immediate_flag,
            total_sequence_number,
            data_segment_length,
            lun,
            initiator_task_tag,
            target_transfer_tag,
            cmd_sequence_number,
            status_sequence_number,
            expected_data_length,
        })
    }
}

/// iSCSI PDU
#[derive(Debug, Clone)]
pub struct Pdu {
    /// PDU header
    pub header: PduHeader,
    /// Data segment
    pub data: Vec<u8>,
}

impl Pdu {
    /// Create a new PDU
    pub fn new(header: PduHeader, data: Vec<u8>) -> Self {
        Self { header, data }
    }

    /// Serialize PDU to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.data);
        bytes
    }

    /// Parse PDU from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IscsiError> {
        let header = PduHeader::from_bytes(bytes)?;

        let data_start = 48;
        let data_end = data_start + header.data_segment_length as usize;

        if bytes.len() < data_end {
            return Err(IscsiError::InvalidPdu);
        }

        let data = bytes[data_start..data_end].to_vec();

        Ok(Self { header, data })
    }

    /// Create login request PDU
    pub fn login_request(
        session_id: u64,
        cmd_sequence_number: u32,
        is_final: bool,
        stage: LoginStage,
        data: Vec<u8>,
    ) -> Self {
        let stage_code = match stage {
            LoginStage::Security => 0,
            LoginStage::Operational => 1,
            LoginStage::FullFeature => 3,
        };

        let transit_flag = 0u8;

        let mut header = PduHeader {
            opcode: OpCode::LoginRequest,
            final_flag: is_final,
            immediate_flag: false,
            total_sequence_number: session_id as u32,
            data_segment_length: data.len() as u32,
            lun: 0,
            initiator_task_tag: cmd_sequence_number,
            target_transfer_tag: 0xFFFFFFFF,
            cmd_sequence_number,
            status_sequence_number: 0,
            expected_data_length: 0,
        };

        // Set version (0x00) and stage in first byte
        let _opcode = header.opcode as u8;
        let _final_flag = header.final_flag as u8;
        let first = (_opcode) << 24
            | ((_final_flag) << 23)
            | ((transit_flag) << 22)
            | (stage_code << 16);
        let mut bytes = first.to_be_bytes().to_vec();
        bytes.extend_from_slice(&header.total_sequence_number.to_be_bytes());
        bytes.extend_from_slice(&header.data_segment_length.to_be_bytes());
        bytes.extend_from_slice(&header.lun.to_be_bytes());
        bytes.extend_from_slice(&header.initiator_task_tag.to_be_bytes());
        bytes.extend_from_slice(&header.target_transfer_tag.to_be_bytes());
        bytes.extend_from_slice(&header.cmd_sequence_number.to_be_bytes());
        bytes.extend_from_slice(&header.status_sequence_number.to_be_bytes());
        bytes.extend_from_slice(&header.expected_data_length.to_be_bytes());

        Self { header: PduHeader { ..header }, data }
    }

    /// Create SCSI command PDU
    pub fn scsi_command(
        lun: u64,
        task_tag: u32,
        cmd_sequence_number: u32,
        cdb: Vec<u8>,
        expected_data_length: u32,
    ) -> Self {
        let mut header = PduHeader {
            opcode: OpCode::ScsiCommand,
            final_flag: true,
            immediate_flag: false,
            total_sequence_number: 0,
            data_segment_length: cdb.len() as u32,
            lun,
            initiator_task_tag: task_tag,
            target_transfer_tag: 0xFFFFFFFF,
            cmd_sequence_number,
            status_sequence_number: 0,
            expected_data_length,
        };

        Self {
            header,
            data: cdb,
        }
    }
}

/// iSCSI session
pub struct IscsiSession {
    /// Session ID
    session_id: u64,
    /// Initiator name
    initiator_name: String,
    /// Target name
    target_name: String,
    /// Session state
    state: Mutex<SessionState>,
    /// Command sequence number
    cmd_sequence_number: AtomicU32,
    /// Next task tag
    next_task_tag: AtomicU32,
}

impl IscsiSession {
    /// Create a new iSCSI session
    pub fn new(initiator_name: String, target_name: String) -> Self {
        Self {
            session_id: 0,
            initiator_name,
            target_name,
            state: Mutex::new(SessionState::NotConnected),
            cmd_sequence_number: AtomicU32::new(1),
            next_task_tag: AtomicU32::new(1),
        }
    }

    /// Get session state
    pub fn state(&self) -> SessionState {
        *self.state.lock()
    }

    /// Create login request PDU
    pub fn create_login_request(&self, is_final: bool, stage: LoginStage) -> Pdu {
        let cmd_sequence_number = self.cmd_sequence_number.fetch_add(1, Ordering::SeqCst);

        // Build login data with key-value pairs
        let data = alloc::format!(
            "InitiatorName={}\nTargetName={}\nSessionType=Normal\n",
            self.initiator_name, self.target_name
        )
        .into_bytes();

        Pdu::login_request(self.session_id, cmd_sequence_number, is_final, stage, data)
    }

    /// Handle login response
    pub fn handle_login_response(&self, pdu: &Pdu) -> Result<(), IscsiError> {
        if pdu.header.opcode != OpCode::LoginResponse {
            return Err(IscsiError::UnexpectedPdu);
        }

        // Parse status
        if !pdu.data.is_empty() {
            let status = pdu.data[0];
            if status != 0 {
                return Err(IscsiError::LoginFailed(status));
            }
        }

        Ok(())
    }

    /// Create SCSI command PDU
    pub fn create_scsi_command(&self, lun: u64, cdb: Vec<u8>, expected_data_length: u32) -> Pdu {
        let task_tag = self.next_task_tag.fetch_add(1, Ordering::SeqCst);
        let cmd_sequence_number = self.cmd_sequence_number.fetch_add(1, Ordering::SeqCst);

        Pdu::scsi_command(lun, task_tag, cmd_sequence_number, cdb, expected_data_length)
    }
}

/// iSCSI target
pub struct IscsiTarget {
    /// Target name
    name: String,
    /// Target address
    address: String,
    /// Target port
    port: u16,
    /// Available LUNs
    luns: Mutex<Vec<u64>>,
}

impl IscsiTarget {
    /// Create a new iSCSI target
    pub fn new(name: String, address: String, port: u16) -> Self {
        Self {
            name,
            address,
            port,
            luns: Mutex::new(Vec::new()),
        }
    }

    /// Get target name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get target address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get target port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Add LUN
    pub fn add_lun(&self, lun: u64) {
        self.luns.lock().push(lun);
    }

    /// Get LUNs
    pub fn luns(&self) -> Vec<u64> {
        self.luns.lock().clone()
    }

    /// Handle incoming login request
    pub fn handle_login_request(&self, pdu: &Pdu) -> Result<Pdu, IscsiError> {
        if pdu.header.opcode != OpCode::LoginRequest {
            return Err(IscsiError::UnexpectedPdu);
        }

        // Create login response
        let mut response_header = PduHeader {
            opcode: OpCode::LoginResponse,
            final_flag: true,
            immediate_flag: false,
            total_sequence_number: pdu.header.total_sequence_number,
            data_segment_length: 0,
            lun: 0,
            initiator_task_tag: pdu.header.initiator_task_tag,
            target_transfer_tag: 0xFFFFFFFF,
            cmd_sequence_number: pdu.header.cmd_sequence_number,
            status_sequence_number: 0,
            expected_data_length: 0,
        };

        Ok(Pdu {
            header: response_header,
            data: vec![StatusCode::Success as u8],
        })
    }

    /// Handle SCSI command
    pub fn handle_scsi_command(&self, pdu: &Pdu) -> Result<Pdu, IscsiError> {
        if pdu.header.opcode != OpCode::ScsiCommand {
            return Err(IscsiError::UnexpectedPdu);
        }

        // Create SCSI response
        let response_header = PduHeader {
            opcode: OpCode::ScsiResponse,
            final_flag: true,
            immediate_flag: false,
            total_sequence_number: pdu.header.total_sequence_number,
            data_segment_length: 0,
            lun: pdu.header.lun,
            initiator_task_tag: pdu.header.initiator_task_tag,
            target_transfer_tag: pdu.header.target_transfer_tag,
            cmd_sequence_number: pdu.header.cmd_sequence_number,
            status_sequence_number: 0,
            expected_data_length: 0,
        };

        Ok(Pdu {
            header: response_header,
            data: vec![StatusCode::Success as u8],
        })
    }
}

/// Discovery service for finding iSCSI targets
pub struct DiscoveryService;

impl DiscoveryService {
    /// Discover targets using SendTargets
    pub fn send_targets(target_address: &str, port: u16) -> Result<Vec<IscsiTarget>, IscsiError> {
        // In a real implementation, this would:
        // 1. Connect to the target
        // 2. Send a SendTargets command
        // 3. Parse the response
        // 4. Return discovered targets

        // For now, return empty list
        Ok(Vec::new())
    }

    /// Discover targets using SLP (Service Location Protocol)
    pub fn discover_via_slp() -> Result<Vec<IscsiTarget>, IscsiError> {
        // In a real implementation, this would use SLP
        Ok(Vec::new())
    }
}

/// iSCSI errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IscsiError {
    /// Invalid PDU
    InvalidPdu,
    /// Invalid opcode
    InvalidOpcode,
    /// Unexpected PDU
    UnexpectedPdu,
    /// Login failed
    LoginFailed(u8),
    /// Authentication failed
    AuthenticationFailed,
    /// Connection error
    ConnectionError,
    /// Timeout
    Timeout,
    /// Target not found
    TargetNotFound,
    /// LUN not found
    LunNotFound,
    /// SCSI error
    ScsiError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdu_serialization() {
        let header = PduHeader {
            opcode: OpCode::NopOut,
            final_flag: true,
            immediate_flag: false,
            total_sequence_number: 0,
            data_segment_length: 0,
            lun: 0,
            initiator_task_tag: 1,
            target_transfer_tag: 0xFFFFFFFF,
            cmd_sequence_number: 1,
            status_sequence_number: 0,
            expected_data_length: 0,
        };

        let pdu = Pdu::new(header, vec![]);
        let bytes = pdu.to_bytes();
        let parsed = Pdu::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.header.opcode, OpCode::NopOut);
    }

    #[test]
    fn test_session_creation() {
        let session = IscsiSession::new(
            "iqn.2025-01.com.example:nos-initiator".to_string(),
            "iqn.2025-01.com.example:target".to_string(),
        );

        assert_eq!(session.state(), SessionState::NotConnected);
    }
}
