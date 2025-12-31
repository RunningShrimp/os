# UDP Optimization API Reference

## Quick Start Guide

### 1. UDP Fast Path Usage

```rust
use crate::subsystems::net::udp_fast_path::*;

// Create fast path instance (detect CPU count)
let fast_path = UdpFastPath::new(num_cpus);

// Enable kernel bypass for latency-critical apps
fast_path.enable_kernel_bypass(true);

// Set batch size
fast_path.set_batch_size(32);

// Zero-copy receive
let mut buffer = [0u8; 65536];
let (len, addr) = fast_path.recv_zero_copy(cpu_id, &mut buffer)?;

// Batched send
let packets = vec![
    (addr1, data1),
    (addr2, data2),
    // ... up to 32 packets
];
let sent = fast_path.send_batch(cpu_id, &packets)?;

// Fast socket lookup
if let Some(socket) = fast_path.fast_lookup(cpu_id, &addr) {
    // Use socket for zero-copy operations
}
```

### 2. UDP Offload Usage

```rust
use crate::subsystems::net::udp_optimization::*;

// Configure offload settings
let config = UdpOffloadConfig {
    gso_enabled: true,
    gro_enabled: true,
    max_gso_size: 9216,  // 9KB jumbo frames
    checksum_offload: true,
    jumbo_frames: true,
    max_batch_size: 32,
};

// Create offload manager
let offload = UdpOffload::new(config);

// Segment large packet with GSO
let segments = offload.segment(&large_packet, mtu)?;

// Coalesce small packets with GRO
let coalesced = offload.coalesce(&small_packets)?;

// Calculate checksum (with hardware offload)
offload.calculate_checksum(&mut packet, src_addr, dst_addr)?;

// Verify checksum
if offload.verify_checksum(&packet, src_addr, dst_addr) {
    // Process packet
}
```

### 3. Batched Packet Processor

```rust
use crate::subsystems::net::udp_optimization::*;

// Create batched processor
let mut processor = BatchedPacketProcessor::new(64, config);

// Add packets to batch
for packet in incoming_packets {
    processor.add_packet(packet)?;

    // Auto-flush when batch is full
}

// Manual flush
let processed = processor.flush()?;

// Access offload manager
processor.offload().set_gso_enabled(false);
```

### 4. UDP Multicast Usage

```rust
use crate::subsystems::net::udp_multicast::*;

// Create multicast manager with IGMPv3
let multicast = UdpMulticast::new(IgmpVersion::V3);

// Join multicast group
multicast.join_group(group_addr, None)?;

// Join source-specific multicast
multicast.join_group(group_addr, Some(source_addr))?;

// Add multicast route
multicast.add_route(
    source_addr,
    group_addr,
    input_interface,
    vec![iface1, iface2, iface3],
)?;

// Forward multicast packet
let output_ifaces = multicast.forward_packet(&packet, source, group)?;

// Get group information
if let Some(group) = multicast.get_group(group_addr) {
    println!("Members: {}", group.member_count());
}

// Get statistics
let stats = multicast.get_stats();
println!("Forwarded: {} packets", stats.packets_forwarded.load(Ordering::Relaxed));
```

---

## Performance Tuning Parameters

### UdpFastPath

| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `batch_size` | 32 | 1-32 | Packets per batch |
| `buffer_pool_size` | 1024 | 256-4096 | Zero-copy buffers |
| `max_cpu_sockets` | 128 | 64-256 | Sockets per CPU cache |

### UdpOffload

| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `max_gso_size` | 9216 | 1500-65535 | Max GSO segment size |
| `max_gro_size` | 65536 | 9216-65536 | Max GRO packet size |
| `min_gro_count` | 4 | 2-16 | Min packets for aggregation |
| `max_batch_size` | 32 | 8-64 | Max batch size |

### UdpMulticast

| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `cache_ttl` | 64 | 16-256 | Forwarding cache TTL |
| `group_timeout` | 300 | 60-3600 | Group membership timeout (s) |
| `query_interval` | 125 | 60-300 | IGMP query interval (s) |

---

## Well-Known Multicast Addresses

```rust
// IPv4 Multicast Ranges
224.0.0.0 - 224.0.0.255  // Local network (TTL=1)
224.0.1.0 - 238.255.255.255  // Globally scoped
239.0.0.0 - 239.255.255.255  // Organization-local

// Specific Addresses
ALL_SYSTEMS      = 224.0.0.1   // All systems
ALL_ROUTERS      = 224.0.0.2   // All routers
ALL_OSPF_ROUTERS = 224.0.0.5   // OSPF routers
MDNS             = 224.0.0.251 // mDNS
```

---

## Error Handling

All modules use Result types for proper error handling:

```rust
// Fast Path Errors
pub enum UdpFastPathError {
    BufferExhausted,      // No buffers available
    BatchTooLarge,        // Batch exceeds maximum
    InvalidAddress,       // Invalid socket address
    SocketNotFound,       // Socket lookup failed
    KernelBypassDisabled, // Bypass not enabled
    NotSupported,         // Operation not available
}

// Offload Errors
pub enum UdpOffloadError {
    GsoDisabled,          // GSO not enabled
    GroDisabled,          // GRO not enabled
    PacketTooLarge,       // Exceeds maximum size
    PacketMismatch,       // Packets don't match
    TooFewPackets,        // Not enough for GRO
    ChecksumFailed,       // Checksum error
    PmtudDisabled,        // MTU discovery disabled
    BatchFailed,          // Batch processing failed
}

// Multicast Errors
pub enum MulticastError {
    InvalidAddress,       // Not a multicast address
    NotMember,            // Not a group member
    RouteNotFound,        // No route for (S,G)
    SsmNotSupported,      // Need IGMPv3 for SSM
    NotSupported,         // Operation not available
    FilterFailed,         // Source filtering failed
}
```

---

## Statistics Tracking

### Fast Path Statistics
```rust
pub struct UdpFastPathStats {
    pub zero_copy_recvs: AtomicU64,      // Zero-copy receives
    pub fast_lookup_hits: AtomicU64,     // Cache hits
    pub fast_lookup_misses: AtomicU64,   // Cache misses
    pub batch_sends: AtomicU64,          // Batched sends
    pub batch_packets_sent: AtomicU64,   // Packets in batches
    pub kernel_bypass_ops: AtomicU64,    // Kernel bypass ops
}
```

### Offload Statistics
```rust
pub struct UdpOffloadStats {
    pub gso_segments: AtomicU64,         // GSO segments created
    pub gso_bytes: AtomicU64,            // GSO bytes processed
    pub gro_coalesced: AtomicU64,        // GRO packets coalesced
    pub gro_aggregated: AtomicU64,       // GRO packets aggregated
    pub checksums_offloaded: AtomicU64,  // Checksums offloaded
    pub jumbo_frames_sent: AtomicU64,    // Jumbo frames sent
    pub batched_ops: AtomicU64,          // Batched operations
}
```

### Multicast Statistics
```rust
pub struct MulticastStats {
    pub groups_joined: AtomicU64,        // Groups joined
    pub groups_left: AtomicU64,          // Groups left
    pub packets_forwarded: AtomicU64,    // Packets forwarded
    pub bytes_forwarded: AtomicU64,      // Bytes forwarded
    pub cache_hits: AtomicU64,           // Cache hits
    pub cache_misses: AtomicU64,         // Cache misses
}
```

---

## Best Practices

### 1. Zero-Copy Operations
- Preallocate buffers during initialization
- Use per-CPU buffer pools to avoid contention
- Return buffers promptly to pool after use

### 2. Batch Processing
- Use optimal batch size (16-32 packets)
- Batch similar operations together
- Flush batches proactively, not just when full

### 3. Socket Caching
- Keep frequently used sockets in per-CPU cache
- Monitor cache hit/miss ratios
- Adjust cache size based on workload

### 4. Hardware Offload
- Enable GSO/GRO for bulk transfers
- Use checksum offload when available
- Validate hardware capabilities at runtime

### 5. Multicast
- Use SSM for controlled source filtering
- Implement proper leave group handling
- Monitor forwarding cache hit rates
- Use IGMPv3 for advanced features

---

## Performance Monitoring

```rust
// Monitor fast path performance
let stats = fast_path.get_stats(cpu_id)?;
let hit_rate = stats.fast_lookup_hits as f64 /
               (stats.fast_lookup_hits + stats.fast_lookup_misses) as f64;
println!("Cache hit rate: {:.2}%", hit_rate * 100.0);

// Monitor offload efficiency
let stats = offload.get_stats();
let gro_ratio = stats.gro_coalesced as f64 /
               stats.gro_aggregated.max(1) as f64;
println!("GRO ratio: {:.2}", gro_ratio);

// Monitor multicast forwarding
let stats = multicast.get_stats();
let cache_hit_rate = stats.cache_hits as f64 /
                     (stats.cache_hits + stats.cache_misses) as f64;
println!("Forwarding cache hit rate: {:.2}%", cache_hit_rate * 100.0);
```
