# [0092] // TODO: Implement driver notification

**File:** `kernel/src/drivers/device_manager.rs`
**Line:** 869
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
866: 
867:     fn notify_driver_device_removed(&self, driver_name: &str, device: &Device) -> Result<(), DeviceManagerError> {
868:         // 通知驱动程序设备移除
869:         // TODO: Implement driver notification
870:         // if let Some(driver_manager) = crate::drivers::get_driver_manager() {
871:         //     driver_manager.remove_device(device.id)?;
872:         // }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
