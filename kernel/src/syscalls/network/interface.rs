//! Network interface syscalls

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::sync::Mutex;

use super::*;
use super::super::common::SyscallError;

// ============================================================================
// Network Interface State
// ============================================================================

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub index: u32,
    pub flags: u32,          // IFF_UP, IFF_BROADCAST, etc.
    pub mtu: u32,
    pub hw_addr: [u8; 6],    // MAC address
    pub ipv4_addr: u32,      // IPv4 address
    pub ipv4_netmask: u32,   // IPv4 netmask
    pub ipv4_broadcast: u32, // IPv4 broadcast address
}

// Interface flags
const IFF_UP: u32 = 0x1;
const IFF_BROADCAST: u32 = 0x2;
const IFF_LOOPBACK: u32 = 0x8;
const IFF_RUNNING: u32 = 0x40;
const IFF_MULTICAST: u32 = 0x1000;

// Global interface list
static INTERFACES: Mutex<Vec<NetworkInterface>> = Mutex::new(Vec::new());
static NEXT_IF_INDEX: Mutex<u32> = Mutex::new(1);

/// Initialize default network interfaces
pub fn init_interfaces() {
    let mut interfaces = INTERFACES.lock();
    
    // Create loopback interface
    let lo = NetworkInterface {
        name: String::from("lo"),
        index: 1,
        flags: IFF_UP | IFF_LOOPBACK | IFF_RUNNING,
        mtu: 65536,
        hw_addr: [0; 6],
        ipv4_addr: 0x7F000001,      // 127.0.0.1
        ipv4_netmask: 0xFF000000,   // 255.0.0.0
        ipv4_broadcast: 0x7FFFFFFF, // 127.255.255.255
    };
    interfaces.push(lo);
    
    // Create eth0 interface (dummy for now)
    let eth0 = NetworkInterface {
        name: String::from("eth0"),
        index: 2,
        flags: IFF_UP | IFF_BROADCAST | IFF_RUNNING | IFF_MULTICAST,
        mtu: 1500,
        hw_addr: [0x02, 0x00, 0x00, 0x00, 0x00, 0x01],
        ipv4_addr: 0xC0A80001,      // 192.168.0.1
        ipv4_netmask: 0xFFFFFF00,   // 255.255.255.0
        ipv4_broadcast: 0xC0A800FF, // 192.168.0.255
    };
    interfaces.push(eth0);
    
    let mut next_idx = NEXT_IF_INDEX.lock();
    *next_idx = 3;
}

/// Configure network interface
pub fn sys_ifconfig(args: &[u64]) -> super::super::common::SyscallResult {
    use super::super::common::extract_args;
    
    // This would parse ifreq structure from user space
    // For now, just return success
    let _ = args;
    Ok(0)
}

/// Get network interface information
pub fn sys_ifinfo(args: &[u64]) -> super::super::common::SyscallResult {
    use super::super::common::extract_args;
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 2)?;
    let if_index = args[0] as u32;
    let info_ptr = args[1] as usize;
    
    if info_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let interfaces = INTERFACES.lock();
    
    // Find interface by index
    for iface in interfaces.iter() {
        if iface.index == if_index {
            // Return interface information
            // In a real implementation, we would copy the full ifreq structure
            let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
            let table = crate::process::manager::PROC_TABLE.lock();
            let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
            let pagetable = proc.pagetable;
            drop(table);
            
            if !pagetable.is_null() {
                let mtu = iface.mtu;
                unsafe {
                    copyout(pagetable, info_ptr, &mtu as *const _ as *const u8, 4)
                        .map_err(|_| SyscallError::BadAddress)?;
                }
            }
            
            return Ok(0);
        }
    }
    
    Err(SyscallError::NotFound)
}

/// List network interfaces
pub fn sys_iflist(args: &[u64]) -> super::super::common::SyscallResult {
    use super::super::common::extract_args;
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 2)?;
    let buf_ptr = args[0] as usize;
    let buf_len = args[1] as usize;
    
    let interfaces = INTERFACES.lock();
    let count = interfaces.len();
    
    // If buf_ptr is 0, just return the count
    if buf_ptr == 0 {
        return Ok(count as u64);
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let table = crate::process::manager::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Copy interface indices to buffer
    let mut offset = 0;
    for iface in interfaces.iter() {
        if offset + 4 > buf_len {
            break;
        }
        
        let idx = iface.index;
        unsafe {
            copyout(pagetable, buf_ptr + offset, &idx as *const _ as *const u8, 4)
                .map_err(|_| SyscallError::BadAddress)?;
        }
        offset += 4;
    }
    
    Ok(count as u64)
}

/// Add a route to the routing table
pub fn add_route(dest: &str, gateway: &str, _netmask: &str) -> Result<(), i32> {
    // Parse destination and gateway addresses
    // For now, just log and accept
    crate::println!("[network] Adding route: {} via {}", dest, gateway);
    Ok(())
}

/// Bring network interface up
pub fn interface_up(interface_name: &str) -> Result<(), i32> {
    let mut interfaces = INTERFACES.lock();
    
    for iface in interfaces.iter_mut() {
        if iface.name == interface_name {
            iface.flags |= IFF_UP | IFF_RUNNING;
            crate::println!("[network] Interface {} is up", interface_name);
            return Ok(());
        }
    }
    
    Err(crate::reliability::errno::ENODEV)
}

/// Add address to network interface
pub fn add_interface_address(interface_name: &str, address: &str, netmask: &str) -> Result<(), i32> {
    let mut interfaces = INTERFACES.lock();
    
    for iface in interfaces.iter_mut() {
        if iface.name == interface_name {
            // Parse and set address (simplified - would need proper IP parsing)
            crate::println!("[network] Set {} address to {} netmask {}", 
                interface_name, address, netmask);
            return Ok(());
        }
    }
    
    Err(crate::reliability::errno::ENODEV)
}

/// Set interface MTU
pub fn set_interface_mtu(interface_name: &str, mtu: u32) -> Result<(), i32> {
    if mtu < 68 || mtu > 65535 {
        return Err(crate::reliability::errno::EINVAL);
    }
    
    let mut interfaces = INTERFACES.lock();
    
    for iface in interfaces.iter_mut() {
        if iface.name == interface_name {
            iface.mtu = mtu;
            crate::println!("[network] Set {} MTU to {}", interface_name, mtu);
            return Ok(());
        }
    }
    
    Err(crate::reliability::errno::ENODEV)
}

/// Create a virtual ethernet pair
pub fn create_veth_pair(name1: &str, name2: &str) -> Result<(), i32> {
    let mut interfaces = INTERFACES.lock();
    let mut next_idx = NEXT_IF_INDEX.lock();
    
    // Create first veth
    let veth1 = NetworkInterface {
        name: String::from(name1),
        index: *next_idx,
        flags: IFF_BROADCAST | IFF_MULTICAST,
        mtu: 1500,
        hw_addr: [0x02, 0x00, 0x00, 0x00, 0x00, *next_idx as u8],
        ipv4_addr: 0,
        ipv4_netmask: 0,
        ipv4_broadcast: 0,
    };
    *next_idx += 1;
    
    // Create second veth
    let veth2 = NetworkInterface {
        name: String::from(name2),
        index: *next_idx,
        flags: IFF_BROADCAST | IFF_MULTICAST,
        mtu: 1500,
        hw_addr: [0x02, 0x00, 0x00, 0x00, 0x00, *next_idx as u8],
        ipv4_addr: 0,
        ipv4_netmask: 0,
        ipv4_broadcast: 0,
    };
    *next_idx += 1;
    
    interfaces.push(veth1);
    interfaces.push(veth2);
    
    crate::println!("[network] Created veth pair: {} <-> {}", name1, name2);
    
    Ok(())
}

/// Create a network bridge
pub fn create_bridge(name: &str) -> Result<(), i32> {
    let mut interfaces = INTERFACES.lock();
    let mut next_idx = NEXT_IF_INDEX.lock();
    
    // Create bridge interface
    let bridge = NetworkInterface {
        name: String::from(name),
        index: *next_idx,
        flags: IFF_BROADCAST | IFF_MULTICAST,
        mtu: 1500,
        hw_addr: [0x02, 0x00, 0x00, 0x00, 0x00, *next_idx as u8],
        ipv4_addr: 0,
        ipv4_netmask: 0,
        ipv4_broadcast: 0,
    };
    *next_idx += 1;
    
    interfaces.push(bridge);
    
    crate::println!("[network] Created bridge: {}", name);
    
    Ok(())
}