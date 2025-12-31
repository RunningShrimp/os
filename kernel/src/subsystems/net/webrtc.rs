//! WebRTC (Web Real-Time Communication) Protocol Stack
//!
//! This module implements the WebRTC protocol stack for real-time communication,
//! including peer connection, media streams, data channels, and ICE/DTLS/SRTP integration.

#![allow(dead_code)]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::subsystems::sync::{Mutex, RwLock};

use super::{
    ice::{IceAgent, IceCandidate, IceCredentials},
    srtp::SrtpSession,
    rtp::{RtpPacket, RtpSession},
};

/// WebRTC configuration
#[derive(Debug, Clone)]
pub struct WebRtcConfig {
    /// ICE servers (STUN/TURN)
    pub ice_servers: Vec<IceServer>,
    /// ICE transport policy
    pub ice_transport_policy: IceTransportPolicy,
    /// Bundle policy
    pub bundle_policy: BundlePolicy,
    /// RTCP mux policy
    pub rtcp_mux_policy: RtcpMuxPolicy,
    /// Maximum video bitrate (bps)
    pub max_video_bitrate: u32,
    /// Maximum audio bitrate (bps)
    pub max_audio_bitrate: u32,
    /// Enable IPv6
    pub enable_ipv6: bool,
    /// Enable data channels
    pub enable_data_channels: bool,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            ice_servers: Vec::new(),
            ice_transport_policy: IceTransportPolicy::All,
            bundle_policy: BundlePolicy::Balanced,
            rtcp_mux_policy: RtcpMuxPolicy::Require,
            max_video_bitrate: 2_000_000,  // 2 Mbps
            max_audio_bitrate: 128_000,    // 128 Kbps
            enable_ipv6: true,
            enable_data_channels: true,
        }
    }
}

/// ICE server configuration
#[derive(Debug, Clone)]
pub struct IceServer {
    /// Server URLs (STUN or TURN)
    pub urls: Vec<String>,
    /// Username (for TURN)
    pub username: Option<String>,
    /// Credential (for TURN)
    pub credential: Option<String>,
    /// Credential type
    pub credential_type: IceCredentialType,
}

/// ICE credential type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceCredentialType {
    /// Password-based credential
    Password,
    /// OAuth token
    Token,
}

/// ICE transport policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceTransportPolicy {
    /// Use all candidates
    All,
    /// Only relay candidates
    Relay,
}

/// Bundle policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundlePolicy {
    /// Balanced bundle policy
    Balanced,
    /// Max bundle policy
    MaxBundle,
    /// Max compat policy
    MaxCompat,
}

/// RTCP mux policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtcpMuxPolicy {
    /// Negotiate RTCP mux
    Negotiate,
    /// Require RTCP mux
    Require,
}

/// WebRTC peer connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PeerConnectionState {
    /// New peer connection
    New,
    /// ICE gathering
    Gathering,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Disconnected
    Disconnected,
    /// Failed
    Failed,
    /// Closed
    Closed,
}

/// ICE connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IceConnectionState {
    /// New ICE connection
    New,
    /// ICE checking
    Checking,
    /// ICE connected
    Connected,
    /// ICE completed
    Completed,
    /// ICE failed
    Failed,
    /// ICE disconnected
    Disconnected,
    /// ICE closed
    Closed,
}

/// ICE gathering state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IceGatheringState {
    /// New ICE gathering
    New,
    /// ICE gathering in progress
    Gathering,
    /// ICE gathering complete
    Complete,
}

/// Signaling state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SignalingState {
    /// Stable signaling state
    Stable,
    /// Have local offer
    HaveLocalOffer,
    /// Have remote offer
    HaveRemoteOffer,
    /// Have local pranswer
    HaveLocalPranswer,
    /// Have remote pranswer
    HaveRemotePranswer,
    /// Closed
    Closed,
}

/// Media stream type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    /// Audio media
    Audio,
    /// Video media
    Video,
    /// Application data
    Application,
}

/// WebRTC peer connection
pub struct PeerConnection {
    /// Peer connection ID
    id: u32,
    /// Configuration
    config: WebRtcConfig,
    /// ICE agent (wrapped in Arc<Mutex<>> for concurrent mutable access)
    ice_agent: Mutex<Option<Arc<Mutex<IceAgent>>>>,
    /// DTLS state
    dtls_state: Mutex<DtlsState>,
    /// SRTP sessions (one per media stream)
    srtp_sessions: RwLock<BTreeMap<u32, Arc<SrtpSession>>>,
    /// RTP sessions
    rtp_sessions: RwLock<BTreeMap<u32, Arc<RtpSession>>>,
    /// Data channels
    data_channels: RwLock<BTreeMap<u16, Arc<DataChannel>>>,
    /// Local media streams
    local_streams: Mutex<Vec<MediaStream>>,
    /// Remote media streams
    remote_streams: Mutex<Vec<MediaStream>>,
    /// Peer connection state
    state: AtomicU32, // PeerConnectionState
    /// ICE connection state
    ice_state: AtomicU32, // IceConnectionState
    /// ICE gathering state
    gathering_state: AtomicU32, // IceGatheringState
    /// Signaling state
    signaling_state: AtomicU32, // SignalingState
    /// Next data channel ID
    next_data_channel_id: AtomicU32,
    /// Next media stream ID
    next_stream_id: AtomicU32,
    /// Local SDP
    local_sdp: Mutex<Option<SessionDescription>>,
    /// Remote SDP
    remote_sdp: Mutex<Option<SessionDescription>>,
    /// ICE candidates
    local_candidates: Mutex<Vec<IceCandidate>>,
    remote_candidates: Mutex<Vec<IceCandidate>>,
    /// Is closed
    closed: AtomicBool,
}

impl PeerConnection {
    /// Create a new peer connection
    pub fn new(id: u32, config: WebRtcConfig) -> Self {
        Self {
            id,
            config,
            ice_agent: Mutex::new(None),  // Will be initialized when creating offer/answer
            dtls_state: Mutex::new(DtlsState::New),
            srtp_sessions: RwLock::new(BTreeMap::new()),
            rtp_sessions: RwLock::new(BTreeMap::new()),
            data_channels: RwLock::new(BTreeMap::new()),
            local_streams: Mutex::new(Vec::new()),
            remote_streams: Mutex::new(Vec::new()),
            state: AtomicU32::new(PeerConnectionState::New as u32),
            ice_state: AtomicU32::new(IceConnectionState::New as u32),
            gathering_state: AtomicU32::new(IceGatheringState::New as u32),
            signaling_state: AtomicU32::new(SignalingState::Stable as u32),
            next_data_channel_id: AtomicU32::new(0),
            next_stream_id: AtomicU32::new(0),
            local_sdp: Mutex::new(None),
            remote_sdp: Mutex::new(None),
            local_candidates: Mutex::new(Vec::new()),
            remote_candidates: Mutex::new(Vec::new()),
            closed: AtomicBool::new(false),
        }
    }

    /// Initialize the peer connection
    pub fn initialize(&self) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        // Initialize ICE agent
        let mut ice_guard = self.ice_agent.lock();
        let ice_agent = Arc::new(Mutex::new(IceAgent::new(self.id, self.config.clone())));
        *ice_guard = Some(ice_agent);

        // Set initial state
        self.set_state(PeerConnectionState::Gathering);
        self.set_ice_gathering_state(IceGatheringState::New);

        crate::log_info!("PeerConnection {} initialized", self.id);
        Ok(())
    }

    /// Create an offer
    pub fn create_offer(&self, options: &OfferOptions) -> Result<SessionDescription, WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        // Create SDP offer
        let mut sdp = SessionDescription::new(SdpType::Offer);

        // Add ICE credentials
        if let Some(ice_agent) = self.ice_agent.lock().as_ref() {
            let ice_creds = ice_agent.lock().get_credentials();
            sdp.set_ice_credentials(ice_creds);
        }

        // Add media sections
        if options.audio_to_receiver {
            sdp.add_media_section(MediaSection::new(MediaType::Audio));
        }
        if options.video_to_receiver {
            sdp.add_media_section(MediaSection::new(MediaType::Video));
        }

        // Add data channel if enabled
        if self.config.enable_data_channels {
            sdp.add_media_section(MediaSection::new(MediaType::Application));
        }

        // Set as local SDP
        *self.local_sdp.lock() = Some(sdp.clone());

        // Update signaling state
        self.set_signaling_state(SignalingState::HaveLocalOffer);

        Ok(sdp)
    }

    /// Create an answer
    pub fn create_answer(&self) -> Result<SessionDescription, WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        let remote_sdp = self.remote_sdp.lock();
        let remote = remote_sdp.as_ref()
            .ok_or(WebRtcError::NoRemoteDescription)?;

        // Create SDP answer based on remote offer
        let mut sdp = SessionDescription::new(SdpType::Answer);

        // Copy remote ICE credentials
        sdp.set_ice_credentials(remote.get_ice_credentials());

        // Match remote media sections
        for media in remote.get_media_sections() {
            sdp.add_media_section(media.clone());
        }

        drop(remote_sdp);

        // Set as local SDP
        *self.local_sdp.lock() = Some(sdp.clone());

        // Update signaling state
        self.set_signaling_state(SignalingState::Stable);

        Ok(sdp)
    }

    /// Set local description
    pub fn set_local_description(&self, desc: SessionDescription) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        *self.local_sdp.lock() = Some(desc.clone());

        // Start ICE gathering
        if let Some(ice_agent) = self.ice_agent.lock().as_ref() {
            self.set_ice_gathering_state(IceGatheringState::Gathering);
            ice_agent.lock().gather_candidates()?;
        }

        Ok(())
    }

    /// Set remote description
    pub fn set_remote_description(&self, desc: SessionDescription) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        *self.remote_sdp.lock() = Some(desc);

        // Update signaling state
        let current_state = self.signaling_state.load(Ordering::Acquire);
        let state = unsafe { core::mem::transmute::<u32, SignalingState>(current_state) };
        match state {
            SignalingState::HaveLocalOffer => {
                self.set_signaling_state(SignalingState::Stable);
            },
            SignalingState::Stable => {
                self.set_signaling_state(SignalingState::HaveRemoteOffer);
            },
            _ => {},
        }

        Ok(())
    }

    /// Add ICE candidate
    pub fn add_ice_candidate(&self, candidate: IceCandidate) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        self.remote_candidates.lock().push(candidate.clone());

        // Add to ICE agent
        if let Some(ice_agent) = self.ice_agent.lock().as_ref() {
            ice_agent.lock().add_remote_candidate(candidate)?;
        }

        Ok(())
    }

    /// Add data channel
    pub fn create_data_channel(&self, label: String, config: DataChannelConfig)
        -> Result<Arc<DataChannel>, WebRtcError>
    {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        if !self.config.enable_data_channels {
            return Err(WebRtcError::DataChannelsDisabled);
        }

        let id = self.next_data_channel_id.fetch_add(1, Ordering::SeqCst) as u16;
        let channel = Arc::new(DataChannel::new(id, label, config));

        self.data_channels.write().insert(id, channel.clone());

        crate::log_info!("Created data channel {} with label '{}'", id, channel.label());
        Ok(channel)
    }

    /// Add media stream
    pub fn add_stream(&self, stream: MediaStream) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        self.local_streams.lock().push(stream);
        Ok(())
    }

    /// Remove media stream
    pub fn remove_stream(&self, stream_id: u32) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        let mut streams = self.local_streams.lock();
        streams.retain(|s| s.id() != stream_id);
        Ok(())
    }

    /// Send RTP packet
    pub fn send_rtp(&self, packet: RtpPacket, stream_id: u32) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        // Get RTP session
        let sessions = self.rtp_sessions.read();
        let session = sessions.get(&stream_id)
            .ok_or(WebRtcError::NoRtpSession)?;

        session.send_packet(packet)
            .map_err(|e| WebRtcError::RtpError(e.to_string()))
    }

    /// Receive RTP packet
    pub fn receive_rtp(&self, packet: &mut [u8], stream_id: u32) -> Result<(), WebRtcError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        // Get RTP session
        let sessions = self.rtp_sessions.read();
        let session = sessions.get(&stream_id)
            .ok_or(WebRtcError::NoRtpSession)?;

        // Try to get mutable access through Arc
        // This requires RtpSession to be Clone
        match Arc::try_unwrap(session.clone()) {
            Ok(mut sess) => {
                sess.receive_packet(packet)
                    .map_err(|e| WebRtcError::RtpError(e.to_string()))
            }
            Err(_) => {
                // Arc has multiple references, cannot get mutable access
                // For now, just return an error
                let _ = packet;
                Err(WebRtcError::Internal("Cannot get mutable access to RTP session".to_string()))
            }
        }
    }

    /// Close the peer connection
    pub fn close(&self) -> Result<(), WebRtcError> {
        if self.closed.swap(true, Ordering::Acquire) {
            return Err(WebRtcError::Closed);
        }

        self.set_state(PeerConnectionState::Closed);
        self.set_signaling_state(SignalingState::Closed);
        self.set_ice_state(IceConnectionState::Closed);

        // Close all data channels
        let channels = self.data_channels.read();
        for channel in channels.values() {
            channel.close();
        }
        drop(channels);

        // Close ICE agent
        *self.ice_agent.lock() = None;

        crate::log_info!("PeerConnection {} closed", self.id);
        Ok(())
    }

    /// Get peer connection state
    pub fn state(&self) -> PeerConnectionState {
        unsafe { core::mem::transmute(self.state.load(Ordering::Acquire)) }
    }

    /// Get ICE connection state
    pub fn ice_connection_state(&self) -> IceConnectionState {
        unsafe { core::mem::transmute(self.ice_state.load(Ordering::Acquire)) }
    }

    /// Get ICE gathering state
    pub fn ice_gathering_state(&self) -> IceGatheringState {
        unsafe { core::mem::transmute(self.gathering_state.load(Ordering::Acquire)) }
    }

    /// Get signaling state
    pub fn signaling_state(&self) -> SignalingState {
        unsafe { core::mem::transmute(self.signaling_state.load(Ordering::Acquire)) }
    }

    /// Set peer connection state
    fn set_state(&self, state: PeerConnectionState) {
        self.state.store(state as u32, Ordering::Release);
    }

    /// Set ICE state
    fn set_ice_state(&self, state: IceConnectionState) {
        self.ice_state.store(state as u32, Ordering::Release);
    }

    /// Set ICE gathering state
    fn set_ice_gathering_state(&self, state: IceGatheringState) {
        self.gathering_state.store(state as u32, Ordering::Release);
    }

    /// Set signaling state
    fn set_signaling_state(&self, state: SignalingState) {
        self.signaling_state.store(state as u32, Ordering::Release);
    }
}

/// Offer options
#[derive(Debug, Clone, Default)]
pub struct OfferOptions {
    /// Offer to receive audio
    pub audio_to_receiver: bool,
    /// Offer to receive video
    pub video_to_receiver: bool,
    /// Voice activity detection
    pub voice_activity_detection: bool,
    /// ICE restart
    pub ice_restart: bool,
}

/// Answer options
#[derive(Debug, Clone, Default)]
pub struct AnswerOptions {
    /// Voice activity detection
    pub voice_activity_detection: bool,
}

/// Session description (SDP)
#[derive(Debug, Clone)]
pub struct SessionDescription {
    /// SDP type
    sdp_type: SdpType,
    /// SDP content
    sdp: String,
    /// ICE username fragment
    ice_ufrag: Option<String>,
    /// ICE password
    ice_pwd: Option<String>,
    /// Media sections
    media_sections: Vec<MediaSection>,
}

/// SDP type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdpType {
    /// SDP offer
    Offer,
    /// SDP pranswer (provisional answer)
    Pranswer,
    /// SDP answer
    Answer,
    /// SDP rollback
    Rollback,
}

impl SessionDescription {
    /// Create a new session description
    pub fn new(sdp_type: SdpType) -> Self {
        Self {
            sdp_type,
            sdp: String::new(),
            ice_ufrag: None,
            ice_pwd: None,
            media_sections: Vec::new(),
        }
    }

    /// Get SDP type
    pub fn get_type(&self) -> SdpType {
        self.sdp_type
    }

    /// Get SDP content
    pub fn get_sdp(&self) -> &str {
        &self.sdp
    }

    /// Set SDP content
    pub fn set_sdp(&mut self, sdp: String) {
        self.sdp = sdp;
    }

    /// Set ICE credentials
    pub fn set_ice_credentials(&mut self, creds: IceCredentials) {
        self.ice_ufrag = Some(creds.username);
        self.ice_pwd = Some(creds.password);
    }

    /// Get ICE credentials
    pub fn get_ice_credentials(&self) -> IceCredentials {
        IceCredentials {
            username: self.ice_ufrag.clone().unwrap_or_default(),
            password: self.ice_pwd.clone().unwrap_or_default(),
        }
    }

    /// Add media section
    pub fn add_media_section(&mut self, media: MediaSection) {
        self.media_sections.push(media);
    }

    /// Get media sections
    pub fn get_media_sections(&self) -> &[MediaSection] {
        &self.media_sections
    }

    /// Serialize to SDP format
    pub fn serialize(&self) -> String {
        let mut sdp = String::from("v=0\r\n");
        sdp.push_str("o=- 0 0 IN IP4 0.0.0.0\r\n");
        sdp.push_str("s=-\r\n");
        sdp.push_str("t=0 0\r\n");

        // Add ICE credentials
        if let Some(ref ufrag) = self.ice_ufrag {
            sdp.push_str(&format!("a=ice-ufrag:{}\r\n", ufrag));
        }
        if let Some(ref pwd) = self.ice_pwd {
            sdp.push_str(&format!("a=ice-pwd:{}\r\n", pwd));
        }

        // Add media sections
        for media in &self.media_sections {
            sdp.push_str(&media.serialize());
        }

        sdp
    }
}

/// Media section in SDP
#[derive(Debug, Clone)]
pub struct MediaSection {
    /// Media type
    media_type: MediaType,
    /// Port number
    port: u16,
    /// Protocol (RTP/AVP, etc.)
    protocol: String,
    /// Codec list
    codecs: Vec<Codec>,
    /// SSRC
    ssrc: Option<u32>,
}

impl MediaSection {
    /// Create a new media section
    pub fn new(media_type: MediaType) -> Self {
        Self {
            media_type,
            port: 9,  // Default discard port
            protocol: "RTP/AVP".to_string(),
            codecs: Vec::new(),
            ssrc: None,
        }
    }

    /// Set port
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    /// Add codec
    pub fn add_codec(&mut self, codec: Codec) {
        self.codecs.push(codec);
    }

    /// Set SSRC
    pub fn set_ssrc(&mut self, ssrc: u32) {
        self.ssrc = Some(ssrc);
    }

    /// Serialize to SDP format
    pub fn serialize(&self) -> String {
        let media_str = match self.media_type {
            MediaType::Audio => "audio",
            MediaType::Video => "video",
            MediaType::Application => "application",
        };

        let mut sdp = format!("m={} {} {}\r\n", media_str, self.port, self.protocol);

        // Add codecs
        for codec in &self.codecs {
            sdp.push_str(&format!("a=rtpmap:{} {}/{}\r\n",
                codec.payload_type, codec.name, codec.clock_rate));
        }

        // Add SSRC
        if let Some(ssrc) = self.ssrc {
            sdp.push_str(&format!("a=ssrc:{} cname: {}\r\n", ssrc, "nos"));
        }

        sdp
    }
}

/// Codec information
#[derive(Debug, Clone)]
pub struct Codec {
    /// Payload type
    pub payload_type: u8,
    /// Codec name
    pub name: String,
    /// Clock rate (Hz)
    pub clock_rate: u32,
    /// Number of channels
    pub channels: u8,
}

/// Media stream
#[derive(Debug, Clone)]
pub struct MediaStream {
    /// Stream ID
    id: u32,
    /// Stream label
    label: String,
    /// Audio tracks
    audio_tracks: Vec<AudioTrack>,
    /// Video tracks
    video_tracks: Vec<VideoTrack>,
}

impl MediaStream {
    /// Create a new media stream
    pub fn new(id: u32, label: String) -> Self {
        Self {
            id,
            label,
            audio_tracks: Vec::new(),
            video_tracks: Vec::new(),
        }
    }

    /// Get stream ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get stream label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Add audio track
    pub fn add_audio_track(&mut self, track: AudioTrack) {
        self.audio_tracks.push(track);
    }

    /// Add video track
    pub fn add_video_track(&mut self, track: VideoTrack) {
        self.video_tracks.push(track);
    }
}

/// Audio track
#[derive(Debug, Clone)]
pub struct AudioTrack {
    /// Track ID
    id: String,
    /// Track kind
    kind: String,
    /// Enabled
    enabled: bool,
    /// Muted
    muted: bool,
}

impl AudioTrack {
    /// Create a new audio track
    pub fn new(id: String) -> Self {
        Self {
            id,
            kind: "audio".to_string(),
            enabled: true,
            muted: false,
        }
    }

    /// Get track ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Enable/disable track
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Video track
#[derive(Debug, Clone)]
pub struct VideoTrack {
    /// Track ID
    id: String,
    /// Track kind
    kind: String,
    /// Width
    width: u32,
    /// Height
    height: u32,
    /// Frame rate
    frame_rate: f32,
    /// Enabled
    enabled: bool,
    /// Muted
    muted: bool,
}

impl VideoTrack {
    /// Create a new video track
    pub fn new(id: String, width: u32, height: u32, frame_rate: f32) -> Self {
        Self {
            id,
            kind: "video".to_string(),
            width,
            height,
            frame_rate,
            enabled: true,
            muted: false,
        }
    }

    /// Get track ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Enable/disable track
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Data channel configuration
#[derive(Debug, Clone)]
pub struct DataChannelConfig {
    /// Ordered delivery
    pub ordered: bool,
    /// Maximum packet lifetime (ms)
    pub max_packet_lifetime: Option<u16>,
    /// Maximum retransmits
    pub max_retransmits: Option<u16>,
    /// Protocol
    pub protocol: String,
    /// Negotiated
    pub negotiated: bool,
    /// ID (for negotiated channels)
    pub id: Option<u16>,
}

impl Default for DataChannelConfig {
    fn default() -> Self {
        Self {
            ordered: true,
            max_packet_lifetime: None,
            max_retransmits: None,
            protocol: String::new(),
            negotiated: false,
            id: None,
        }
    }
}

/// Data channel for WebRTC data channels
pub struct DataChannel {
    /// Channel ID
    id: u16,
    /// Channel label
    label: String,
    /// Configuration
    config: DataChannelConfig,
    /// Channel state
    state: AtomicU32, // DataChannelState
    /// Buffered amount (low)
    buffered_amount_low_threshold: AtomicU32,
    /// Send queue
    send_queue: Mutex<Vec<Vec<u8>>>,
    /// Receive queue
    receive_queue: Mutex<Vec<Vec<u8>>>,
}

/// Data channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DataChannelState {
    /// Connecting
    Connecting = 0,
    /// Open
    Open = 1,
    /// Closing
    Closing = 2,
    /// Closed
    Closed = 3,
}

impl DataChannel {
    /// Create a new data channel
    pub fn new(id: u16, label: String, config: DataChannelConfig) -> Self {
        Self {
            id,
            label,
            config,
            state: AtomicU32::new(DataChannelState::Connecting as u32),
            buffered_amount_low_threshold: AtomicU32::new(0),
            send_queue: Mutex::new(Vec::new()),
            receive_queue: Mutex::new(Vec::new()),
        }
    }

    /// Get channel ID
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Get channel label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Get channel state
    pub fn state(&self) -> DataChannelState {
        unsafe { core::mem::transmute(self.state.load(Ordering::Acquire)) }
    }

    /// Send data
    pub fn send(&self, data: &[u8]) -> Result<(), WebRtcError> {
        if self.state.load(Ordering::Acquire) != DataChannelState::Open as u32 {
            return Err(WebRtcError::DataChannelClosed);
        }

        self.send_queue.lock().push(data.to_vec());
        Ok(())
    }

    /// Receive data
    pub fn receive(&self) -> Option<Vec<u8>> {
        self.receive_queue.lock().pop()
    }

    /// Close the data channel
    pub fn close(&self) {
        self.state.store(DataChannelState::Closed as u32, Ordering::Release);
    }

    /// Open the data channel
    pub fn open(&self) {
        self.state.store(DataChannelState::Open as u32, Ordering::Release);
    }
}

/// DTLS state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DtlsState {
    /// New DTLS state
    New,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Closed
    Closed,
    /// Failed
    Failed,
}

/// WebRTC error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebRtcError {
    /// Invalid SDP
    InvalidSdp(String),
    /// Invalid ICE candidate
    InvalidIceCandidate(String),
    /// No RTP session
    NoRtpSession,
    /// RTP error
    RtpError(String),
    /// Connection closed
    Closed,
    /// Internal error
    Internal(String),
    /// No remote description
    NoRemoteDescription,
    /// Data channels disabled
    DataChannelsDisabled,
    /// SRTP error
    SrtpError(String),
    /// ICE error
    IceError(String),
    /// Data channel closed
    DataChannelClosed,
    /// Invalid state
    InvalidState,
    /// Operation not supported
    NotSupported,
}

impl core::fmt::Display for WebRtcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Closed => write!(f, "Peer connection is closed"),
            Self::NoRemoteDescription => write!(f, "No remote description set"),
            Self::DataChannelsDisabled => write!(f, "Data channels are disabled"),
            Self::NoRtpSession => write!(f, "No RTP session found"),
            Self::RtpError(e) => write!(f, "RTP error: {}", e),
            Self::SrtpError(e) => write!(f, "SRTP error: {}", e),
            Self::IceError(e) => write!(f, "ICE error: {}", e),
            Self::DataChannelClosed => write!(f, "Data channel is closed"),
            Self::InvalidState => write!(f, "Invalid state for operation"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::InvalidSdp(e) => write!(f, "Invalid SDP: {}", e),
            Self::InvalidIceCandidate(e) => write!(f, "Invalid ICE candidate: {}", e),
            Self::Internal(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl From<alloc::string::String> for WebRtcError {
    fn from(s: alloc::string::String) -> Self {
        Self::IceError(s)
    }
}

/// WebRTC peer connection manager
pub struct PeerConnectionFactory {
    /// Peer connections
    connections: RwLock<BTreeMap<u32, Arc<PeerConnection>>>,
    /// Next connection ID
    next_id: AtomicU32,
}

impl PeerConnectionFactory {
    /// Create a new factory
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(BTreeMap::new()),
            next_id: AtomicU32::new(1),
        }
    }

    /// Create a new peer connection
    pub fn create_peer_connection(&self, config: WebRtcConfig)
        -> Result<Arc<PeerConnection>, WebRtcError>
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let pc = Arc::new(PeerConnection::new(id, config));

        pc.initialize()?;

        self.connections.write().insert(id, pc.clone());

        Ok(pc)
    }

    /// Get peer connection by ID
    pub fn get_peer_connection(&self, id: u32) -> Option<Arc<PeerConnection>> {
        self.connections.read().get(&id).cloned()
    }

    /// Close peer connection
    pub fn close_peer_connection(&self, id: u32) -> Result<(), WebRtcError> {
        let connections = self.connections.read();
        if let Some(pc) = connections.get(&id) {
            pc.close()?;
            drop(connections);
            self.connections.write().remove(&id);
            Ok(())
        } else {
            Err(WebRtcError::Closed)
        }
    }
}

impl Default for PeerConnectionFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_connection_create() {
        let config = WebRtcConfig::default();
        let pc = PeerConnection::new(1, config);
        assert_eq!(pc.id, 1);
        assert_eq!(pc.state(), PeerConnectionState::New);
    }

    #[test]
    fn test_sdp_creation() {
        let sdp = SessionDescription::new(SdpType::Offer);
        assert_eq!(sdp.get_type(), SdpType::Offer);
    }

    #[test]
    fn test_data_channel() {
        let config = DataChannelConfig::default();
        let dc = DataChannel::new(0, "test".to_string(), config);
        assert_eq!(dc.id(), 0);
        assert_eq!(dc.label(), "test");
        assert_eq!(dc.state(), DataChannelState::Connecting);
    }
}
