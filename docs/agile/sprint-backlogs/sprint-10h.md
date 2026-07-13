# Sprint 10H — Privilege Broker & Qualification

**Status:** 🚧 In Progress | **Stories:** US-MON-37, US-MON-38

## Tasks

- [ ] separate unprivileged TUI and privileged typed command broker
  - [x] standalone broker process and bounded authenticated server
  - [x] reusable typed Unix client transport with broker peer verification, deadlines and response correlation
  - [ ] connect native health collection/state mapping through the client
- [ ] peer auth, DeviceId+generation binding, allowlists, audit and limits
  - [x] peer credentials, generation-bound grants, fixed ATA allowlist, replay/session/concurrency limits and sanitized audit foundation
  - [ ] process resource limits and runtime grant reconciliation
- [ ] malicious IPC/config/path-reuse security tests
- [ ] Alpine/static/config/theme/UI carry-over verification
- [ ] complete protocol hardware matrix, fuzz/license/security review
- [ ] sync Cargo/docs/changelog statuses only from evidence

## Exit Gate

Release cannot execute arbitrary/raw data commands and all claimed hardware support has recorded evidence
