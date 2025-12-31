# Security Mechanisms Design / 安全机制设计
# 安全机制设计文档

## Table of Contents / 目录

1. [Memory Security / 内存安全](#memory-security)
2. [Access Control / 访问控制](#access-control)
3. [Sandboxing / 沙箱隔离](#sandboxing)
4. [Runtime Protections / 运行时保护](#runtime-protections)
5. [Audit and Monitoring / 审计与监控](#audit-and-monitoring)
6. [Security Hardening Checklist / 安全加固清单](#security-hardening-checklist)
7. [Performance Impact / 性能影响](#performance-impact)

---

## Memory Security / 内存安全

### ASLR (Address Space Layout Randomization) / 地址空间布局随机化

#### Overview / 概述

Address Space Layout Randomization (ASLR) is a security technique that randomly arranges the address space positions of key data areas of a process, including the base of the executable and the stack, heap, and libraries. This makes it difficult for attackers to predict memory addresses and exploit memory corruption vulnerabilities.

地址空间布局随机化（ASLR）是一种安全技术，通过随机排列进程关键数据区域的地址空间位置（包括可执行文件基址、栈、堆和库），使攻击者难以预测内存地址并利用内存损坏漏洞。

#### Implementation Details / 实现细节

##### Stack Randomization / 栈随机化

The stack location is randomized at process creation time with configurable entropy bits. Our implementation supports 16-28 bits of stack randomization, providing up to 268 million possible stack positions.

栈位置在进程创建时随机化，具有可配置的熵位数。我们的实现支持16-28位的栈随机化，提供多达2.68亿个可能的栈位置。

**Configuration / 配置:**
```rust
pub struct AslrEntropy {
    pub stack_bits: u8,     // Default: 24 bits (16M positions)
    pub mmap_bits: u8,       // Default: 28 bits (268M positions)
    pub heap_bits: u8,       // Default: 16 bits (64K positions)
    pub exec_bits: u8,       // Default: 16 bits (64K positions)
    pub pie_bits: u8,        // Default: 16 bits (64K positions)
    pub library_bits: u8,    // Default: 24 bits (16M positions)
}
```

**Entropy Quality / 熵质量:**

Our ASLR implementation uses multiple entropy sources to ensure high-quality randomization:

我们的ASLR实现使用多种熵源来确保高质量的随机化：

1. **RDRAND Instruction**: Hardware-generated random numbers from Intel/AMD CPUs
   **RDRAND指令**: 来自Intel/AMD CPU的硬件生成的随机数

2. **High-Resolution Timer**: TSC (Time Stamp Counter) providing timing-based entropy
   **高分辨率定时器**: TSC（时间戳计数器）提供基于时间的熵

3. **CPU-specific Data**: Core ID, frequency, and other processor-specific information
   **CPU特定数据**: 核心ID、频率和其他处理器特定信息

4. **Memory Layout**: Stack pointer and heap base addresses
   **内存布局**: 栈指针和堆基地址

5. **Process Information**: PID, parent PID, and execution context
   **进程信息**: PID、父PID和执行上下文

```rust
fn generate_seed(&self) -> u64 {
    let mut seed: u64 = 0;

    // Combine multiple entropy sources
    seed ^= self.get_rdrand_entropy().unwrap_or(0);
    seed ^= self.random_number() as u64;
    seed ^= self.get_timestamp_entropy();
    seed ^= self.get_cpu_entropy();
    seed ^= self.get_memory_entropy();
    seed ^= self.get_process_entropy();

    // Additional mixing using bit rotations and multiplications
    seed = seed.wrapping_mul(0x517cc1b727220a95);
    seed ^= seed.rotate_right(17);
    seed ^= seed.rotate_left(43);
    seed ^= seed.rotate_right(21);

    seed
}
```

##### Heap Randomization / 堆随机化

Heap allocations are randomized using the `brk()` system call and `mmap()` for large allocations. The heap base address varies between process executions, making it difficult to predict heap object locations.

堆分配使用`brk()`系统调用和`mmap()`进行大分配随机化。堆基地址在进程执行之间变化，使得预测堆对象位置变得困难。

**Implementation / 实现:**
- **Small allocations (< 128KB)**: Randomized via `brk()` with 12-16 bits of entropy
  **小分配 (< 128KB)**: 通过`brk()`随机化，具有12-16位熵
- **Large allocations (≥ 128KB)**: Randomized via `mmap()` with 24-28 bits of entropy
  **大分配 (≥ 128KB)**: 通过`mmap()`随机化，具有24-28位熵

##### PIE (Position Independent Executable) / 位置无关可执行文件

All executables are compiled as PIE by default, allowing the base address of the executable itself to be randomized. This provides 16-24 bits of entropy depending on the architecture and address space size.

所有可执行文件默认编译为PIE，允许可执行文件本身的基地址被随机化。这根据架构和地址空间大小提供16-24位的熵。

**Compile-time Flags / 编译时标志:**
```rust
// Force PIE for all executables
pub force_pie: bool,

// Randomize shared libraries
pub randomize_libraries: bool,
```

##### Library Randomization / 库随机化

Shared libraries (DSOs) are loaded at random addresses. Each library's base address is randomized independently, providing 24 bits of entropy per library on 64-bit systems.

共享库（DSO）在随机地址加载。每个库的基地址独立随机化，在64位系统上每个库提供24位熵。

##### Re-randomization (Execshield-style) / 重新随机化

Our implementation supports periodic re-randomization of memory regions, similar to Red Hat's Execshield. This provides additional protection against information leaks that could bypass ASLR.

我们的实现支持内存区域的定期重新随机化，类似于Red Hat的Execshield。这提供了针对可能绕过ASLR的信息泄露的额外保护。

```rust
pub fn perform_periodic_rerandomization(&mut self) -> Result<usize, &'static str> {
    let mut rerandomized_count = 0;
    let interval = self.rerandomization_interval.load(Ordering::Relaxed);

    for &pid in self.process_states.keys() {
        if self.should_rerandomize() {
            self.rerandomize_process(pid)?;
            rerandomized_count += 1;
        }
    }

    Ok(rerandomized_count)
}
```

**Default Re-randomization Interval / 默认重新随机化间隔:** 300 seconds (5 minutes) / 300秒（5分钟）

#### Bypass Detection / 绕过检测

Our ASLR implementation includes bypass detection mechanisms to identify attempts to circumvent randomization:

我们的ASLR实现包括绕过检测机制，以识别试图规避随机化的行为：

**Detection Types / 检测类型:**
1. **Information Leak Detection**: Detects memory disclosures that reveal address layouts
   **信息泄露检测**: 检测揭示地址布局的内存泄露
2. **Brute Force Detection**: Identifies repeated probing of memory addresses
   **暴力破解检测**: 识别对内存地址的重复探测
3. **Side-Channel Detection**: Monitors for timing-based attacks
   **侧信道检测**: 监控基于时间的攻击
4. **ROP/JOP Detection**: Identifies return-oriented or jump-oriented programming patterns
   **ROP/JOP检测**: 识别面向返回或面向跳转的编程模式

```rust
pub enum AslrBypassType {
    None,
    InformationLeak,
    BruteForce,
    SideChannel,
    ReturnOrientedProgramming,
    JumpOrientedProgramming,
}
```

---

### Stack Protection / 栈保护

#### Stack Canaries / 栈金丝雀

Stack canaries are random values placed between local variables and the return address on the stack. They detect buffer overflow attacks before the return address can be corrupted.

栈金丝雀是放置在栈上局部变量和返回地址之间的随机值。它们在返回地址被损坏之前检测缓冲区溢出攻击。

#### Canary Types / 金丝雀类型

1. **Random Canaries**: Generated using high-entropy random sources
   **随机金丝雀**: 使用高熵随机源生成
2. **Terminator Canaries**: Include null bytes (0x00) to disrupt string-based exploits
   **终结符金丝雀**: 包含空字节（0x00）以破坏基于字符串的漏洞利用

**Canary Format / 金丝雀格式:**
```rust
// Ensure canary doesn't have common patterns
canary &= 0xFFFFFFFFFFFFFF00u64; // Clear low byte to avoid null bytes
canary |= 0xFF; // Set low byte to 0xFF for easy detection
```

#### Compiler Integration / 编译器集成

We support GCC/Clang's stack protector flags:

我们支持GCC/Clang的栈保护器标志：

- **`-fstack-protector`**: Protects functions with character arrays
  **`-fstack-protector`**: 保护具有字符数组的函数
- **`-fstack-protector-strong`**: Protects more functions (recommended)
  **`--fstack-protector-strong`**: 保护更多函数（推荐）
- **`-fstack-protector-all`**: Protects all functions (highest overhead)
  **`-fstack-protector-all`**: 保护所有函数（最高开销）

#### Canary Generation / 金丝雀生成

Canaries are generated using multiple entropy sources:

金丝雀使用多种熵源生成：

```rust
pub fn generate_canary(&self, thread_id: u64) -> u64 {
    let generation = self.generation_counter.fetch_add(1, Ordering::SeqCst);
    let base_seed = self.global_seed.load(Ordering::SeqCst);

    let mut canary = base_seed;

    // Mix in thread ID
    canary ^= thread_id;

    // Mix in generation counter
    canary ^= generation as u64;

    // Mix in timestamp
    canary ^= self.get_timestamp_entropy();

    // Add additional mixing based on configuration
    if self.config.high_entropy_canaries {
        canary ^= self.get_cpu_entropy();
        canary ^= self.get_memory_entropy();

        // Rotate and mix
        canary = canary.rotate_left(13);
        canary ^= canary >> 7;
        canary = canary.rotate_left(17);
    }

    // Ensure canary doesn't have common patterns
    canary &= 0xFFFFFFFFFFFFFF00u64;
    canary |= 0xFF;

    canary
}
```

#### Per-Thread Canaries / 每线程金丝雀

Each thread receives its own canary value, preventing canaries leaked in one thread from being used in another. Canaries are re-randomized periodically based on the configured interval.

每个线程接收自己的金丝雀值，防止在一个线程中泄露的金丝雀在另一个线程中被使用。根据配置的间隔定期重新随机化金丝雀。

**Configuration / 配置:**
```rust
pub struct CanaryConfig {
    pub canary_size: usize,                 // Default: 8 bytes (64-bit)
    pub randomization_interval: usize,       // Default: 1000 function calls
    pub per_thread_canaries: bool,           // Default: true
    pub high_entropy_canaries: bool,         // Default: true
    pub validate_on_context_switch: bool,    // Default: true
    pub corruption_action: CanaryCorruptionAction,
}
```

#### Detection and Response / 检测和响应

When canary corruption is detected, the system can take several actions:

当检测到金丝雀损坏时，系统可以采取几种行动：

**Actions / 行动:**
1. **Terminate**: Immediately kill the process (default, most secure)
   **终止**: 立即终止进程（默认，最安全）
2. **Raise Exception**: Trigger a security exception for logging
   **引发异常**: 触发安全异常以进行记录
3. **Log and Continue**: Record the violation but allow execution (debugging only)
   **记录并继续**: 记录违规但允许执行（仅用于调试）
4. **Custom Handler**: Call a user-defined handler function
   **自定义处理程序**: 调用用户定义的处理函数

```rust
pub enum CanaryCorruptionAction {
    Terminate,
    RaiseException,
    LogAndContinue,
    CustomHandler(fn(&CanaryCorruptionInfo)),
}
```

---

### Heap Protection / 堆保护

#### Safe Unlinking / 安全解链

Heap metadata is hardened to prevent exploitation of the unlink macro. Our implementation includes:

堆元数据经过强化，以防止unlink宏的利用。我们的实现包括：

1. **Pointer Obfuscation**: Pointers are XOR-encoded with random keys
   **指针混淆**: 指针使用随机密钥进行XOR编码
2. **Integrity Checks**: Metadata includes checksums to detect corruption
   **完整性检查**: 元数据包括校验和以检测损坏
3. **Safe Unlinking**: Validates pointers before dereferencing
   **安全解链**: 在解引用之前验证指针

#### Use-After-Free Mitigation / 释放后重用缓解

**Tcmalloc Integration**: Tcmalloc provides several protections against use-after-free vulnerabilities:

**Tcmalloc集成**: Tcmalloc提供针对释放后重用漏洞的几种保护：

1. **Quarantine**: Freed chunks are placed in a quarantine list before reuse
   **隔离**: 释放的块在重用之前被放置在隔离列表中
2. **Delayed Reuse**: Memory is not immediately reused, extending the vulnerability window
   **延迟重用**: 内存不会立即重用，延长了漏洞窗口
3. **Pattern Filling**: Freed memory is filled with known patterns (0x6A, 0xA6, etc.)
   **模式填充**: 释放的内存用已知模式填充（0x6A、0xA6等）
4. **Alloc-After-Free Detection**: Detects allocations to recently freed addresses
   **释放后分配检测**: 检测到最近释放地址的分配

#### Heap Metadata Hardening / 堆元数据强化

**Chunk Headers**: Each heap chunk includes hardened metadata:

**块头**: 每个堆块包括强化的元数据：

```rust
struct HeapChunkHeader {
    size: usize,              // Obfuscated with random XOR mask
    prev_size: usize,         // Obfuscated with random XOR mask
    fd: *mut HeapChunk,       // XOR-encoded forward pointer
    bk: *mut HeapChunk,       // XOR-encoded backward pointer
    checksum: u64,            // Metadata integrity checksum
    canary: u64,              // Stack canary-style protection
}
```

**Double-Free Detection**: Each chunk includes a magic number that is checked on free to prevent double-free attacks:

**双重释放检测**: 每个块包含一个魔数，在释放时检查以防止双重释放攻击：

```rust
const CHUNK_MAGIC: u64 = 0xABADC0DEDEADBEEF;

fn validate_chunk(chunk: &HeapChunk) -> bool {
    chunk.magic == CHUNK_MAGIC && !chunk.is_quarantined
}
```

---

### Fortify Source / 源码强化

#### _FORTIFY_SOURCE Implementation

`_FORTIFY_SOURCE` is a macro that adds buffer overflow checking to commonly used memory functions. When the compiler can determine the object size at compile time, it replaces unsafe functions with size-checked variants.

`_FORTIFY_SOURCE`是一个宏，向常用内存函数添加缓冲区溢出检查。当编译器可以在编译时确定对象大小时，它用经过大小检查的变体替换不安全的函数。

**Supported Functions / 支持的函数:**
- `memcpy`, `memmove`, `memset`
- `strcpy`, `strncpy`, `strcat`, `strncat`
- `sprintf`, `snprintf`, `vsprintf`, `vsnprintf`
- `fgets`, `read`

**Compile-time Checks / 编译时检查:**

When object size is known, the compiler adds checks:

当对象大小已知时，编译器添加检查：

```c
// Example: memcpy with fortification
#define memcpy(dest, src, len) \
    (__builtin_object_size(dest, 0) != (size_t)-1 && \
     len > __builtin_object_size(dest, 0) ? \
     __chk_fail() : \
     __builtin_memcpy(dest, src, len))
```

**Runtime Behavior / 运行时行为:**

1. **If size is known at compile time**: Compile-time checks are added
   **如果在编译时大小已知**: 添加编译时检查
2. **If size cannot be determined**: Falls back to standard library function
   **如果无法确定大小**: 回退到标准库函数
3. **On overflow detection**: Calls `__chk_fail()`, which terminates the process
   **检测到溢出时**: 调用`__chk_fail()`，终止进程

---

## Access Control / 访问控制

### DAC (Discretionary Access Control) / 自主访问控制

#### Traditional Unix Permissions / 传统Unix权限

The foundation of our access control system is traditional Unix file permissions:

我们访问控制系统的基础是传统Unix文件权限：

**Permission Bits / 权限位:**
```
-rwxrwxrwx
 |||| |||| |
 |||| |||| +-- Others: read, write, execute
 |||| ++++----- Group: read, write, execute
 ++++--------- Owner: read, write, execute
```

**Representation / 表示:**
```rust
pub struct FileMode {
    pub user_read: bool,
    pub user_write: bool,
    pub user_execute: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_execute: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_execute: bool,
}
```

#### Permission Checking / 权限检查

Permission checks are performed at file access time using the following algorithm:

权限检查在文件访问时使用以下算法执行：

```rust
pub fn check_permission(
    inode: &Inode,
    credentials: &Credentials,
    desired_permission: Permission
) -> bool {
    // Check if user is owner
    if inode.uid == credentials.uid {
        return inode.mode.user_permissions.contains(desired_permission);
    }

    // Check if user is in group
    if inode.gid == credentials.gid || credentials.groups.contains(&inode.gid) {
        return inode.mode.group_permissions.contains(desired_permission);
    }

    // Fall back to other permissions
    inode.mode.other_permissions.contains(desired_permission)
}
```

#### Umask / 文件创建掩码

The umask controls default permissions for newly created files:

umask控制新创建文件的默认权限：

**Default umask**: `022` (rw-r--r--)
**默认umask**: `022` (rw-r--r--)

**Calculation / 计算:**
```rust
fn apply_umask(base_mode: FileMode, umask: u32) -> FileMode {
    FileMode {
        user_write: base_mode.user_write && (umask & 0200) == 0,
        user_read: base_mode.user_read && (umask & 0400) == 0,
        user_execute: base_mode.user_execute && (umask & 0100) == 0,
        group_write: base_mode.group_write && (umask & 0020) == 0,
        group_read: base_mode.group_read && (umask & 0040) == 0,
        group_execute: base_mode.group_execute && (umask & 0010) == 0,
        other_write: base_mode.other_write && (umask & 0002) == 0,
        other_read: base_mode.other_read && (umask & 0004) == 0,
        other_execute: base_mode.other_execute && (umask & 0001) == 0,
    }
}
```

---

### ACL (Access Control Lists) / 访问控制列表

#### Extended ACLs / 扩展ACL

Extended ACLs provide fine-grained access control beyond traditional Unix permissions. They allow specifying permissions for multiple users and groups.

扩展ACL提供传统Unix权限之外的细粒度访问控制。它们允许为多个用户和组指定权限。

**ACL Entry Structure / ACL条目结构:**
```rust
pub struct AclEntry {
    pub tag: AclTag,          // ACL_USER, ACL_GROUP, ACL_MASK, etc.
    pub qualifier: u32,       // UID or GID
    pub permissions: AclPermissions,
}

pub struct AclPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}
```

**ACL Types / ACL类型:**
1. **ACL_USER_OBJ**: Owner permissions (equivalent to Unix user bits)
   **ACL_USER_OBJ**: 所有者权限（等同于Unix用户位）
2. **ACL_USER**: Additional user permissions
   **ACL_USER**: 附加用户权限
3. **ACL_GROUP_OBJ**: Owning group permissions (equivalent to Unix group bits)
   **ACL_GROUP_OBJ**: 所属组权限（等同于Unix组位）
4. **ACL_GROUP**: Additional group permissions
   **ACL_GROUP**: 附加组权限
5. **ACL_MASK**: Maximum permissions for additional users/groups
   **ACL_MASK**: 附加用户/组的最大权限
6. **ACL_OTHER**: Permissions for everyone else (equivalent to Unix other bits)
   **ACL_OTHER**: 其他人的权限（等同于Unix其他位）

#### NFSv4 ACLs / NFSv4访问控制列表

NFSv4 ACLs provide a more sophisticated ACL model with inheritance and denial entries:

NFSv4 ACL提供更复杂的ACL模型，具有继承和拒绝条目：

**Features / 特性:**
1. **Allow and Deny Entries**: Explicit allow and deny rules
   **允许和拒绝条目**: 显式允许和拒绝规则
2. **Inheritance Flags**: Control how ACLs propagate to child objects
   **继承标志**: 控制ACL如何传播到子对象
3. **ACE Ordering**: Rules are evaluated in order
   **ACE排序**: 规则按顺序评估

```rust
pub struct NfsV4AclEntry {
    pub typ: NfsV4AclType,        // ALLOW or DENY
    pub who: NfsV4Who,            // User, group, or everyone
    pub flags: NfsV4Flags,        // Inheritance flags
    pub access_mask: NfsV4Mask,   // Read, write, execute, etc.
}

pub struct NfsV4Flags {
    pub file_inherit: bool,       // Inherit to files
    pub directory_inherit: bool,  // Inherit to directories
    pub no_propagate: bool,       // Don't propagate to grandchildren
    pub inherit_only: bool,       // Only applies to inherited entries
}
```

#### ACL Inheritance / ACL继承

ACL inheritance controls how permissions propagate to new files and directories:

ACL继承控制权限如何传播到新文件和目录：

**Inheritance Rules / 继承规则:**
1. **Default ACL**: Applied to new child objects
   **默认ACL**: 应用于新的子对象
2. **File Inherit**: ACL applies to files in this directory
   **文件继承**: ACL应用于此目录中的文件
3. **Directory Inherit**: ACL applies to subdirectories
   **目录继承**: ACL应用于子目录
4. **Inherit Only**: ACL only applies to inherited entries, not the directory itself
   **仅继承**: ACL仅适用于继承的条目，而不适用于目录本身

```rust
pub fn inherit_acl(
    parent_acl: &Acl,
    child_type: ObjectType,
    child_name: &str
) -> Acl {
    let mut inherited = Acl::new();

    for entry in &parent_acl.entries {
        if entry.flags.inherit_only && !entry.is_inherited() {
            continue;
        }

        match child_type {
            ObjectType::File if entry.flags.file_inherit => {
                inherited.add_entry(entry.clone());
            },
            ObjectType::Directory if entry.flags.directory_inherit => {
                inherited.add_entry(entry.clone());
            },
            _ => {},
        }
    }

    inherited
}
```

---

### Capabilities / 能力

#### POSIX Capabilities / POSIX能力

Capabilities provide fine-grained privilege separation by breaking root privileges into distinct units. A process can have specific capabilities without requiring full root access.

能力通过将root权限分解为不同的单元来提供细粒度的权限分离。进程可以具有特定的能力，而无需完全的root访问权限。

**Capability Set Types / 能力集类型:**
1. **Permitted**: Capabilities the process is allowed to use
   **允许**: 进程被允许使用的能力
2. **Inheritable**: Capabilities inherited across execve
   **可继承**: 跨execve继承的能力
3. **Effective**: Capabilities currently in use (checked by kernel)
   **有效**: 当前正在使用的能力（由内核检查）
4. **Bounding**: Restricts capabilities that can be gained
   **边界**: 限制可以获得的能力
5. **Ambient**: Inheritable capabilities that stay effective across execve
   **环境**: 跨execve保持有效的可继承能力

**Key Capabilities / 关键能力:**
```rust
pub enum Capability {
    CAP_CHOWN,           // Change file ownership
    CAP_DAC_OVERRIDE,    // Bypass file read/write/execute permission checks
    CAP_DAC_READ_SEARCH, // Bypass file read permission checks and directory read/search
    CAP_FOWNER,          // Bypass permission checks on operations that normally require file owner
    CAP_FSETID,          // Don't clear set-user-ID/set-group-ID mode bits
    CAP_KILL,            // Bypass permission checks for sending signals
    CAP_SETGID,          // Set group ID
    CAP_SETUID,          // Set user ID
    CAP_SETPCAP,         // Modify capabilities
    CAP_NET_BIND_SERVICE, // Bind to privileged ports (< 1024)
    CAP_NET_ADMIN,       // Perform various network-related operations
    CAP_NET_RAW,         // Use RAW and PACKET sockets
    CAP_SYS_ADMIN,       // Perform system administration tasks
    CAP_SYS_BOOT,        // Use reboot
    CAP_SYS_CHROOT,      // Use chroot
    CAP_SYS_MODULE,      // Load and unload kernel modules
    CAP_SYS_TIME,        // Set system clock
    // ... and many more
}
```

#### Capability Bounding Sets / 能力边界集

The bounding set is a security mechanism that restricts which capabilities a process can ever gain, even if it gains privileges:

边界集是一种安全机制，限制进程可以获得哪些能力，即使它获得了权限：

```rust
fn drop_capability_from_bounding(cap: Capability) -> Result<()> {
    let current = get_bounding_set();

    if !current.contains(&cap) {
        return Ok(()); // Already dropped
    }

    // Once dropped, capabilities cannot be added back to bounding set
    prctl(PR_CAPBSET_DROP, cap as u64, 0, 0, 0)?;

    Ok(())
}
```

#### Ambient Capabilities / 环境能力

Ambient capabilities allow preserving specific capabilities across execve without set-user-ID binaries:

环境能力允许在execve之间保留特定能力，而无需set-user-ID二进制文件：

```rust
fn set_ambient_capability(cap: Capability) -> Result<()> {
    // Must be in permitted and inheritable sets
    if !has_capability(cap, CapabilitySet::Permitted) ||
       !has_capability(cap, CapabilitySet::Inheritable) {
        return Err(Error::PermissionDenied);
    }

    prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, cap as u64, 0, 0)?;

    Ok(())
}
```

---

### MAC (Mandatory Access Control) / 强制访问控制

#### SELinux Integration / SELinux集成

SELinux (Security-Enhanced Linux) provides mandatory access control using Type Enforcement:

SELinux（安全增强Linux）使用类型强制提供强制访问控制：

**Core Concepts / 核心概念:**
1. **Subjects**: Processes (domains)
   **主体**: 进程（域）
2. **Objects**: Files, sockets, etc. (types)
   **对象**: 文件、套接字等（类型）
3. **Rules**: Allow/deny transitions between domains and accesses to types
   **规则**: 允许/拒绝域之间的转换和对类型的访问

**Policy Structure / 策略结构:**
```
# Type definitions
type httpd_t;
type httpd_exec_t;
type httpd_log_t;
type httpd_content_t;

# Type transitions
type_transition httpd_exec_t httpd_t;

# Allow rules
allow httpd_t httpd_content_t:file { read open getattr };
allow httpd_t httpd_log_t:file { write create open append };
```

**Implementation / 实现:**
```rust
pub struct SelinuxContext {
    pub user: String,      // SELinux user
    pub role: String,      // SELinux role
    pub type_: String,     // SELinux type (domain for processes)
    pub level: String,     // MLS level (Multi-Level Security)
}

pub fn selinux_check_permission(
    subject: &SelinuxContext,
    object: &SelinuxContext,
    object_class: ObjectClass,
    permission: Permission
) -> bool {
    // Query SELinux policy database
    policy_db.allow(
        &subject.type_,
        &object.type_,
        object_class,
        permission
    )
}
```

#### AppArmor Profiles / AppArmor配置文件

AppArmor provides MAC based on file paths, making it easier to configure than SELinux:

AppArmor提供基于文件路径的MAC，使其比SELinux更容易配置：

**Profile Example / 配置文件示例:**
```
# /etc/apparmor.d/usr.sbin.nginx
profile nginx /usr/sbin/nginx {
    # Include base rules
    #include <abstractions/base>
    #include <abstractions/nginx>

    # Allow access to configuration
    /etc/nginx/** r,
    /etc/nginx/*.conf r,

    # Allow access to web content
    /var/www/** r,
    /var/www/html/** r,

    # Allow logging
    /var/log/nginx/ w,
    /var/log/nginx/** w,

    # Deny access to sensitive files
    deny /etc/shadow rwx,
    deny /root/** rwx,

    # Network access
    network inet stream,
    network inet6 stream,

    # Capabilities
    capability setuid,
    capability setgid,
    capability net_bind_service,
}
```

**Implementation / 实现:**
```rust
pub struct AppArmorProfile {
    pub name: String,
    pub rules: Vec<AppArmorRule>,
}

pub enum AppArmorRule {
    PathRule {
        path: String,
        permissions: FileAccess,  // r, w, x, m, etc.
        allow: bool,
    },
    NetworkRule {
        family: AddressFamily,
        sock_type: SocketType,
        allow: bool,
    },
    CapabilityRule {
        capability: Capability,
        allow: bool,
    },
}

pub fn apparmor_check(
    profile: &AppArmorProfile,
    operation: &Operation
) -> bool {
    match operation {
        Operation::FileAccess { path, perm } => {
            profile.rules.iter().any(|rule| {
                matches!(rule, AppArmorRule::PathRule { path: p, permissions, allow }
                    if path_matches(p, path) && permissions.contains(perm) && *allow)
            })
        },
        Operation::Network { family, sock_type } => {
            // Check network rules
            true
        },
        Operation::Capability(cap) => {
            // Check capability rules
            true
        },
    }
}
```

#### Smack Policies / Smack策略

Smack (Simplified Mandatory Access Control Kernel) provides a simpler MAC model based on labels:

Smack（简化的强制访问控制内核）提供基于标签的更简单的MAC模型：

**Label-Based Access Control / 基于标签的访问控制:**
```
# Subjects and objects have labels
# "admin" can read/write anything with label "*"
# "user" can read objects with label "shared" and write to own label
# "private" data can only be accessed by processes with same label

# Rules
admin * rwx
user shared r
user user rw
private private rw
```

---

## Sandboxing / 沙箱隔离

### Seccomp / 安全计算

#### System Call Filtering / 系统调用过滤

Seccomp (Secure Computing Mode) allows processes to restrict the system calls they can make. This reduces the kernel attack surface by limiting available syscalls.

Seccomp（安全计算模式）允许进程限制它们可以进行的系统调用。这通过限制可用的系统调用来减少内核攻击面。

**Seccomp Modes / Seccomp模式:**
1. **Strict Mode (Mode 1)**: Only allow `read`, `write`, `_exit`, and `sigreturn`
   **严格模式（模式1）**: 只允许`read`、`write`、`_exit`和`sigreturn`
2. **Filter Mode (Mode 2)**: Allow filtering syscalls using BPF programs
   **过滤模式（模式2）**: 允许使用BPF程序过滤系统调用

#### BPF Filter Programs / BPF过滤器程序

Seccomp filters are written in a restricted BPF (Berkeley Packet Filter) language:

Seccomp过滤器使用受限的BPF（伯克利数据包过滤器）语言编写：

```c
// Example seccomp filter in C
#include <linux/seccomp.h>
#include <linux/filter.h>

struct sock_filter filter_code[] = {
    // Load system call number
    BPF_STMT(BPF_LD + BPF_W + BPF_ABS, offsetof(struct seccomp_data, nr)),

    // Allow write, read, exit, sigreturn
    BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_write, 1, 0),
    BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
    BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_read, 1, 0),
    BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
    BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_exit, 1, 0),
    BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
    BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_rt_sigreturn, 1, 0),
    BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),

    // Kill process for all other syscalls
    BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_KILL_PROCESS),
};

struct sock_fprog prog = {
    .len = sizeof(filter_code) / sizeof(filter_code[0]),
    .filter = filter_code,
};

prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog);
```

**Rust Implementation / Rust实现:**
```rust
pub struct SeccompFilter {
    pub rules: Vec<SeccompRule>,
}

pub struct SeccompRule {
    pub syscall: SyscallNumber,
    pub action: SeccompAction,
    pub arg_filters: Vec<ArgFilter>,
}

pub enum SeccompAction {
    Allow,
    KillProcess,
    KillThread,
    Trap,
    Errno(u32),
    Trace(u32),
    Log,
}

pub fn apply_seccomp_filter(filter: &SeccompFilter) -> Result<()> {
    let bpf_program = compile_bpf_filter(filter)?;

    unsafe {
        libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &bpf_program);
    }

    Ok(())
}
```

---

### Namespaces / 命名空间

Namespaces provide process isolation by creating separate views of system resources:

命名空间通过创建系统资源的独立视图来提供进程隔离：

#### Process Isolation (PID Namespace) / 进程隔离

**Features / 特性:**
- Each namespace has its own PID space
  每个命名空间都有自己的PID空间
- Processes in child namespaces have multiple PIDs
  子命名空间中的进程有多个PID
- `init` process (PID 1) in each namespace
  每个命名空间中的`init`进程（PID 1）

**Implementation / 实现:**
```rust
pub fn create_pid_namespace() -> Result<PidNamespace> {
    unsafe {
        let res = libc::unshare(libc::CLONE_NEWPID);
        if res == 0 {
            Ok(PidNamespace::new())
        } else {
            Err(Error::last_os_error())
        }
    }
}

pub fn fork_in_namespace() -> Result<u32> {
    let pid = unsafe { libc::fork() };

    if pid == 0 {
        // Child process in new namespace
        Ok(0)
    } else {
        // Parent process
        Ok(pid as u32)
    }
}
```

#### Network Isolation (Network Namespace) / 网络隔离

**Features / 特性:**
- Separate network stack per namespace
  每个命名空间独立的网络栈
- Isolated routing tables, firewall rules, and sockets
  隔离的路由表、防火墙规则和套接字
- Virtual Ethernet pairs for inter-namespace communication
  用于命名空间间通信的虚拟以太网对

**Implementation / 实现:**
```rust
pub fn create_network_namespace() -> Result<NetworkNamespace> {
    unsafe {
        let res = libc::unshare(libc::CLONE_NEWNET);
        if res == 0 {
            Ok(NetworkNamespace::new())
        } else {
            Err(Error::last_os_error())
        }
    }
}

pub fn create_veth_pair(
    name1: &str,
    name2: &str,
    namespace1: &NetworkNamespace,
    namespace2: &NetworkNamespace
) -> Result<()> {
    // Create virtual Ethernet pair
    netlink::create_veth(name1, name2)?;

    // Move one end to each namespace
    netlink::set_interface_namespace(name1, namespace1)?;
    netlink::set_interface_namespace(name2, namespace2)?;

    Ok(())
}
```

#### Mount Namespaces / 挂载命名空间

**Features / 特性:**
- Separate filesystem mount points
  独立的文件系统挂载点
- Chroot-like isolation without the limitations
  类似chroot的隔离，但没有限制
- Propagation flags (shared, private, slave, unbindable)
  传播标志（共享、私有、从属、不可绑定）

**Implementation / 实现:**
```rust
pub fn create_mount_namespace() -> Result<MountNamespace> {
    unsafe {
        let res = libc::unshare(libc::CLONE_NEWNS);
        if res == 0 {
            Ok(MountNamespace::new())
        } else {
            Err(Error::last_os_error())
        }
    }
}

pub fn pivot_root(new_root: &str, put_old: &str) -> Result<()> {
    unsafe {
        // Bind mount new root to itself
        libc::mount(
            new_root.as_ptr(),
            new_root.as_ptr(),
            std::ptr::null(),
            libc::MS_BIND | libc::MS_REC,
            std::ptr::null()
        );

        // Create put_old directory
        libc::mkdir(put_old.as_ptr(), 0o700);

        // Pivot root
        libc::syscall(
            libc::SYS_pivot_root,
            new_root.as_ptr(),
            put_old.as_ptr()
        );

        // Change to new root
        libc::chdir("/");

        // Unmount old root
        libc::umount2(put_old, libc::MNT_DETACH);

        Ok(())
    }
}
```

#### User Namespaces / 用户命名空间

**Features / 特性:**
- Allow unprivileged processes to have root privileges within namespace
  允许非特权进程在命名空间内拥有root权限
- UID/GID mapping between namespaces
  命名空间之间的UID/GID映射
- Capability isolation
  能力隔离

**Implementation / 实现:**
```rust
pub struct UserNamespace {
    pub uid_map: Vec<UidMapping>,
    pub gid_map: Vec<GidMapping>,
}

pub struct UidMapping {
    pub inside_uid: u32,
    pub outside_uid: u32,
    pub count: u32,
}

pub fn create_user_namespace() -> Result<UserNamespace> {
    unsafe {
        let res = libc::unshare(libc::CLONE_NEWUSER);
        if res == 0 {
            let mut ns = UserNamespace::new();

            // Map current user to root in namespace
            ns.set_uid_map(&[
                UidMapping {
                    inside_uid: 0,
                    outside_uid: libc::getuid(),
                    count: 1,
                }
            ])?;

            Ok(ns)
        } else {
            Err(Error::last_os_error())
        }
    }
}

pub fn set_uid_map(map: &[UidMapping]) -> Result<()> {
    let path = "/proc/self/uid_map";
    let mut file = File::open(path)?;

    for mapping in map {
        writeln!(
            file,
            "{} {} {}",
            mapping.inside_uid, mapping.outside_uid, mapping.count
        )?;
    }

    Ok(())
}
```

---

### Cgroups / 控制组

#### Resource Limiting / 资源限制

Cgroups (Control Groups) limit and account resource usage for process groups:

Cgroups（控制组）限制和核算进程组的资源使用：

**Resource Controllers / 资源控制器:**
1. **cpu**: CPU scheduling and usage limits
   **cpu**: CPU调度和使用限制
2. **memory**: Memory usage limits and accounting
   **memory**: 内存使用限制和核算
3. **io**: I/O bandwidth limits
   **io**: I/O带宽限制
4. **blkio**: Block device I/O limits
   **blkio**: 块设备I/O限制
5. **devices**: Device access whitelist
   **devices**: 设备访问白名单
6. **cpuset**: CPU and memory node assignment
   **cpuset**: CPU和内存节点分配

**Memory Limits / 内存限制:**
```rust
pub struct MemoryCgroup {
    pub limit_in_bytes: usize,
    pub soft_limit_in_bytes: usize,
    pub swap_limit_in_bytes: usize,
    pub swappiness: u32,
}

pub fn create_memory_cgroup(name: &str, limit: usize) -> Result<MemoryCgroup> {
    let cgroup_path = format!("/sys/fs/cgroup/memory/{}", name);

    // Create cgroup directory
    fs::create_dir(&cgroup_path)?;

    // Set memory limit
    let limit_file = format!("{}/memory.limit_in_bytes", cgroup_path);
    fs::write(&limit_file, limit.to_string())?;

    // Set soft limit
    let soft_limit_file = format!("{}/memory.soft_limit_in_bytes", cgroup_path);
    fs::write(&soft_limit_file, (limit * 90 / 100).to_string())?;

    Ok(MemoryCgroup {
        limit_in_bytes: limit,
        soft_limit_in_bytes: limit * 90 / 100,
        swap_limit_in_bytes: limit,
        swappiness: 60,
    })
}

pub fn add_pid_to_cgroup(cgroup: &str, pid: u32) -> Result<()> {
    let tasks_file = format!("/sys/fs/cgroup/memory/{}/tasks", cgroup);
    fs::write(&tasks_file, pid.to_string())?;
    Ok(())
}
```

#### Device Whitelisting / 设备白名单

The devices controller restricts which devices a cgroup can access:

devices控制器限制cgroup可以访问的设备：

```rust
pub struct DeviceRule {
    pub device_type: DeviceType,  // 'a' (all), 'b' (block), 'c' (char)
    pub major: u32,               // Major number (or '*' for all)
    pub minor: u32,               // Minor number (or '*' for all)
    pub access: DeviceAccess,     // 'r' (read), 'w' (write), 'm' (mknod)
}

pub enum DeviceAccess {
    Read,
    Write,
    ReadWrite,
    Mknod,
}

pub fn add_device_rule(cgroup: &str, rule: DeviceRule) -> Result<()> {
    let devices_file = format!("/sys/fs/cgroup/devices/{}/devices.allow", cgroup);

    let rule_string = format!(
        "{} {}:{} {}",
        rule.device_type as char,
        rule.major,
        rule.minor,
        match rule.access {
            DeviceAccess::Read => "r",
            DeviceAccess::Write => "w",
            DeviceAccess::ReadWrite => "rw",
            DeviceAccess::Mknod => "m",
        }
    );

    fs::write(&devices_file, rule_string)?;

    Ok(())
}

// Example: Allow /dev/null access
add_device_rule(
    "my_container",
    DeviceRule {
        device_type: DeviceType::Char,
        major: 1,
        minor: 3,
        access: DeviceAccess::ReadWrite,
    }
)?;
```

---

## Runtime Protections / 运行时保护

### SMEP/SMAP (x86_64) / x86_64保护

#### Supervisor Mode Execution Prevention (SMEP) / 特权模式执行防止

SMEP prevents the CPU from executing code located in user-mode pages while in supervisor mode (kernel mode). This protects against kernel privilege escalation exploits that attempt to jump to user-space shellcode.

SMEP防止CPU在特权模式（内核模式）时执行位于用户模式页面的代码。这可以防止试图跳转到用户空间shellcode的内核权限提升漏洞利用。

**Hardware Support / 硬件支持:**
- Intel: Ivy Bridge (2012) and later
  Intel: Ivy Bridge（2012）及以后
- AMD: Not directly supported (equivalent via SMEPP)
  AMD: 不直接支持（通过SMEPP等效支持）

**Enable in Kernel / 在内核中启用:**
```rust
#[cfg(target_arch = "x86_64")]
pub fn enable_smep() {
    unsafe {
        let mut cr4: u64;
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
        );

        // Set bit 20 (SMEP)
        cr4 |= 1 << 20;

        core::arch::asm!(
            "mov cr4, {}",
            in(reg) cr4,
        );
    }
}
```

#### Supervisor Mode Access Prevention (SMAP) / 特权模式访问防止

SMAP prevents the CPU from accessing user-mode pages while in supervisor mode, except when explicitly overridden. This provides even stronger protection than SMEP.

SMAP防止CPU在特权模式时访问用户模式页面，除非显式覆盖。这提供了比SMEP更强的保护。

**Enable in Kernel / 在内核中启用:**
```rust
#[cfg(target_arch = "x86_64")]
pub fn enable_smap() {
    unsafe {
        let mut cr4: u64;
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
        );

        // Set bit 21 (SMAP)
        cr4 |= 1 << 21;

        core::arch::asm!(
            "mov cr4, {}",
            in(reg) cr4,
        );
    }
}

// Temporarily disable SMAP to access user memory
pub unsafe fn stac() {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("stac");
}

// Re-enable SMAP
pub unsafe fn clac() {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("clac");
}

// Usage example
pub fn copy_from_user(user_ptr: *const u8, kernel_buf: &mut [u8]) -> Result<usize> {
    unsafe {
        stac();  // Disable SMAP

        let mut copied = 0;
        for i in 0..kernel_buf.len() {
            kernel_buf[i] = *user_ptr.add(i);
            copied += 1;
        }

        clac();  // Re-enable SMAP

        Ok(copied)
    }
}
```

---

### NX Bit / 不可执行位

#### No-Execute Pages / 不可执行页面

The NX (No-Execute) bit marks memory pages as non-executable, preventing code execution in data sections (stack, heap, etc.).

NX（不可执行）位将内存页面标记为不可执行，防止在数据段（栈、堆等）中执行代码。

**Implementation / 实现:**
```rust
pub fn set_nx_bit(page_table_entry: &mut PageTableEntry) {
    page_table_entry.flags &= PageTableFlags::NO_EXECUTE;
}

pub fn mmap_with_nx(
    addr: VirtAddr,
    size: usize,
    prot: ProtectionFlags
) -> Result<VirtAddr> {
    let mut flags = MappingFlags::empty();

    if prot.contains(ProtectionFlags::READ) {
        flags |= MappingFlags::READ;
    }
    if prot.contains(ProtectionFlags::WRITE) {
        flags |= MappingFlags::WRITE;
    }
    // Never set EXECUTE flag by default
    // NX is enforced by default

    create_mapping(addr, size, flags)
}
```

#### W^X (Write XOR Execute) / 写异或执行

The W^X principle ensures that memory is either writable OR executable, but never both simultaneously:

W^X原则确保内存可写或可执行，但绝不能同时两者：

```rust
pub fn validate_protection_flags(prot: ProtectionFlags) -> Result<()> {
    let has_write = prot.contains(ProtectionFlags::WRITE);
    let has_execute = prot.contains(ProtectionFlags::EXECUTE);

    if has_write && has_execute {
        return Err(Error::InvalidFlags);
    }

    Ok(())
}

pub fn mmap_with_wx_enforcement(
    addr: VirtAddr,
    size: usize,
    prot: ProtectionFlags
) -> Result<VirtAddr> {
    // Enforce W^X
    validate_protection_flags(prot)?;

    // Create mapping with validated flags
    create_mapping(addr, size, prot.into())
}
```

---

### Page Table Isolation (PTI) / 页表隔离

#### KPTI (Kernel Page Table Isolation) / 内核页表隔离

KPTI mitigates the Meltdown vulnerability by separating kernel and user page tables. This prevents user-space from reading kernel memory.

KPTI通过分离内核和用户页表来缓解Meltdown漏洞。这可以防止用户空间读取内核内存。

**Implementation / 实现:**
```rust
pub struct PageTables {
    pub user_table: PageTable,
    pub kernel_table: PageTable,
}

pub fn switch_to_user_table() {
    unsafe {
        // Load user CR3
        let user_cr3 = get_user_cr3();
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) user_cr3,
        );
    }
}

pub fn switch_to_kernel_table() {
    unsafe {
        // Load kernel CR3
        let kernel_cr3 = get_kernel_cr3();
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) kernel_cr3,
        );
    }
}

// System call entry
pub fn syscall_entry() {
    switch_to_kernel_table();

    // Handle syscall
    // ...

    switch_to_user_table();
}
```

**Performance Impact / 性能影响:**
- **Overhead**: ~5-10% due to page table switches on syscall/return
  **开销**: 由于系统调用/返回时的页表切换，约5-10%
- **Mitigation**: PCID (Process-Context Identifiers) reduce overhead to ~2-3%
  **缓解**: PCID（进程上下文标识符）将开销减少到约2-3%

---

### CFI (Control Flow Integrity) / 控制流完整性

#### Forward-Edge CFI / 前向边缘CFI

Forward-edge CFI validates indirect branches (function pointers, virtual calls) to ensure they target valid function entries:

前向边缘CFI验证间接分支（函数指针、虚调用）以确保它们以有效的函数入口为目标：

```rust
#[cfg(feature = "cfi")]
pub fn validate_function_pointer(ptr: *const u8) -> bool {
    // Check if pointer points to valid function entry
    // (starts with function prologue)

    let bytes = unsafe { core::slice::from_raw_parts(ptr, 3) };

    // Common function prologues:
    // x86_64: endbr64 (0xF3 0x0F 0x1E 0xFA) with Intel CET
    //        push rbp (0x55)
    //        mov rdi, rdi (0x48 0x89 0xF8)

    // Check for endbr64 (Intel CET)
    if bytes[0] == 0xF3 && bytes[1] == 0x0F && bytes[2] == 0x1E {
        return true;
    }

    // Check for push rbp
    if bytes[0] == 0x55 {
        return true;
    }

    false
}

// Macro to generate CFI-protected function calls
#[macro_export]
macro_rules! cfi_call {
    ($func_ptr:expr, $($args:expr),*) => {
        {
            let func = $func_ptr;
            if !$crate::security::cfi::validate_function_pointer(func as *const u8) {
                panic!("CFI violation: invalid function pointer");
            }
            func($($args),*)
        }
    };
}
```

#### Shadow Stack (CET) / 影子栈

Shadow Stack (part of Intel CET) maintains a separate, protected stack for return addresses, preventing ROP (Return-Oriented Programming) attacks:

影子栈（Intel CET的一部分）维护一个独立的、受保护的返回地址栈，防止ROP（面向返回编程）攻击：

```rust
#[cfg(target_arch = "x86_64")]
pub struct ShadowStack {
    base: VirtAddr,
    top: VirtAddr,
}

#[cfg(target_arch = "x86_64")]
impl ShadowStack {
    pub fn new(size: usize) -> Result<Self> {
        // Allocate shadow stack memory
        let base = allocate_shadow_stack(size)?;

        // Enable shadow stack in CPU
        unsafe {
            let mut ssp: u64;
            core::arch::asm!(
                "rdsspq {}",
                out(reg) ssp,
            );

            Ok(Self {
                base: VirtAddr::new(ssp as usize),
                top: VirtAddr::new(ssp as usize + size),
            })
        }
    }

    pub fn push_return_address(&mut self, ret_addr: u64) {
        unsafe {
            core::arch::asm!(
                "incsspq %rax",  // Increment shadow stack pointer
                "saveprevssp",    // Save previous SSP
                in("rax") self.base.as_u64(),
            );
        }
    }
}
```

#### Indirect Branch Tracking (IBT) / 间接分支跟踪

IBT validates indirect branch targets to ensure they land on valid branch targets:

IBT验证间接分支目标以确保它们落在有效的分支目标上：

```rust
pub enum BranchTarget {
    FunctionEntry(*const u8),
    IndirectBranch(*const u8),
    Invalid,
}

pub fn validate_indirect_branch(target: *const u8) -> bool {
    let bytes = unsafe { core::slice::from_raw_parts(target, 4) };

    // Check for ENDBR instruction (Intel CET)
    // endbr64 for 64-bit: 0xF3 0x0F 0x1E 0xFA
    // endbr32 for 32-bit: 0xF3 0x0F 0x1E 0xFB

    matches!(bytes, [0xF3, 0x0F, 0x1E, 0xFA] | [0xF3, 0x0F, 0x1E, 0xFB])
}

// Compiler-generated indirect branch with IBT
pub fn indirect_call_with_ibt(func_ptr: *const u8, args: &[u64]) -> u64 {
    if !validate_indirect_branch(func_ptr) {
        // Control protection exception (#CP)
        unsafe {
            core::arch::asm!("ud2");  // Trigger undefined instruction
        }
    }

    unsafe {
        let func: extern "C" fn(&[u64]) -> u64 = core::mem::transmute(func_ptr);
        func(args)
    }
}
```

---

## Audit and Monitoring / 审计与监控

### Audit Framework / 审计框架

#### Event Logging / 事件记录

The audit framework logs security-relevant events for compliance and forensic analysis:

审计框架记录安全相关事件以进行合规性和取证分析：

```rust
pub struct AuditEvent {
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
    pub success: bool,
    pub details: HashMap<String, String>,
}

pub enum AuditEventType {
    Syscall,
    FileAccess,
    ProcessCreation,
    ProcessTermination,
    NetworkConnection,
    PrivilegeChange,
    SecurityViolation,
}

pub fn log_audit_event(event: AuditEvent) {
    let audit_log = get_audit_log();

    // Format event as string
    let event_str = format!(
        "type={} timestamp={} pid={} uid={} gid={} success={} details={:?}",
        event.event_type,
        event.timestamp,
        event.pid,
        event.uid,
        event.gid,
        event.success,
        event.details
    );

    // Write to audit log
    audit_log.write(&event_str);

    // Send to monitoring system
    if is_security_critical(&event) {
        send_alert(&event);
    }
}
```

#### Audit Rules / 审计规则

Audit rules control which events are logged:

审计规则控制哪些事件被记录：

```rust
pub struct AuditRule {
    pub filter: AuditFilter,
    pub action: AuditAction,
}

pub enum AuditFilter {
    SyscallFilter {
        syscall_numbers: Vec<u32>,
    },
    PathFilter {
        path: String,
        operations: Vec<FileOperation>,
    },
    UserFilter {
        uids: Vec<u32>,
    },
    ProgramFilter {
        executable: String,
    },
}

pub enum AuditAction {
    Log,
    LogAndDeny,
    Ignore,
}

pub fn add_audit_rule(rule: AuditRule) -> Result<()> {
    let audit_subsystem = get_audit_subsystem();

    match rule.filter {
        AuditFilter::SyscallFilter { syscall_numbers } => {
            for syscall in syscall_numbers {
                audit_subsystem.watch_syscall(syscall, rule.action);
            }
        },
        AuditFilter::PathFilter { path, operations } => {
            audit_subsystem.watch_path(&path, &operations, rule.action);
        },
        _ => {},
    }

    Ok(())
}
```

#### Compliance Reporting / 合规报告

The audit system generates compliance reports for standards like PCI-DSS, HIPAA, and GDPR:

审计系统为PCI-DSS、HIPAA和GDPR等标准生成合规报告：

```rust
pub struct ComplianceReport {
    pub standard: ComplianceStandard,
    pub period: Duration,
    pub total_events: u64,
    pub violations: Vec<AuditEvent>,
    pub compliance_score: f32,
}

pub enum ComplianceStandard {
    PCIDSS,
    HIPAA,
    GDPR,
    SOX,
}

pub fn generate_compliance_report(
    standard: ComplianceStandard,
    period: Duration
) -> ComplianceReport {
    let audit_log = get_audit_log();
    let events = audit_log.get_events_in_period(period);

    let violations: Vec<_> = events
        .iter()
        .filter(|e| is_violation(e, &standard))
        .cloned()
        .collect();

    let score = calculate_compliance_score(&events, &violations);

    ComplianceReport {
        standard,
        period,
        total_events: events.len() as u64,
        violations,
        compliance_score: score,
    }
}
```

---

### Memory Auditing / 内存审计

#### Access Pattern Analysis / 访问模式分析

Memory auditing analyzes access patterns to detect anomalies:

内存审计分析访问模式以检测异常：

```rust
pub struct AccessPattern {
    pub address: VirtAddr,
    pub access_type: MemoryAccessType,
    pub frequency: u64,
    pub timestamps: Vec<u64>,
}

pub struct AnomalyDetector {
    baseline_patterns: HashMap<VirtAddr, AccessPattern>,
    anomaly_threshold: f32,
}

impl AnomalyDetector {
    pub fn detect_anomalies(&mut self, accesses: Vec<MemoryAccess>) -> Vec<MemoryAnomaly> {
        let mut anomalies = Vec::new();

        for access in accesses {
            if let Some(pattern) = self.baseline_patterns.get_mut(&access.address) {
                // Check if frequency deviates significantly from baseline
                let freq_diff = (access.frequency as f32 - pattern.frequency as f32)
                    / pattern.frequency as f32;

                if freq_diff.abs() > self.anomaly_threshold {
                    anomalies.push(MemoryAnomaly {
                        address: access.address,
                        anomaly_type: AnomalyType::FrequencyAnomaly,
                        severity: Severity::Medium,
                    });
                }
            } else {
                // New access pattern not in baseline
                anomalies.push(MemoryAnomaly {
                    address: access.address,
                    anomaly_type: AnomalyType::UnknownPattern,
                    severity: Severity::Low,
                });
            }
        }

        anomalies
    }
}
```

#### Anomaly Detection / 异常检测

Machine learning models can detect advanced memory anomalies:

机器学习模型可以检测高级内存异常：

```rust
pub struct MLAnomalyDetector {
    model: Model,
    feature_extractor: FeatureExtractor,
}

impl MLAnomalyDetector {
    pub fn analyze_memory_behavior(&self, accesses: &[MemoryAccess]) -> Vec<Anomaly> {
        // Extract features from access patterns
        let features = self.feature_extractor.extract(accesses);

        // Run anomaly detection model
        let predictions = self.model.predict(&features);

        // Convert predictions to anomalies
        predictions
            .into_iter()
            .filter(|p| p.anomaly_score > 0.8)
            .map(|p| Anomaly {
                address: p.address,
                anomaly_type: p.anomaly_type,
                confidence: p.anomaly_score,
                description: p.description,
            })
            .collect()
    }
}
```

#### Leak Detection / 泄漏检测

Memory leak detection tracks allocations and deallocations:

内存泄漏检测跟踪分配和释放：

```rust
pub struct LeakDetector {
    allocations: HashMap<*const u8, AllocationInfo>,
    allocation_stack_traces: HashMap<*const u8, Vec<Frame>>,
}

pub struct AllocationInfo {
    pub size: usize,
    pub allocated_at: u64,
    pub freed: bool,
}

impl LeakDetector {
    pub fn detect_leaks(&self) -> Vec<MemoryLeak> {
        let mut leaks = Vec::new();

        for (&ptr, alloc_info) in &self.allocations {
            if !alloc_info.freed {
                let stack_trace = self.allocation_stack_traces
                    .get(&ptr)
                    .cloned()
                    .unwrap_or_default();

                leaks.push(MemoryLeak {
                    address: ptr as usize,
                    size: alloc_info.size,
                    allocated_at: alloc_info.allocated_at,
                    stack_trace,
                });
            }
        }

        leaks
    }
}
```

---

### Intrusion Detection / 入侵检测

#### IDS/IPS Integration / IDS/IPS集成

Integration with Intrusion Detection/Prevention Systems:

与入侵检测/防御系统集成：

```rust
pub struct IDSEvent {
    pub timestamp: u64,
    pub source: String,
    pub event_type: IDSEventType,
    pub severity: Severity,
    pub details: HashMap<String, String>,
}

pub enum IDSEventType {
    SignatureMatch {
        signature_id: String,
        signature_name: String,
    },
    AnomalyDetected {
        anomaly_type: String,
        confidence: f32,
    },
    BehaviorViolation {
        rule_id: String,
        description: String,
    },
}

pub struct IDS {
    pub signature_engine: SignatureEngine,
    pub anomaly_detector: AnomalyDetector,
    pub behavior_monitor: BehaviorMonitor,
}

impl IDS {
    pub fn process_event(&mut self, event: &SystemEvent) -> Option<IDSEvent> {
        // Check signature-based rules
        if let Some(sig_match) = self.signature_engine.match_signatures(event) {
            return Some(IDSEvent {
                timestamp: event.timestamp,
                source: event.source.clone(),
                event_type: IDSEventType::SignatureMatch {
                    signature_id: sig_match.id,
                    signature_name: sig_match.name,
                },
                severity: sig_match.severity,
                details: sig_match.details,
            });
        }

        // Run anomaly detection
        if let Some(anomaly) = self.anomaly_detector.detect(event) {
            return Some(IDSEvent {
                timestamp: event.timestamp,
                source: event.source.clone(),
                event_type: IDSEventType::AnomalyDetected {
                    anomaly_type: anomaly.anomaly_type,
                    confidence: anomaly.confidence,
                },
                severity: anomaly.severity,
                details: anomaly.details,
            });
        }

        // Check behavioral rules
        if let Some(violation) = self.behavior_monitor.check(event) {
            return Some(IDSEvent {
                timestamp: event.timestamp,
                source: event.source.clone(),
                event_type: IDSEventType::BehaviorViolation {
                    rule_id: violation.rule_id,
                    description: violation.description,
                },
                severity: violation.severity,
                details: violation.details,
            });
        }

        None
    }
}
```

#### Signature-Based Detection / 基于签名的检测

Signature-based detection uses known attack patterns:

基于签名的检测使用已知攻击模式：

```rust
pub struct Signature {
    pub id: String,
    pub name: String,
    pub pattern: SignaturePattern,
    pub severity: Severity,
}

pub enum SignaturePattern {
    SyscallSequence {
        syscalls: Vec<u32>,
    },
    MemoryAccessPattern {
        addresses: Vec<VirtAddr>,
        access_types: Vec<MemoryAccessType>,
    },
    NetworkTraffic {
        source_port: u16,
        destination_port: u16,
        protocol: u8,
        payload_pattern: Vec<u8>,
    },
}

pub struct SignatureEngine {
    signatures: Vec<Signature>,
}

impl SignatureEngine {
    pub fn match_signatures(&self, event: &SystemEvent) -> Option<&Signature> {
        for sig in &self.signatures {
            match &sig.pattern {
                SignaturePattern::SyscallSequence { syscalls } => {
                    if let SystemEventData::SyscallSequence { sequence } = &event.data {
                        if *sequence == *syscalls {
                            return Some(sig);
                        }
                    }
                },
                SignaturePattern::MemoryAccessPattern { addresses, access_types } => {
                    if let SystemEventData::MemoryAccess { addr, access_type } = &event.data {
                        if addresses.contains(addr) && access_types.contains(access_type) {
                            return Some(sig);
                        }
                    }
                },
                _ => {},
            }
        }

        None
    }
}
```

#### Behavioral Analysis / 行为分析

Behavioral analysis establishes baselines and detects deviations:

行为分析建立基线并检测偏差：

```rust
pub struct BehaviorProfile {
    pub pid: u32,
    pub executable: String,
    pub baseline_metrics: BaselineMetrics,
}

pub struct BaselineMetrics {
    pub typical_syscalls: HashMap<u32, f32>,  // syscall -> frequency
    pub memory_access_pattern: HashMap<VirtAddr, u64>,
    pub network_connections: HashSet<ConnectionInfo>,
    pub cpu_usage_range: (f32, f32),
    pub memory_usage_range: (f32, f32),
}

pub struct BehaviorMonitor {
    profiles: HashMap<u32, BehaviorProfile>,
    learning_mode: bool,
}

impl BehaviorMonitor {
    pub fn check(&mut self, event: &SystemEvent) -> Option<BehaviorViolation> {
        let profile = self.profiles.get_mut(&event.pid)?;

        if self.learning_mode {
            // Update baseline
            profile.update_baseline(event);
            return None;
        }

        // Check for deviations
        if let Some(deviation) = profile.check_deviation(event) {
            return Some(BehaviorViolation {
                rule_id: format!("behavior-{}", event.pid),
                description: deviation.description,
                severity: deviation.severity,
                details: deviation.details,
            });
        }

        None
    }
}
```

---

## Security Hardening Checklist / 安全加固清单

### Compile-time Flags / 编译时标志

**Required Flags / 必需标志:**
```bash
# Position Independent Code
-fPIC
-fPIE

# Stack Protection
-fstack-protector-strong

# Fortify Source
-D_FORTIFY_SOURCE=3

# Relocation Read-Only
-Wl,-z,relro
-Wl,-z,now

# No-executable stack and heap
-Wl,-z,noexecstack
-Wl,-z,noexecheap

# Control Flow Integrity (if supported)
-fcf-protection=full

# Stack Clash Protection
-fstack-clash-protection

# Identify Files
-fplugin=annobin

# Optimizations
-O2
```

**Optional/Experimental Flags / 可选/实验性标志:**
```bash
# Shadow Stack (Intel CET)
-fshstk

# Static Analysis
-fanalyzer

# Undefined Behavior Sanitizer
-fsanitize=undefined

# Address Sanitizer (debug builds only)
-fsanitize=address

# Thread Sanitizer (debug builds only)
-fsanitize=thread
```

---

### Runtime Configuration / 运行时配置

**Kernel Parameters / 内核参数:**
```bash
# ASLR
kernel.randomize_va_space = 2

# Core Dumps
kernel.core_pattern = |/bin/false

# Symlink Protection
fs.protected_symlinks = 1
fs.protected_hardlinks = 1

# Device Files
fs.protected_fifos = 2
fs.protected_regular = 1

# ptrace Scope
kernel.yama.ptrace_scope = 1

# Module Loading
kernel.modules_disabled = 1  # After boot
kernel.kexec_load_disabled = 1

# AppArmor
apparmor = 1
apparmor.logsyscall = 1

# Seccomp
kernel.seccomp = 1
```

**System Hardening / 系统加固:**
```bash
# Disable unprivileged user namespaces
kernel.unprivileged_bpf_disabled = 1
user.max_user_namespaces = 0

# Enable Address Space Layout Randomization
sysctl -w kernel.randomize_va_space=2

# Enable dmesg restriction
kernel.dmesg_restrict = 1

# Restrict kernel pointer access
kernel.kptr_restrict = 2

# Restrict perf event usage
kernel.perf_event_paranoid = 3

# Restrict core dumps
fs.suid_dumpable = 0
```

---

### Best Practices / 最佳实践

**Development / 开发:**
1. Use safe Rust by default; minimize unsafe code
   默认使用安全的Rust；最小化不安全代码
2. Enable all compiler warnings and treat them as errors
   启用所有编译器警告并将其视为错误
3. Use static analysis tools (clippy, rust-analyzer)
   使用静态分析工具（clippy、rust-analyzer）
4. Perform regular security audits and penetration testing
   定期进行安全审计和渗透测试
5. Keep dependencies updated and monitor for vulnerabilities
   保持依赖更新并监控漏洞
6. Implement defense in depth (multiple layers of security)
   实施纵深防御（多层安全）

**Deployment / 部署:**
1. Principle of least privilege: run with minimum required permissions
   最小权限原则：以最低必需权限运行
2. Enable all security features (ASLR, stack canaries, etc.)
   启用所有安全功能（ASLR、栈金丝雀等）
3. Use secure communication channels (TLS, SSH)
   使用安全通信通道（TLS、SSH）
4. Implement proper input validation and output encoding
   实施适当的输入验证和输出编码
5. Use secure configuration management (encrypted config files)
   使用安全配置管理（加密的配置文件）
6. Enable comprehensive logging and monitoring
   启用全面的日志记录和监控

**Incident Response / 事件响应:**
1. Have an incident response plan ready
   准备好事件响应计划
2. Regularly backup critical data
   定期备份关键数据
3. Implement automated alerts for suspicious activity
   为可疑活动实施自动警报
4. Conduct regular security training for developers
   为开发人员进行定期安全培训
5. Perform post-incident analysis and improve processes
   进行事后分析并改进流程

---

## Performance Impact / 性能影响

### ASLR Overhead / ASLR开销

- **Performance Impact**: <1% overhead
  **性能影响**: <1%开销
- **Reasoning**: Randomization is performed once at process start or memory allocation time
  **原因**: 随机化在进程启动或内存分配时执行一次
- **Memory Impact**: Negligible (few bytes per process for randomization state)
  **内存影响**: 可忽略不计（每个进程用于随机化状态的几个字节）

**Measurement / 测量:**
```rust
pub fn benchmark_aslr_overhead(iterations: usize) -> (u64, u64) {
    let start = get_timestamp_ns();

    for _ in 0..iterations {
        let base = VirtAddr::new(0x10000000);
        let _ = randomize_memory_region(0, base, 4096, 4096, MemoryRegionType::Stack);
    }

    let elapsed = get_timestamp_ns() - start;

    let overhead_ns = elapsed / iterations as u64;
    let overhead_pct = (overhead_ns as f64 / 1_000_000_000.0) * 100.0;

    (overhead_ns, overhead_pct as u64)
}
```

---

### Stack Canaries Overhead / 栈金丝雀开销

- **Performance Impact**: <2% overhead
  **性能影响**: <2%开销
- **Reasoning**: Canary validation occurs on function entry/exit
  **原因**: 金丝雀验证在函数入口/出口时发生
- **Optimization**: Only applies to functions with buffers (stack-protector-strong)
  **优化**: 仅适用于具有缓冲区的函数（stack-protector-strong）

**Measurement / 测量:**
```rust
pub fn benchmark_canary_overhead(iterations: usize) -> (u64, u64) {
    let start = get_timestamp_ns();

    for _ in 0..iterations {
        let canary = insert_canary();
        // Simulate function body
        let _ = validate_canary(canary);
    }

    let elapsed = get_timestamp_ns() - start;
    let per_call_ns = elapsed / iterations as u64;

    (per_call_ns, (per_call_ns / 10) as u64)  // Assume 10ns baseline
}
```

---

### Seccomp Overhead / Seccomp开销

- **Performance Impact**: Variable (2-20% depending on filter complexity)
  **性能影响**: 可变（2-20%，取决于过滤器复杂性）
- **Reasoning**: BPF program executed on every syscall
  **原因**: 每次系统调用时执行BPF程序
- **Optimization**: Use simple filters and minimize syscall rate
  **优化**: 使用简单的过滤器并最小化系统调用率

**Measurement / 测量:**
```rust
pub fn benchmark_seccomp_overhead(iterations: usize) -> (u64, u64) {
    let filter = create_seccomp_filter();

    let start_no_filter = get_timestamp_ns();
    for _ in 0..iterations {
        unsafe { libc::getpid() };
    }
    let elapsed_no_filter = get_timestamp_ns() - start_no_filter;

    apply_seccomp_filter(&filter).unwrap();

    let start_with_filter = get_timestamp_ns();
    for _ in 0..iterations {
        unsafe { libc::getpid() };
    }
    let elapsed_with_filter = get_timestamp_ns() - start_with_filter;

    let overhead_ns = elapsed_with_filter - elapsed_no_filter;
    let overhead_pct = (overhead_ns as f64 / elapsed_no_filter as f64) * 100.0;

    (overhead_ns / iterations as u64, overhead_pct as u64)
}
```

---

## Summary / 总结

This document provides a comprehensive overview of the security mechanisms implemented in the NOS kernel. The multi-layered approach includes:

本文档提供了NOS内核中实施的安全机制的全面概述。多层方法包括：

1. **Memory Security**: ASLR, stack canaries, heap protection, and fortification
   **内存安全**: ASLR、栈金丝雀、堆保护和强化

2. **Access Control**: DAC, ACLs, capabilities, and MAC (SELinux/AppArmor)
   **访问控制**: DAC、ACL、能力和MAC（SELinux/AppArmor）

3. **Sandboxing**: Seccomp, namespaces, and cgroups for process isolation
   **沙箱**: Seccomp、命名空间和cgroups用于进程隔离

4. **Runtime Protections**: SMEP/SMAP, NX bit, PTI, and CFI
   **运行时保护**: SMEP/SMAP、NX位、PTI和CFI

5. **Audit and Monitoring**: Comprehensive logging, anomaly detection, and IDS integration
   **审计和监控**: 全面的日志记录、异常检测和IDS集成

These mechanisms work together to provide defense-in-depth, protecting against a wide range of attack vectors while maintaining acceptable performance overhead.

这些机制协同工作提供纵深防御，防止广泛的攻击向量，同时保持可接受的性能开销。

---

## References / 参考

1. PaX Team. [ASLR Design](https://pax.grsecurity.net/docs/aslr.txt)
2. Rust Secure Code Working Group. [Security Best Practices](https://github.com/RustSec/community)
3. Linux Kernel Documentation. [Security Features](https://www.kernel.org/doc/html/latest/security/index.html)
4. Intel. [Control-flow Enforcement Technology (CET)](https://software.intel.com/content/www/us/en/develop/articles/intel-cet.html)
5. NIST. [Security and Privacy Controls for Information Systems](https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final)

---

**Document Version / 文档版本**: 1.0
**Last Updated / 最后更新**: 2025-12-31
**Author / 作者**: NOS Security Team
**License / 许可证**: MIT
