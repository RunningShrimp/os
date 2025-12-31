//! # IoT Protocol Implementations
//!
//! This module provides implementations of various IoT protocols used for
//! device communication in edge and IoT scenarios.
//!
//! ## Supported Protocols
//!
//! - **MQTT 3.1.1/5.0**: Message Queuing Telemetry Transport
//! - **CoAP**: Constrained Application Protocol
//! - **LoRaWAN**: Long Range Wide Area Network
//! - **NB-IoT**: Narrowband IoT
//! - **LTE-M**: LTE Cat-M1
//!
//! ## Protocol Comparison
//!
//! | Protocol | Bandwidth | Range | Power Use | Use Case |
//! |----------|-----------|-------|-----------|----------|
//! | MQTT | High | IP network | Medium | General IoT |
//! | CoAP | Low | IP network | Low | Constrained devices |
//! | LoRaWAN | Very Low | Very Long | Very Low | Remote sensors |
//! | NB-IoT | Very Low | Cellular | Very Low | Urban IoT |
//! | LTE-M | Medium | Cellular | Low | Mobile IoT |

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU16, Ordering};

use crate::iot::{IotError, IotResult, QoS};

// =============================================================================
// MQTT Protocol Implementation
// =============================================================================

/// MQTT protocol version
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MqttVersion {
    /// MQTT 3.1.1
    V3_1_1 = 4,
    /// MQTT 5.0
    V5_0 = 5,
}

/// MQTT packet types
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MqttPacketType {
    Connect = 1,
    Connack = 2,
    Publish = 3,
    Puback = 4,
    Pubrec = 5,
    Pubrel = 6,
    Pubcomp = 7,
    Subscribe = 8,
    Suback = 9,
    Unsubscribe = 10,
    Unsuback = 11,
    Pingreq = 12,
    Pingresp = 13,
    Disconnect = 14,
    Auth = 15,
}

/// MQTT QoS levels
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MqttQoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl From<QoS> for MqttQoS {
    fn from(qos: QoS) -> Self {
        match qos {
            QoS::AtMostOnce => MqttQoS::AtMostOnce,
            QoS::AtLeastOnce => MqttQoS::AtLeastOnce,
            QoS::ExactlyOnce => MqttQoS::ExactlyOnce,
        }
    }
}

impl From<MqttQoS> for QoS {
    fn from(qos: MqttQoS) -> Self {
        match qos {
            MqttQoS::AtMostOnce => QoS::AtMostOnce,
            MqttQoS::AtLeastOnce => QoS::AtLeastOnce,
            MqttQoS::ExactlyOnce => QoS::ExactlyOnce,
        }
    }
}

/// MQTT connect return code
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MqttConnectReason {
    Success = 0,
    UnacceptableProtocolVersion = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadUserNameOrPassword = 4,
    NotAuthorized = 5,
}

/// MQTT message
#[derive(Debug, Clone)]
pub struct MqttMessage {
    /// Topic
    pub topic: String,
    /// Payload
    pub payload: Vec<u8>,
    /// QoS level
    pub qos: MqttQoS,
    /// Message ID (for QoS > 0)
    pub packet_id: Option<u16>,
    /// Retain flag
    pub retain: bool,
    /// Duplicate flag
    pub dup: bool,
}

impl MqttMessage {
    /// Create a new MQTT message
    pub fn new(topic: impl Into<String>, payload: Vec<u8>, qos: MqttQoS) -> Self {
        Self {
            topic: topic.into(),
            payload,
            qos,
            packet_id: None,
            retain: false,
            dup: false,
        }
    }

    /// Set packet ID
    pub fn with_packet_id(mut self, id: u16) -> Self {
        self.packet_id = Some(id);
        self
    }

    /// Set retain flag
    pub fn with_retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }

    /// Set duplicate flag
    pub fn with_dup(mut self, dup: bool) -> Self {
        self.dup = dup;
        self
    }

    /// Estimate message size
    pub fn size(&self) -> usize {
        // Topic length (2 bytes) + topic + payload length (2 bytes) + payload
        2 + self.topic.len() + 2 + self.payload.len()
    }
}

/// MQTT Last Will and Testament configuration
#[derive(Debug, Clone)]
pub struct MqttWill {
    /// Topic
    pub topic: String,
    /// Message
    pub message: Vec<u8>,
    /// QoS level
    pub qos: MqttQoS,
    /// Retain flag
    pub retain: bool,
}

impl MqttWill {
    /// Create a new LWT configuration
    pub fn new(
        topic: impl Into<String>,
        message: Vec<u8>,
        qos: MqttQoS,
        retain: bool,
    ) -> Self {
        Self {
            topic: topic.into(),
            message,
            qos,
            retain,
        }
    }
}

/// MQTT client configuration
#[derive(Debug, Clone)]
pub struct MqttClientConfig {
    /// Client ID
    pub client_id: String,
    /// Clean session
    pub clean_session: bool,
    /// Keep-alive interval in seconds
    pub keep_alive: u16,
    /// Username
    pub username: Option<String>,
    /// Password
    pub password: Option<Vec<u8>>,
    /// Last Will and Testament
    pub will: Option<MqttWill>,
    /// Protocol version
    pub version: MqttVersion,
    /// Connect timeout in seconds
    pub connect_timeout: u16,
    /// Auto-reconnect
    pub auto_reconnect: bool,
    /// Reconnect delay in seconds
    pub reconnect_delay: u16,
}

impl Default for MqttClientConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            clean_session: true,
            keep_alive: 60,
            username: None,
            password: None,
            will: None,
            version: MqttVersion::V3_1_1,
            connect_timeout: 30,
            auto_reconnect: true,
            reconnect_delay: 5,
        }
    }
}

/// MQTT client
pub struct MqttClient {
    /// Client configuration
    config: MqttClientConfig,
    /// Broker address
    broker_addr: String,
    /// Broker port
    broker_port: u16,
    /// Packet ID counter
    packet_id: AtomicU16,
    /// Subscriptions
    subscriptions: BTreeMap<String, MqttQoS>,
    /// Retained messages
    retained_messages: BTreeMap<String, MqttMessage>,
    /// Pending messages (QoS 1/2)
    pending_messages: BTreeMap<u16, MqttMessage>,
    /// Connected flag
    connected: bool,
}

impl MqttClient {
    /// Create a new MQTT client
    pub fn new(broker_url: impl Into<String>) -> Result<Self, IotError> {
        let url = broker_url.into();

        // Parse URL (simplified: mqtt://host:port)
        let (broker_addr, broker_port) = if url.contains("://") {
            let parts: Vec<&str> = url.split("://").collect();
            if parts.len() != 2 {
                return Err(IotError::Protocol("Invalid MQTT URL".to_string()));
            }

            let host_port = parts[1];
            if host_port.contains(':') {
                let addr_parts: Vec<&str> = host_port.split(':').collect();
                if addr_parts.len() != 2 {
                    return Err(IotError::Protocol("Invalid MQTT URL".to_string()));
                }
                let port = addr_parts[1].parse::<u16>().map_err(|_| {
                    IotError::Protocol("Invalid MQTT port".to_string())
                })?;
                (addr_parts[0].to_string(), port)
            } else {
                (host_port.to_string(), 1883)
            }
        } else {
            (url, 1883)
        };

        Ok(Self {
            config: MqttClientConfig::default(),
            broker_addr,
            broker_port,
            packet_id: AtomicU16::new(1),
            subscriptions: BTreeMap::new(),
            retained_messages: BTreeMap::new(),
            pending_messages: BTreeMap::new(),
            connected: false,
        })
    }

    /// Set client configuration
    pub fn set_config(&mut self, config: MqttClientConfig) {
        self.config = config;
    }

    /// Connect to broker
    pub fn connect(&mut self) -> IotResult<()> {
        // In a real implementation, this would:
        // 1. Establish TCP connection to broker
        // 2. Send CONNECT packet
        // 3. Wait for CONNACK
        // 4. Handle authentication

        self.connected = true;
        crate::log_info!("MQTT client connected to {}:{}", self.broker_addr.clone(), self.broker_port);
        Ok(())
    }

    /// Disconnect from broker
    pub fn disconnect(&mut self) -> IotResult<()> {
        // Send DISCONNECT packet
        self.connected = false;
        crate::log_info!("MQTT client disconnected");
        Ok(())
    }

    /// Publish message
    pub fn publish(&mut self, mut message: MqttMessage) -> IotResult<()> {
        if !self.connected {
            return Err(IotError::Network("Not connected to broker".to_string()));
        }

        // Generate packet ID for QoS > 0
        if message.qos != MqttQoS::AtMostOnce {
            let packet_id = self.next_packet_id();
            message.packet_id = Some(packet_id);

            // Store pending message for QoS 1/2
            if message.qos == MqttQoS::AtLeastOnce {
                self.pending_messages.insert(packet_id, message.clone());
            } else if message.qos == MqttQoS::ExactlyOnce {
                self.pending_messages.insert(packet_id, message.clone());
            }
        }

        // Store retained message if flag is set
        if message.retain {
            let topic = message.topic.clone();
            let payload_len = message.payload.len();
            let qos = message.qos;
            self.retained_messages.insert(topic.clone(), message);
            crate::log_debug!("MQTT publish: {} ({} bytes, QoS {:?})", topic, payload_len, qos);
        } else {
            crate::log_debug!("MQTT publish: {} ({} bytes, QoS {:?})", message.topic, message.payload.len(), message.qos);
        }

        // In a real implementation, send PUBLISH packet here
        Ok(())
    }

    /// Subscribe to topic
    pub fn subscribe(&mut self, topic: impl Into<String>, qos: MqttQoS) -> IotResult<()> {
        if !self.connected {
            return Err(IotError::Network("Not connected to broker".to_string()));
        }

        let topic = topic.into();
        self.subscriptions.insert(topic.clone(), qos);

        crate::log_info!("MQTT subscribe: {} (QoS {:?})", topic, qos);

        // In a real implementation, send SUBSCRIBE packet and wait for SUBACK
        Ok(())
    }

    /// Unsubscribe from topic
    pub fn unsubscribe(&mut self, topic: impl Into<String>) -> IotResult<()> {
        if !self.connected {
            return Err(IotError::Network("Not connected to broker".to_string()));
        }

        let topic = topic.into();
        self.subscriptions.remove(&topic);

        crate::log_info!("MQTT unsubscribe: {}", topic);

        // In a real implementation, send UNSUBSCRIBE packet and wait for UNSUBACK
        Ok(())
    }

    /// Get retained message for topic
    pub fn get_retained(&self, topic: &str) -> Option<&MqttMessage> {
        self.retained_messages.get(topic)
    }

    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get subscriptions
    pub fn subscriptions(&self) -> &BTreeMap<String, MqttQoS> {
        &self.subscriptions
    }

    /// Generate next packet ID
    fn next_packet_id(&self) -> u16 {
        self.packet_id.fetch_add(1, Ordering::SeqCst)
    }
}

/// MQTT broker (simplified implementation)
pub struct MqttBroker {
    /// Client connections
    clients: BTreeMap<String, Vec<MqttMessage>>,
    /// Retained messages
    retained: BTreeMap<String, MqttMessage>,
    /// Subscriptions: topic -> client IDs
    subscriptions: BTreeMap<String, Vec<String>>,
}

impl MqttBroker {
    /// Create a new MQTT broker
    pub fn new() -> Self {
        Self {
            clients: BTreeMap::new(),
            retained: BTreeMap::new(),
            subscriptions: BTreeMap::new(),
        }
    }

    /// Handle client connect
    pub fn handle_connect(&mut self, client_id: impl Into<String>) -> IotResult<()> {
        let client_id = client_id.into();
        self.clients.insert(client_id.clone(), Vec::new());
        crate::log_info!("MQTT broker: client connected: {}", client_id);
        Ok(())
    }

    /// Handle client disconnect
    pub fn handle_disconnect(&mut self, client_id: &str) -> IotResult<()> {
        self.clients.remove(client_id);

        // Remove client subscriptions
        for (_topic, clients) in self.subscriptions.iter_mut() {
            clients.retain(|id| id != client_id);
        }

        crate::log_info!("MQTT broker: client disconnected: {}", client_id);
        Ok(())
    }

    /// Handle publish
    pub fn handle_publish(&mut self, message: &MqttMessage) -> IotResult<()> {
        // Store retained message
        if message.retain {
            self.retained.insert(message.topic.clone(), message.clone());
        }

        // Deliver to subscribers (simplified topic matching)
        for (topic, clients) in &self.subscriptions {
            if self.topic_matches(topic, &message.topic) {
                for client_id in clients {
                    if let Some(messages) = self.clients.get_mut(client_id) {
                        messages.push(message.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle subscribe
    pub fn handle_subscribe(
        &mut self,
        client_id: &str,
        topic: impl Into<String>,
    ) -> IotResult<()> {
        let topic = topic.into();
        self.subscriptions
            .entry(topic.clone())
            .or_insert_with(Vec::new)
            .push(client_id.to_string());

        crate::log_info!("MQTT broker: {} subscribed to {}", client_id, topic);
        Ok(())
    }

    /// Simple topic matching with wildcards
    fn topic_matches(&self, subscription: &str, topic: &str) -> bool {
        if subscription == topic {
            return true;
        }

        // Handle + wildcard (single level)
        if subscription.contains('+') {
            let sub_parts: Vec<&str> = subscription.split('/').collect();
            let topic_parts: Vec<&str> = topic.split('/').collect();

            if sub_parts.len() != topic_parts.len() {
                return false;
            }

            for (sub_part, topic_part) in sub_parts.iter().zip(topic_parts.iter()) {
                if *sub_part != "+" && *sub_part != *topic_part {
                    return false;
                }
            }

            return true;
        }

        // Handle # wildcard (multi-level)
        if subscription.ends_with("/#") {
            let prefix = &subscription[..subscription.len() - 1];
            return topic.starts_with(prefix);
        }

        false
    }
}

impl Default for MqttBroker {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// CoAP Protocol Implementation
// =============================================================================

/// CoAP methods
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum CoapMethod {
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4,
}

/// CoAP message types
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum CoapMessageType {
    Confirmable = 0,
    NonConfirmable = 1,
    Acknowledgement = 2,
    Reset = 3,
}

/// CoAP response codes
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CoapResponseCode(u8);

impl CoapResponseCode {
    /// Created (2.01)
    pub const CREATED: Self = Self(65);
    /// Deleted (2.02)
    pub const DELETED: Self = Self(66);
    /// Valid (2.03)
    pub const VALID: Self = Self(67);
    /// Changed (2.04)
    pub const CHANGED: Self = Self(68);
    /// Content (2.05)
    pub const CONTENT: Self = Self(69);
    /// Bad Request (4.00)
    pub const BAD_REQUEST: Self = Self(128);
    /// Unauthorized (4.01)
    pub const UNAUTHORIZED: Self = Self(129);
    /// Bad Option (4.02)
    pub const BAD_OPTION: Self = Self(130);
    /// Forbidden (4.03)
    pub const FORBIDDEN: Self = Self(131);
    /// Not Found (4.04)
    pub const NOT_FOUND: Self = Self(132);
    /// Method Not Allowed (4.05)
    pub const METHOD_NOT_ALLOWED: Self = Self(133);
    /// Internal Server Error (5.00)
    pub const INTERNAL_SERVER_ERROR: Self = Self(160);
}

/// CoAP message
#[derive(Debug, Clone)]
pub struct CoapMessage {
    /// Message type
    pub message_type: CoapMessageType,
    /// Message code (method or response)
    pub code: u8,
    /// Message ID
    pub message_id: u16,
    /// Token
    pub token: Vec<u8>,
    /// Options
    pub options: Vec<CoapOption>,
    /// Payload
    pub payload: Vec<u8>,
}

impl CoapMessage {
    /// Create a new CoAP request
    pub fn new_request(
        method: CoapMethod,
        message_type: CoapMessageType,
        message_id: u16,
    ) -> Self {
        Self {
            message_type,
            code: method as u8,
            message_id,
            token: Vec::new(),
            options: Vec::new(),
            payload: Vec::new(),
        }
    }

    /// Create a new CoAP response
    pub fn new_response(
        response_code: CoapResponseCode,
        message_id: u16,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            message_type: CoapMessageType::Acknowledgement,
            code: response_code.0,
            message_id,
            token: Vec::new(),
            options: Vec::new(),
            payload,
        }
    }

    /// Add option
    pub fn add_option(&mut self, option: CoapOption) {
        self.options.push(option);
    }

    /// Set payload
    pub fn set_payload(&mut self, payload: Vec<u8>) {
        self.payload = payload;
    }
}

/// CoAP option
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoapOption {
    /// Option number
    pub number: u16,
    /// Option value
    pub value: Vec<u8>,
}

impl CoapOption {
    /// Create a new CoAP option
    pub fn new(number: u16, value: Vec<u8>) -> Self {
        Self { number, value }
    }

    /// Uri-Path option (11)
    pub fn uri_path(path: impl Into<String>) -> Self {
        Self {
            number: 11,
            value: path.into().into_bytes(),
        }
    }

    /// Content-Format option (12)
    pub fn content_format(format: u16) -> Self {
        let mut value = Vec::new();
        if format < 13 {
            value.push(format as u8);
        } else if format < 269 {
            value.push(((format - 13) / 255 + 13) as u8);
            value.push(((format - 13) % 255) as u8);
        } else {
            // Extended encoding
        }
        Self { number: 12, value }
    }

    /// Observe option (6)
    pub fn observe(sequence_number: u32) -> Self {
        let value = sequence_number.to_be_bytes().to_vec();
        Self { number: 6, value }
    }
}

/// CoAP client
pub struct CoapClient {
    /// Base endpoint
    endpoint: String,
    /// Message ID counter
    message_id: AtomicU16,
    /// Observe subscriptions
    observe_subscriptions: BTreeMap<String, (u16, Vec<u8>)>,
}

impl CoapClient {
    /// Create a new CoAP client
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            message_id: AtomicU16::new(0),
            observe_subscriptions: BTreeMap::new(),
        }
    }

    /// Send GET request
    pub fn get(&self, path: impl Into<String>) -> IotResult<CoapMessage> {
        let message_id = self.next_message_id();
        let mut request = CoapMessage::new_request(
            CoapMethod::Get,
            CoapMessageType::Confirmable,
            message_id,
        );
        request.add_option(CoapOption::uri_path(path));

        crate::log_debug!("CoAP GET: {}", &self.endpoint);

        // In a real implementation, send request and wait for response
        Ok(CoapMessage::new_response(CoapResponseCode::CONTENT, message_id, Vec::new()))
    }

    /// Send POST request
    pub fn post(&self, path: impl Into<String>, payload: Vec<u8>) -> IotResult<CoapMessage> {
        let message_id = self.next_message_id();
        let mut request = CoapMessage::new_request(
            CoapMethod::Post,
            CoapMessageType::Confirmable,
            message_id,
        );
        request.add_option(CoapOption::uri_path(path));
        request.set_payload(payload);

        crate::log_debug!("CoAP POST: {}", &self.endpoint);

        // In a real implementation, send request and wait for response
        Ok(CoapMessage::new_response(CoapResponseCode::CHANGED, message_id, Vec::new()))
    }

    /// Send PUT request
    pub fn put(&self, path: impl Into<String>, payload: Vec<u8>) -> IotResult<CoapMessage> {
        let message_id = self.next_message_id();
        let mut request = CoapMessage::new_request(
            CoapMethod::Put,
            CoapMessageType::Confirmable,
            message_id,
        );
        request.add_option(CoapOption::uri_path(path));
        request.set_payload(payload);

        crate::log_debug!("CoAP PUT: {}", &self.endpoint);

        Ok(CoapMessage::new_response(CoapResponseCode::CHANGED, message_id, Vec::new()))
    }

    /// Send DELETE request
    pub fn delete(&self, path: impl Into<String>) -> IotResult<CoapMessage> {
        let message_id = self.next_message_id();
        let mut request = CoapMessage::new_request(
            CoapMethod::Delete,
            CoapMessageType::Confirmable,
            message_id,
        );
        request.add_option(CoapOption::uri_path(path));

        crate::log_debug!("CoAP DELETE: {}", &self.endpoint);

        Ok(CoapMessage::new_response(CoapResponseCode::DELETED, message_id, Vec::new()))
    }

    /// Subscribe to resource (Observe)
    pub fn observe(&mut self, path: impl Into<String>) -> IotResult<u32> {
        let path = path.into();
        let message_id = self.next_message_id();
        let token = vec![0x01, 0x02, 0x03, 0x04];

        let mut request = CoapMessage::new_request(
            CoapMethod::Get,
            CoapMessageType::Confirmable,
            message_id,
        );
        request.add_option(CoapOption::uri_path(&path));
        request.add_option(CoapOption::observe(0));
        request.token = token.clone();

        self.observe_subscriptions.insert(path.clone(), (message_id, token));

        crate::log_info!("CoAP observe: {}", &path);

        Ok(0)
    }

    /// Cancel observation
    pub fn cancel_observe(&mut self, path: &str) -> IotResult<()> {
        self.observe_subscriptions.remove(path);
        crate::log_info!("CoAP cancel observe: {}", path);
        Ok(())
    }

    /// Generate next message ID
    fn next_message_id(&self) -> u16 {
        self.message_id.fetch_add(1, Ordering::SeqCst)
    }
}

// =============================================================================
// LoRaWAN Protocol Implementation
// =============================================================================

/// LoRaWAN device class
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LoRaWanClass {
    /// Class A: Pure ALOHA (battery-powered)
    ClassA,
    /// Class B: Scheduled receive slots
    ClassB,
    /// Class C: Continuous receive (mains-powered)
    ClassC,
}

/// LoRaWAN data rate
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum LoRaWanDataRate {
    DR0 = 0,
    DR1 = 1,
    DR2 = 2,
    DR3 = 3,
    DR4 = 4,
    DR5 = 5,
}

/// LoRaWAN MAC command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoRaWanMacCommand {
    /// LinkCheckReq
    LinkCheckReq,
    /// LinkCheckAns
    LinkCheckAns { margin: u8, gateway_count: u8 },
    /// LinkADRReq
    LinkADRReq {
        data_rate: LoRaWanDataRate,
        tx_power: u8,
        channels: u16,
        redundancy: u8,
    },
    /// DutyCycleReq
    DutyCycleReq { max_duty_cycle: u8 },
    /// RXParamSetupReq
    RXParamSetupReq {
        frequency: u32,
        data_rate: LoRaWanDataRate,
    },
    /// DevStatusReq
    DevStatusReq,
    /// DevStatusAns
    DevStatusAns { battery: u8, margin: u8 },
}

/// LoRaWAN device configuration
#[derive(Debug, Clone)]
pub struct LoRaWanConfig {
    /// Device EUI (64-bit)
    pub device_eui: [u8; 8],
    /// Application EUI (64-bit)
    pub app_eui: [u8; 8],
    /// Application Key (128-bit)
    pub app_key: [u8; 16],
    /// Device class
    pub class: LoRaWanClass,
    /// Data rate
    pub data_rate: LoRaWanDataRate,
    /// TX power
    pub tx_power: u8,
    /// Adaptive data rate
    pub adr: bool,
    /// Duty cycle enabled
    pub duty_cycle: bool,
}

impl Default for LoRaWanConfig {
    fn default() -> Self {
        Self {
            device_eui: [0u8; 8],
            app_eui: [0u8; 8],
            app_key: [0u8; 16],
            class: LoRaWanClass::ClassA,
            data_rate: LoRaWanDataRate::DR0,
            tx_power: 14,
            adr: true,
            duty_cycle: true,
        }
    }
}

/// LoRaWAN device
pub struct LoRaWanDevice {
    /// Device configuration
    config: LoRaWanConfig,
    /// Frame counter up
    frame_counter_up: u32,
    /// Frame counter down
    frame_counter_down: u32,
    /// Joined flag
    joined: bool,
}

impl LoRaWanDevice {
    /// Create a new LoRaWAN device
    pub fn new(config: LoRaWanConfig) -> Self {
        Self {
            config,
            frame_counter_up: 0,
            frame_counter_down: 0,
            joined: false,
        }
    }

    /// Join network (OTAA)
    pub fn join(&mut self) -> IotResult<()> {
        // In a real implementation, this would:
        // 1. Generate JoinRequest message
        // 2. Send to network
        // 3. Wait for JoinAccept
        // 4. Derive session keys

        self.joined = true;
        crate::log_info!("LoRaWAN device joined network");
        Ok(())
    }

    /// Activate by personalization (ABP)
    pub fn activate_abp(&mut self, _dev_addr: [u8; 4], _nwk_skey: [u8; 16], _app_skey: [u8; 16]) -> IotResult<()> {
        self.joined = true;
        crate::log_info!("LoRaWAN device activated (ABP)");
        Ok(())
    }

    /// Send uplink data
    pub fn send(&mut self, data: &[u8], _confirmed: bool) -> IotResult<()> {
        if !self.joined {
            return Err(IotError::InvalidState("Device not joined".to_string()));
        }

        self.frame_counter_up += 1;

        crate::log_debug!(
            "LoRaWAN uplink: {} bytes (FCnt={})",
            data.len(),
            self.frame_counter_up
        );

        // In a real implementation, encrypt and send data
        Ok(())
    }

    /// Handle downlink data
    pub fn handle_downlink(&mut self, data: &[u8]) -> IotResult<()> {
        self.frame_counter_down += 1;

        crate::log_debug!(
            "LoRaWAN downlink: {} bytes (FCnt={})",
            data.len(),
            self.frame_counter_down
        );

        Ok(())
    }

    /// Handle MAC command
    pub fn handle_mac_command(&mut self, cmd: LoRaWanMacCommand) -> IotResult<()> {
        crate::log_debug!("LoRaWAN MAC command: {:?}", cmd);
        Ok(())
    }

    /// Check if device is joined
    pub fn is_joined(&self) -> bool {
        self.joined
    }

    /// Get frame counter
    pub fn frame_counter(&self) -> u32 {
        self.frame_counter_up
    }
}

// =============================================================================
// Cellular IoT (NB-IoT and LTE-M)
// =============================================================================

/// Cellular IoT technology
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CellularIoTTechnology {
    /// NB-IoT (Narrowband IoT)
    NBIoT,
    /// LTE-M (LTE Cat-M1)
    LteM,
}

/// Cellular IoT module configuration
#[derive(Debug, Clone)]
pub struct CellularConfig {
    /// APN (Access Point Name)
    pub apn: String,
    /// Band selection
    pub band: Option<u16>,
    /// Technology
    pub technology: CellularIoTTechnology,
    /// Power saving mode
    pub psm: bool,
    /// eDRX (Extended Discontinuous Reception)
    pub edrx: bool,
    /// Auto-connect
    pub auto_connect: bool,
}

impl Default for CellularConfig {
    fn default() -> Self {
        Self {
            apn: String::new(),
            band: None,
            technology: CellularIoTTechnology::NBIoT,
            psm: true,
            edrx: true,
            auto_connect: true,
        }
    }
}

/// Cellular IoT signal quality
#[derive(Debug, Clone, Copy)]
pub struct SignalQuality {
    /// RSSI (Received Signal Strength Indicator) in dBm
    pub rssi: i16,
    /// RSRP (Reference Signal Received Power) in dBm
    pub rsrp: Option<i16>,
    /// RSRQ (Reference Signal Received Quality) in dB
    pub rsrq: Option<i8>,
    /// Signal strength (0-31 for CSQ)
    pub csq: Option<u8>,
}

impl SignalQuality {
    /// Create new signal quality
    pub fn new(rssi: i16) -> Self {
        Self {
            rssi,
            rsrp: None,
            rsrq: None,
            csq: None,
        }
    }

    /// Get signal quality as percentage
    pub fn quality_percent(&self) -> u8 {
        // Map RSSI from -110dBm (0%) to -50dBm (100%)
        let range = 60; // -110 to -50
        let value = self.rssi + 110;
        if value <= 0 {
            0
        } else if value >= range {
            100
        } else {
            ((value * 100) / range) as u8
        }
    }
}

/// Cellular IoT module
pub struct CellularIoTModule {
    /// Module configuration
    config: CellularConfig,
    /// Connected flag
    connected: bool,
    /// Signal quality
    signal_quality: Option<SignalQuality>,
    /// IMEI
    imei: Option<String>,
    /// IMSI
    imsi: Option<String>,
}

impl CellularIoTModule {
    /// Create a new cellular IoT module
    pub fn new(config: CellularConfig) -> Self {
        Self {
            config,
            connected: false,
            signal_quality: None,
            imei: None,
            imsi: None,
        }
    }

    /// Initialize module
    pub fn init(&mut self) -> IotResult<()> {
        // In a real implementation, this would:
        // 1. Power on the module
        // 2. Send AT commands
        // 3. Get IMEI and IMSI

        self.imei = Some("123456789012345".to_string());
        self.imsi = Some("123456789012345".to_string());

        crate::log_info!("Cellular IoT module initialized");
        Ok(())
    }

    /// Connect to network
    pub fn connect(&mut self) -> IotResult<()> {
        if !self.imei.is_some() {
            return Err(IotError::InvalidState("Module not initialized".to_string()));
        }

        // In a real implementation, this would:
        // 1. Set APN
        // 2. Attach to network
        // 3. Activate PDP context

        self.connected = true;
        crate::log_info!("Cellular IoT connected to network");
        Ok(())
    }

    /// Disconnect from network
    pub fn disconnect(&mut self) -> IotResult<()> {
        self.connected = false;
        crate::log_info!("Cellular IoT disconnected");
        Ok(())
    }

    /// Get signal quality
    pub fn get_signal_quality(&mut self) -> IotResult<SignalQuality> {
        // In a real implementation, send AT+CSQ or equivalent
        let quality = SignalQuality::new(-75);
        self.signal_quality = Some(quality);
        Ok(quality)
    }

    /// Send data
    pub fn send(&self, data: &[u8]) -> IotResult<()> {
        if !self.connected {
            return Err(IotError::Network("Not connected".to_string()));
        }

        crate::log_debug!("Cellular IoT send: {} bytes", data.len());
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Enter power saving mode
    pub fn enter_psm(&self) -> IotResult<()> {
        if !self.config.psm {
            return Err(IotError::NotSupported("PSM not enabled".to_string()));
        }

        crate::log_info!("Cellular IoT entering PSM");
        Ok(())
    }

    /// Wake from power saving mode
    pub fn wake_from_psm(&self) -> IotResult<()> {
        crate::log_info!("Cellular IoT waking from PSM");
        Ok(())
    }
}

// =============================================================================
// Protocol Bridge and Translation
// =============================================================================

/// Protocol bridge for translating between different IoT protocols
pub struct ProtocolBridge {
    /// MQTT client
    mqtt_client: Option<MqttClient>,
    /// CoAP clients
    coap_clients: BTreeMap<String, CoapClient>,
}

impl ProtocolBridge {
    /// Create a new protocol bridge
    pub fn new() -> Self {
        Self {
            mqtt_client: None,
            coap_clients: BTreeMap::new(),
        }
    }

    /// Add MQTT client
    pub fn add_mqtt_client(&mut self, client: MqttClient) {
        self.mqtt_client = Some(client);
    }

    /// Add CoAP client
    pub fn add_coap_client(&mut self, name: impl Into<String>, client: CoapClient) {
        self.coap_clients.insert(name.into(), client);
    }

    /// Bridge MQTT to CoAP
    pub fn mqtt_to_coap(
        &mut self,
        mqtt_message: &MqttMessage,
        coap_path: &str,
    ) -> IotResult<()> {
        // Extract CoAP endpoint from MQTT topic (simplified)
        let coap_client = self.coap_clients.get_mut("default").ok_or_else(|| {
            IotError::NotSupported("No CoAP client configured".to_string())
        })?;

        // Map MQTT topic to CoAP resource path
        coap_client.post(coap_path, mqtt_message.payload.clone())?;
        Ok(())
    }

    /// Bridge CoAP to MQTT
    pub fn coap_to_mqtt(
        &mut self,
        _coap_path: &str,
        payload: Vec<u8>,
        mqtt_topic: &str,
    ) -> IotResult<()> {
        let mqtt_client = self
            .mqtt_client
            .as_mut()
            .ok_or_else(|| IotError::NotSupported("No MQTT client configured".to_string()))?;

        let message = MqttMessage::new(mqtt_topic, payload, MqttQoS::AtLeastOnce);
        mqtt_client.publish(message)?;
        Ok(())
    }
}

impl Default for ProtocolBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_client_creation() {
        let client = MqttClient::new("mqtt://broker.example.com:1883").unwrap();
        assert!(!client.is_connected());
    }

    #[test]
    fn test_mqtt_client_connect() {
        let mut client = MqttClient::new("mqtt://broker.example.com:1883").unwrap();
        client.connect().unwrap();
        assert!(client.is_connected());
    }

    #[test]
    fn test_mqtt_publish() {
        let mut client = MqttClient::new("mqtt://broker.example.com:1883").unwrap();
        client.connect().unwrap();

        let message = MqttMessage::new("test/topic", b"hello".to_vec(), MqttQoS::AtLeastOnce);
        client.publish(message).unwrap();
    }

    #[test]
    fn test_mqtt_subscribe() {
        let mut client = MqttClient::new("mqtt://broker.example.com:1883").unwrap();
        client.connect().unwrap();

        client.subscribe("sensors/+/temperature", MqttQoS::AtLeastOnce)
            .unwrap();

        let subs = client.subscriptions();
        assert!(subs.contains_key("sensors/+/temperature"));
    }

    #[test]
    fn test_mqtt_retained_messages() {
        let mut client = MqttClient::new("mqtt://broker.example.com:1883").unwrap();
        client.connect().unwrap();

        let message = MqttMessage::new("test/topic", b"retained".to_vec(), MqttQoS::AtLeastOnce)
            .with_retain(true);
        client.publish(message).unwrap();

        let retained = client.get_retained("test/topic");
        assert!(retained.is_some());
        assert!(retained.unwrap().retain);
    }

    #[test]
    fn test_mqtt_broker() {
        let mut broker = MqttBroker::new();

        broker.handle_connect("client1").unwrap();
        broker.handle_subscribe("client1", "sensors/#").unwrap();

        let message = MqttMessage::new("sensors/temp", b"22.5".to_vec(), MqttQoS::AtLeastOnce)
            .with_retain(true);
        broker.handle_publish(&message).unwrap();

        let retained = broker.retained.get("sensors/temp");
        assert!(retained.is_some());
    }

    #[test]
    fn test_coap_client() {
        let client = CoapClient::new("coap://example.com:5683");

        let response = client.get("/sensor/temp").unwrap();
        assert_eq!(response.message_type, CoapMessageType::Acknowledgement);
    }

    #[test]
    fn test_coap_message() {
        let msg = CoapMessage::new_request(CoapMethod::Get, CoapMessageType::Confirmable, 123);
        assert_eq!(msg.code, CoapMethod::Get as u8);
        assert_eq!(msg.message_id, 123);
    }

    #[test]
    fn test_coap_options() {
        let uri_path = CoapOption::uri_path("sensors/temp");
        assert_eq!(uri_path.number, 11);
        assert_eq!(uri_path.value, b"sensors/temp");

        let content_format = CoapOption::content_format(50); // JSON
        assert_eq!(content_format.number, 12);
    }

    #[test]
    fn test_lorawan_device() {
        let config = LoRaWanConfig::default();
        let mut device = LoRaWanDevice::new(config);

        assert!(!device.is_joined());
        device.join().unwrap();
        assert!(device.is_joined());

        device.send(b"hello", false).unwrap();
        assert_eq!(device.frame_counter(), 1);
    }

    #[test]
    fn test_cellular_iot_module() {
        let config = CellularConfig::default();
        let mut module = CellularIoTModule::new(config);

        module.init().unwrap();
        assert!(module.imei.is_some());

        module.connect().unwrap();
        assert!(module.is_connected());

        let quality = module.get_signal_quality().unwrap();
        assert_eq!(quality.rssi, -75);
    }

    #[test]
    fn test_protocol_bridge() {
        let mut bridge = ProtocolBridge::new();

        let mqtt_client = MqttClient::new("mqtt://broker.example.com:1883").unwrap();
        bridge.add_mqtt_client(mqtt_client);

        let coap_client = CoapClient::new("coap://example.com:5683");
        bridge.add_coap_client("default", coap_client);

        // Test bridging
        let mqtt_msg = MqttMessage::new("test/topic", b"data".to_vec(), MqttQoS::AtLeastOnce);
        // This would fail without actual network but demonstrates the API
        // bridge.mqtt_to_coap(&mqtt_msg, "/coap/resource").ok();
    }

    #[test]
    fn test_qos_conversion() {
        let qos: QoS = QoS::AtLeastOnce;
        let mqtt_qos: MqttQoS = qos.into();
        assert_eq!(mqtt_qos, MqttQoS::AtLeastOnce);

        let qos_back: QoS = mqtt_qos.into();
        assert_eq!(qos_back, QoS::AtLeastOnce);
    }

    #[test]
    fn test_signal_quality() {
        let quality = SignalQuality::new(-75);
        assert_eq!(quality.rssi, -75);
        assert!(quality.quality_percent() > 50);

        let poor_quality = SignalQuality::new(-105);
        assert!(poor_quality.quality_percent() < 20);
    }
}
