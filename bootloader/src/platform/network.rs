//! Network support module
#![cfg(feature = "network_support")]

//! Minimal network support for bootloader

#[derive(Clone, Copy, Debug)]
pub struct NetworkConfig {
    pub ip_address: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
}

impl NetworkConfig {
    pub fn new() -> Self {
        NetworkConfig {
            ip_address: [0, 0, 0, 0],
            netmask: [0, 0, 0, 0],
            gateway: [0, 0, 0, 0],
        }
    }
}
