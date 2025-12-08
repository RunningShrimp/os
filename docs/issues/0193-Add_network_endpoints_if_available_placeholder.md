# [0193] // Add network endpoints if available (placeholder)

**File:** `kernel/src/services/discovery.rs`
**Line:** 252
**Marker:** placeholder
**Suggested Priority:** Low
**Suggested Owner Role:** QA/Tester
**Suggested Estimate (hours):** 8
**Suggested Labels:** `low;placeholder`

## Context

```
249:         );
250:         endpoints.push(local_endpoint);
251: 
252:         // Add network endpoints if available (placeholder)
253:         if service_info.status == ServiceStatus::Running {
254:             let network_endpoint = ServiceEndpoint::new(
255:                 service_id,
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
