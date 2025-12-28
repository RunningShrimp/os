#!/bin/bash
# Files that need ToString import
files=(
  "kernel/src/debug/visualization.rs"
  "kernel/src/di/mod.rs"
  "kernel/src/error/panic_handler.rs"
  "kernel/src/event/bus.rs"
  "kernel/src/monitoring/alerting.rs"
  "kernel/src/monitoring/health.rs"
  "kernel/src/monitoring/metrics.rs"
  "kernel/src/perf/mod.rs"
  "kernel/src/security/aslr.rs"
  "kernel/src/security/memory_audit.rs"
  "kernel/src/services/discovery.rs"
  "kernel/src/subsystems/drivers/disk_io.rs"
  "kernel/src/subsystems/fs/api/mod.rs"
  "kernel/src/subsystems/fs/file_permissions.rs"
  "kernel/src/subsystems/fs/mod.rs"
  "kernel/src/subsystems/ipc/mqueue.rs"
  "kernel/src/subsystems/net/buffer_management.rs"
  "kernel/src/subsystems/perf/monitor.rs"
  "kernel/src/subsystems/perf/profiler.rs"
  "kernel/src/subsystems/process/dynamic_linker.rs"
  "kernel/src/subsystems/syscalls/object/property.rs"
  "kernel/src/subsystems/syscalls/services/mod.rs"
  "kernel/src/syscall_interface.rs"
  "kernel/src/vfs/sysfs.rs"
  "kernel/src/vfs/tmpfs.rs"
)

for file in "${files[@]}"; do
  if [ -f "$file" ]; then
    # Check if file already has ToString import
    if ! grep -q "use alloc::string::ToString" "$file"; then
      echo "Adding ToString import to $file"
      # Find first alloc:: import and add ToString after it
      sed -i '' '/^use alloc::[^s]/a\
use alloc::string::ToString;
' "$file" 2>/dev/null || sed -i '/^use alloc::[^s]/a\
use alloc::string::ToString;
' "$file"
    else
      echo "$file already has ToString import"
    fi
  fi
done
