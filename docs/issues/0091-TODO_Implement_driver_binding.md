# [0091] // TODO: Implement driver binding

**File:** `kernel/src/drivers/device_manager.rs`
**Line:** 812
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
809:         crate::println!("[device_manager] 为设备 {} 绑定驱动程序", device.name);
810: 
811:         // 使用驱动程序管理器绑定驱动程序
812:         // TODO: Implement driver binding
813:         // if let Some(driver_manager) = crate::drivers::get_driver_manager() {
814:         //     driver_manager.register_device(device.clone())?;
815:         // }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
