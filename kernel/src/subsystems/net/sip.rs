//! SIP (Session Initiation Protocol) Protocol Stack
//!
//! This module implements the SIP protocol for VoIP (Voice over IP) signaling,
//! including registration, call setup, teardown, and messaging.

#![allow(dead_code)]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::subsystems::sync::{Mutex, RwLock};

use super::socket::SocketAddr;

/// SIP user agent
pub struct SipUserAgent {
    /// User agent ID
    id: u32,
    /// SIP address (AOR)
    address: SipAddress,
    /// User agent configuration
    config: SipConfig,
    /// Registration state
    registration: Mutex<RegistrationState>,
    /// Active dialogs
    dialogs: RwLock<BTreeMap<String, Arc<SipDialog>>>,
    /// Transaction manager
    transactions: Arc<TransactionManager>,
    /// Transport manager
    transport: Arc<SipTransport>,
    /// Next CSeq
    next_cseq: AtomicU32,
    /// Call-ID counter
    call_id_counter: AtomicU32,
    /// Tag counter
    tag_counter: AtomicU32,
}

/// SIP address (AOR - Address of Record)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SipAddress {
    /// Display name
    pub display_name: Option<String>,
    /// URI scheme (sip: or sips:)
    pub scheme: String,
    /// Username
    pub username: String,
    /// Host
    pub host: String,
    /// Port
    pub port: Option<u16>,
    /// URI parameters
    pub parameters: Vec<(String, String)>,
}

impl SipAddress {
    /// Create a new SIP address
    pub fn new(username: String, host: String) -> Self {
        Self {
            display_name: None,
            scheme: "sip".to_string(),
            username,
            host,
            port: None,
            parameters: Vec::new(),
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Result<Self, SipError> {
        // Simplified parsing: "sip:user@host" or "Display Name <sip:user@host>"
        let s = s.trim();

        let (display_name, uri_part) = if s.contains('<') {
            let parts: Vec<&str> = s.splitn(2, '<').collect();
            let display = Some(parts[0].trim().trim_matches('"').to_string());
            let uri = parts[1].trim().trim_end_matches('>');
            (display, uri)
        } else {
            (None, s)
        };

        let uri_part = uri_part.trim_start_matches("sip:").trim_start_matches("sips:");

        let at_pos = uri_part.find('@')
            .ok_or(SipError::InvalidAddress)?;

        let username = uri_part[..at_pos].to_string();
        let host_port = &uri_part[at_pos + 1..];

        let (host, port) = if let Some(colon_pos) = host_port.find(':') {
            let host = host_port[..colon_pos].to_string();
            let port = host_port[colon_pos + 1..].parse::<u16>()
                .ok();
            (host, port)
        } else {
            (host_port.to_string(), None)
        };

        Ok(Self {
            display_name,
            scheme: "sip".to_string(),
            username,
            host,
            port,
            parameters: Vec::new(),
        })
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        let mut result = String::new();

        if let Some(ref display) = self.display_name {
            result.push_str(display);
            result.push(' ');
        }

        result.push('<');
        result.push_str(&self.scheme);
        result.push(':');
        result.push_str(&self.username);
        result.push('@');
        result.push_str(&self.host);

        if let Some(port) = self.port {
            result.push(':');
            result.push_str(&port.to_string());
        }

        result.push('>');

        result
    }
}

/// SIP configuration
#[derive(Debug, Clone)]
pub struct SipConfig {
    /// Default transport protocol
    pub transport: SipTransportType,
    /// Default outbound proxy
    pub outbound_proxy: Option<String>,
    /// Registration expiration (seconds)
    pub registration_expires: u32,
    /// Registration retry interval (seconds)
    pub registration_retry: u32,
    /// Max forwards
    pub max_forwards: u32,
    /// Session timer (seconds)
    pub session_timer: u32,
    /// Min session timer (seconds)
    pub session_timer_min: u32,
    /// User agent string
    pub user_agent: String,
    /// Allow multiple registrations
    pub allow_multiple_registrations: bool,
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            transport: SipTransportType::Udp,
            outbound_proxy: None,
            registration_expires: 3600,
            registration_retry: 30,
            max_forwards: 70,
            session_timer: 1800,
            session_timer_min: 90,
            user_agent: "NOS-SIP/1.0".to_string(),
            allow_multiple_registrations: true,
        }
    }
}

/// SIP transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SipTransportType {
    /// UDP transport
    Udp,
    /// TCP transport
    Tcp,
    /// TLS transport
    Tls,
    /// WS transport
    Ws,
    /// WSS transport
    Wss,
}

/// Registration state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationState {
    /// Not registered
    NotRegistered,
    /// Registration in progress
    Registering,
    /// Registered
    Registered,
    /// Registration failed
    Failed,
    /// Unregistered
    Unregistered,
}

/// SIP dialog (call leg)
pub struct SipDialog {
    /// Call-ID
    call_id: String,
    /// Local tag
    local_tag: String,
    /// Remote tag
    remote_tag: Option<String>,
    /// Local URI
    local_uri: SipAddress,
    /// Remote URI
    remote_uri: SipAddress,
    /// Remote target
    remote_target: SipAddress,
    /// Dialog state
    state: DialogState,
    /// Route set
    route_set: Vec<SipAddress>,
    /// Local CSeq
    local_cseq: u32,
    /// Remote CSeq
    remote_cseq: u32,
}

/// Dialog state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogState {
    /// Early dialog (provisional response)
    Early,
    /// Confirmed dialog
    Confirmed,
    /// Terminated dialog
    Terminated,
}

/// SIP transaction
#[derive(Clone)]
pub struct SipTransaction {
    /// Transaction ID
    id: String,
    /// Transaction type
    transaction_type: TransactionType,
    /// Original request
    request: SipMessage,
    /// Last response
    last_response: Option<SipMessage>,
    /// Transaction state
    state: TransactionState,
    /// Retries
    retries: u32,
    /// Timeout (ms)
    timeout: u64,
}

/// Transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    /// Client transaction
    Client,
    /// Server transaction
    Server,
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// Calling
    Calling,
    /// Trying
    Trying,
    /// Proceeding
    Proceeding,
    /// Completed
    Completed,
    /// Confirmed
    Confirmed,
    /// Terminated
    Terminated,
}

/// SIP transaction manager
pub struct TransactionManager {
    /// Client transactions
    client_transactions: Mutex<BTreeMap<String, SipTransaction>>,
    /// Server transactions
    server_transactions: Mutex<BTreeMap<String, SipTransaction>>,
    /// Next transaction ID
    next_id: AtomicU32,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new() -> Self {
        Self {
            client_transactions: Mutex::new(BTreeMap::new()),
            server_transactions: Mutex::new(BTreeMap::new()),
            next_id: AtomicU32::new(1),
        }
    }

    /// Create client transaction
    pub fn create_client_transaction(&self, request: &SipMessage) -> String {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst).to_string();
        let tx = SipTransaction {
            id: id.clone(),
            transaction_type: TransactionType::Client,
            request: request.clone(),
            last_response: None,
            state: TransactionState::Calling,
            retries: 0,
            timeout: 5000, // 5 seconds default
        };

        self.client_transactions.lock().insert(id.clone(), tx);
        id
    }

    /// Get client transaction
    pub fn get_client_transaction(&self, id: &str) -> Option<SipTransaction> {
        self.client_transactions.lock().get(id).cloned()
    }

    /// Remove client transaction
    pub fn remove_client_transaction(&self, id: &str) {
        self.client_transactions.lock().remove(id);
    }
}

/// SIP transport
pub struct SipTransport {
    /// Local socket address
    local_addr: SocketAddr,
    /// Transport type
    transport_type: SipTransportType,
}

impl SipTransport {
    /// Create a new SIP transport
    pub fn new(local_addr: SocketAddr, transport_type: SipTransportType) -> Self {
        Self {
            local_addr,
            transport_type,
        }
    }

    /// Send SIP message
    pub fn send_message(&self, _message: &SipMessage, _dest: &SocketAddr)
        -> Result<(), SipError>
    {
        // Implementation would send via UDP/TCP socket
        Ok(())
    }

    /// Receive SIP message
    pub fn receive_message(&self) -> Result<SipMessage, SipError> {
        // Implementation would receive from UDP/TCP socket
        Err(SipError::Timeout)
    }
}

/// SIP message
#[derive(Debug, Clone)]
pub struct SipMessage {
    /// Message type
    pub message_type: SipMessageType,
    /// Request line (for requests) or Status line (for responses)
    pub start_line: String,
    /// Headers
    pub headers: BTreeMap<String, String>,
    /// Body
    pub body: Option<Vec<u8>>,
}

/// SIP message type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SipMessageType {
    /// Request
    Request(SipMethod),
    /// Response
    Response(u16, String),
}

/// SIP method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SipMethod {
    /// INVITE - initiate a call
    Invite,
    /// ACK - acknowledge INVITE response
    Ack,
    /// BYE - terminate a call
    Bye,
    /// CANCEL - cancel pending request
    Cancel,
    /// REGISTER - register with registrar
    Register,
    /// OPTIONS - query capabilities
    Options,
    /// INFO - send mid-call information
    Info,
    /// MESSAGE - send instant message
    Message,
    /// SUBSCRIBE - subscribe to event
    Subscribe,
    /// NOTIFY - notify event
    Notify,
    /// REFER - refer call to third party
    Refer,
    /// UPDATE - update session parameters
    Update,
    /// PRACK - provisional response acknowledgement
    Prack,
}

impl SipMethod {
    /// Parse method from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "INVITE" => Some(Self::Invite),
            "ACK" => Some(Self::Ack),
            "BYE" => Some(Self::Bye),
            "CANCEL" => Some(Self::Cancel),
            "REGISTER" => Some(Self::Register),
            "OPTIONS" => Some(Self::Options),
            "INFO" => Some(Self::Info),
            "MESSAGE" => Some(Self::Message),
            "SUBSCRIBE" => Some(Self::Subscribe),
            "NOTIFY" => Some(Self::Notify),
            "REFER" => Some(Self::Refer),
            "UPDATE" => Some(Self::Update),
            "PRACK" => Some(Self::Prack),
            _ => None,
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        match self {
            Self::Invite => "INVITE".to_string(),
            Self::Ack => "ACK".to_string(),
            Self::Bye => "BYE".to_string(),
            Self::Cancel => "CANCEL".to_string(),
            Self::Register => "REGISTER".to_string(),
            Self::Options => "OPTIONS".to_string(),
            Self::Info => "INFO".to_string(),
            Self::Message => "MESSAGE".to_string(),
            Self::Subscribe => "SUBSCRIBE".to_string(),
            Self::Notify => "NOTIFY".to_string(),
            Self::Refer => "REFER".to_string(),
            Self::Update => "UPDATE".to_string(),
            Self::Prack => "PRACK".to_string(),
        }
    }
}

impl SipMessage {
    /// Create a new SIP request
    pub fn new_request(method: SipMethod, request_uri: SipAddress) -> Self {
        let start_line = format!("{} {} SIP/2.0",
            method.to_string(),
            request_uri.to_string());

        Self {
            message_type: SipMessageType::Request(method),
            start_line,
            headers: BTreeMap::new(),
            body: None,
        }
    }

    /// Create a new SIP response
    pub fn new_response(status_code: u16, reason_phrase: String) -> Self {
        let start_line = format!("SIP/2.0 {} {}", status_code, reason_phrase);

        Self {
            message_type: SipMessageType::Response(status_code, reason_phrase),
            start_line,
            headers: BTreeMap::new(),
            body: None,
        }
    }

    /// Add header
    pub fn add_header(&mut self, name: String, value: String) {
        self.headers.insert(name, value);
    }

    /// Get header
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    /// Set body
    pub fn set_body(&mut self, body: Vec<u8>, content_type: String) {
        let len = body.len();
        self.body = Some(body);
        self.add_header("Content-Type".to_string(), content_type);
        self.add_header("Content-Length".to_string(),
            len.to_string());
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = String::new();

        result.push_str(&self.start_line);
        result.push_str("\r\n");

        for (name, value) in &self.headers {
            result.push_str(name);
            result.push_str(": ");
            result.push_str(value);
            result.push_str("\r\n");
        }

        result.push_str("\r\n");

        let mut bytes = result.into_bytes();
        if let Some(ref body) = self.body {
            bytes.extend_from_slice(body);
        }

        bytes
    }

    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self, SipError> {
        let data = core::str::from_utf8(data)
            .map_err(|_| SipError::InvalidMessage)?;

        let mut lines = data.lines();

        let start_line = lines.next()
            .ok_or(SipError::InvalidMessage)?;

        let mut headers = BTreeMap::new();

        for line in &mut lines {
            if line.is_empty() {
                break;
            }

            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(name, value);
            }
        }

        // Determine message type
        let message_type = if start_line.starts_with("SIP/2.0") {
            // Response
            let parts: Vec<&str> = start_line.split_whitespace().collect();
            if parts.len() >= 3 {
                let status_code = parts[1].parse::<u16>()
                    .unwrap_or(500);
                let reason = parts[2..].join(" ");
                SipMessageType::Response(status_code, reason)
            } else {
                return Err(SipError::InvalidMessage);
            }
        } else {
            // Request
            let parts: Vec<&str> = start_line.split_whitespace().collect();
            if parts.len() >= 3 {
                let method = SipMethod::from_str(parts[0])
                    .ok_or(SipError::InvalidMessage)?;
                SipMessageType::Request(method)
            } else {
                return Err(SipError::InvalidMessage);
            }
        };

        Ok(Self {
            message_type,
            start_line: start_line.to_string(),
            headers,
            body: None, // Would parse body if present
        })
    }
}

impl SipUserAgent {
    /// Create a new SIP user agent
    pub fn new(id: u32, address: SipAddress, config: SipConfig,
               local_addr: SocketAddr) -> Self
    {
        let transport = Arc::new(SipTransport::new(local_addr, config.transport));

        Self {
            id,
            address,
            config,
            registration: Mutex::new(RegistrationState::NotRegistered),
            dialogs: RwLock::new(BTreeMap::new()),
            transactions: Arc::new(TransactionManager::new()),
            transport,
            next_cseq: AtomicU32::new(1),
            call_id_counter: AtomicU32::new(1),
            tag_counter: AtomicU32::new(1),
        }
    }

    /// Register with registrar
    pub fn register(&self, registrar: &SipAddress)
        -> Result<String, SipError>
    {
        *self.registration.lock() = RegistrationState::Registering;

        let mut request = SipMessage::new_request(SipMethod::Register, registrar.clone());

        // Add From header
        let from_value = self.address.to_string();
        request.add_header("From".to_string(),
            format!("{};tag={}", from_value, self.generate_tag()));

        // Add To header
        request.add_header("To".to_string(), from_value);

        // Add Call-ID
        let call_id = self.generate_call_id();
        request.add_header("Call-ID".to_string(), call_id.clone());

        // Add CSeq
        let cseq = self.next_cseq.fetch_add(1, Ordering::SeqCst);
        request.add_header("CSeq".to_string(),
            format!("{} REGISTER", cseq));

        // Add Contact
        let contact = format!("{};expires={}",
            self.address.to_string(),
            self.config.registration_expires);
        request.add_header("Contact".to_string(), contact);

        // Add Max-Forwards
        request.add_header("Max-Forwards".to_string(),
            self.config.max_forwards.to_string());

        // Add User-Agent
        request.add_header("User-Agent".to_string(),
            self.config.user_agent.clone());

        // Add Allow header
        request.add_header("Allow".to_string(),
            "INVITE, ACK, CANCEL, BYE, OPTIONS, INFO, MESSAGE, SUBSCRIBE, NOTIFY".to_string());

        // Send request
        let dest = self.resolve_address(registrar)?;
        self.transport.send_message(&request, &dest)?;

        Ok(call_id)
    }

    /// Unregister
    pub fn unregister(&self) -> Result<(), SipError> {
        *self.registration.lock() = RegistrationState::Unregistered;
        Ok(())
    }

    /// Initiate a call
    pub fn invite(&self, callee: &SipAddress, sdp: Vec<u8>)
        -> Result<String, SipError>
    {
        let mut request = SipMessage::new_request(SipMethod::Invite, callee.clone());

        // Add From header
        let from_value = self.address.to_string();
        request.add_header("From".to_string(),
            format!("{};tag={}", from_value, self.generate_tag()));

        // Add To header
        request.add_header("To".to_string(), callee.to_string());

        // Add Call-ID
        let call_id = self.generate_call_id();
        request.add_header("Call-ID".to_string(), call_id.clone());

        // Add CSeq
        let cseq = self.next_cseq.fetch_add(1, Ordering::SeqCst);
        request.add_header("CSeq".to_string(),
            format!("{} INVITE", cseq));

        // Add Contact
        request.add_header("Contact".to_string(),
            self.address.to_string());

        // Add Content-Type
        request.add_header("Content-Type".to_string(),
            "application/sdp".to_string());

        // Add SDP body
        request.set_body(sdp, "application/sdp".to_string());

        // Send request
        let dest = self.resolve_address(callee)?;
        self.transport.send_message(&request, &dest)?;

        Ok(call_id)
    }

    /// Accept a call
    pub fn accept_call(&self, call_id: &str, sdp: Vec<u8>)
        -> Result<(), SipError>
    {
        let dialogs = self.dialogs.read();
        let dialog = dialogs.get(call_id)
            .ok_or(SipError::DialogNotFound)?;

        let mut response = SipMessage::new_response(200, "OK".to_string());

        // Add To tag
        response.add_header("To".to_string(),
            format!("{};tag={}",
                dialog.remote_uri.to_string(),
                dialog.local_tag.clone()));

        // Add Call-ID
        response.add_header("Call-ID".to_string(), call_id.to_string());

        // Add CSeq
        response.add_header("CSeq".to_string(),
            format!("{} INVITE", dialog.remote_cseq));

        // Add Contact
        response.add_header("Contact".to_string(),
            dialog.local_uri.to_string());

        // Add SDP body
        response.set_body(sdp, "application/sdp".to_string());

        // Send response
        let dest = self.resolve_address(&dialog.remote_target)?;
        self.transport.send_message(&response, &dest)?;

        Ok(())
    }

    /// Reject a call
    pub fn reject_call(&self, call_id: &str, reason: u16) -> Result<(), SipError> {
        let dialogs = self.dialogs.read();
        let dialog = dialogs.get(call_id)
            .ok_or(SipError::DialogNotFound)?;

        let reason_phrase = match reason {
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            486 => "Busy Here",
            487 => "Request Terminated",
            600 => "Busy Everywhere",
            603 => "Decline",
            _ => "Unknown",
        }.to_string();

        let response = SipMessage::new_response(reason, reason_phrase);

        // Send response
        let dest = self.resolve_address(&dialog.remote_target)?;
        self.transport.send_message(&response, &dest)?;

        Ok(())
    }

    /// Terminate a call
    pub fn bye(&self, call_id: &str) -> Result<(), SipError> {
        let dialogs = self.dialogs.read();
        let dialog = dialogs.get(call_id)
            .ok_or(SipError::DialogNotFound)?;

        let mut request = SipMessage::new_request(SipMethod::Bye, dialog.remote_target.clone());

        // Add From header
        request.add_header("From".to_string(),
            format!("{};tag={}",
                dialog.local_uri.to_string(),
                dialog.local_tag));

        // Add To header
        request.add_header("To".to_string(),
            format!("{};tag={}",
                dialog.remote_uri.to_string(),
                dialog.remote_tag.as_ref().unwrap_or(&String::new())));

        // Add Call-ID
        request.add_header("Call-ID".to_string(), call_id.to_string());

        // Add CSeq
        let cseq = dialog.local_cseq + 1;
        request.add_header("CSeq".to_string(),
            format!("{} BYE", cseq));

        // Send request
        let dest = self.resolve_address(&dialog.remote_target)?;
        self.transport.send_message(&request, &dest)?;

        Ok(())
    }

    /// Send instant message
    pub fn send_message(&self, to: &SipAddress, content: String)
        -> Result<String, SipError>
    {
        let mut request = SipMessage::new_request(SipMethod::Message, to.clone());

        // Add From header
        request.add_header("From".to_string(),
            format!("{};tag={}",
                self.address.to_string(),
                self.generate_tag()));

        // Add To header
        request.add_header("To".to_string(), to.to_string());

        // Add Call-ID
        let call_id = self.generate_call_id();
        request.add_header("Call-ID".to_string(), call_id.clone());

        // Add CSeq
        let cseq = self.next_cseq.fetch_add(1, Ordering::SeqCst);
        request.add_header("CSeq".to_string(),
            format!("{} MESSAGE", cseq));

        // Add Content-Type
        request.add_header("Content-Type".to_string(),
            "text/plain".to_string());

        // Add message body
        request.set_body(content.into_bytes(), "text/plain".to_string());

        // Send request
        let dest = self.resolve_address(to)?;
        self.transport.send_message(&request, &dest)?;

        Ok(call_id)
    }

    /// Generate unique tag
    fn generate_tag(&self) -> String {
        let id = self.tag_counter.fetch_add(1, Ordering::SeqCst);
        format!("{:x}", id)
    }

    /// Generate Call-ID
    fn generate_call_id(&self) -> String {
        let id = self.call_id_counter.fetch_add(1, Ordering::SeqCst);
        format!("{}@{}", id, self.address.host)
    }

    /// Resolve address to socket address
    fn resolve_address(&self, addr: &SipAddress) -> Result<SocketAddr, SipError> {
        // Simplified: assume host is IP address
        use super::ipv4::Ipv4Addr;

        let port = addr.port.unwrap_or(5060);

        // Parse IP address (simplified)
        let ip = if addr.host == "localhost" || addr.host == "127.0.0.1" {
            Ipv4Addr::new(127, 0, 0, 1)
        } else {
            // Would need DNS resolution
            return Err(SipError::DnsFailure);
        };

        Ok(SocketAddr::new_ipv4(ip, port))
    }
}

/// SIP error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SipError {
    /// Invalid address
    InvalidAddress,
    /// Invalid message
    InvalidMessage,
    /// Dialog not found
    DialogNotFound,
    /// Transaction failed
    TransactionFailed,
    /// Transport error
    TransportError,
    /// Timeout
    Timeout,
    /// DNS failure
    DnsFailure,
    /// Authentication required
    AuthenticationRequired,
    /// Unsupported method
    UnsupportedMethod,
}

impl core::fmt::Display for SipError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidAddress => write!(f, "Invalid SIP address"),
            Self::InvalidMessage => write!(f, "Invalid SIP message"),
            Self::DialogNotFound => write!(f, "Dialog not found"),
            Self::TransactionFailed => write!(f, "Transaction failed"),
            Self::TransportError => write!(f, "Transport error"),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::DnsFailure => write!(f, "DNS resolution failed"),
            Self::AuthenticationRequired => write!(f, "Authentication required"),
            Self::UnsupportedMethod => write!(f, "Unsupported SIP method"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sip_address_parse() {
        let addr = SipAddress::parse("sip:user@example.com").unwrap();
        assert_eq!(addr.username, "user");
        assert_eq!(addr.host, "example.com");
    }

    #[test]
    fn test_sip_message() {
        let request = SipMessage::new_request(
            SipMethod::Invite,
            SipAddress::new("alice".to_string(), "example.com".to_string())
        );

        assert!(matches!(request.message_type,
            SipMessageType::Request(SipMethod::Invite)));
    }

    #[test]
    fn test_sip_address_to_string() {
        let addr = SipAddress::new("bob".to_string(), "example.com".to_string());
        let s = addr.to_string();
        assert!(s.contains("bob"));
        assert!(s.contains("example.com"));
    }
}
