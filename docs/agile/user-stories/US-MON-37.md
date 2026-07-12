# US-MON-37 — Privileged Read-only Command Broker

**Sprint:** 10H | **Priority:** Must before native raw access | **Status:** 🚧 Foundation In Progress

แยก privileged protocol execution ออกจาก TUI เพื่อจำกัดผลกระทบหาก frontend/config/plugin ถูกโจมตี

## Acceptance Criteria

1. broker เป็น process แยกและ TUI ไม่มี `CAP_SYS_RAWIO`/`CAP_SYS_ADMIN`
2. IPC peer authentication และ typed request enum; ไม่มี raw path/opcode/buffer/pointer input
3. request bind กับ validated DeviceId+generation และ revalidate ก่อน execute
4. per-backend allowlist, direction/length/timeout/device limits และ audit trail
5. seccomp/resource/concurrency limits ตาม platform support
6. malicious config, replay, confused-deputy, path reuse และ oversized request tests ผ่าน
7. broker ไม่สามารถอ่าน filesystem contents/raw sectors ผ่าน public API

## Progress

- [x] typed generation-bound requests, broker-owned authorization limits and post-open identity contract
- [x] versioned bounded wire frames, kernel-derived peer credentials and replay protection
- [x] fixed per-session request budget; malformed/replayed frames do not consume quota
- [x] privacy-safe audit decision records omit node ID, path, generation, serial and payload
- [x] guarded Unix socket lifecycle, restricted mode, kernel peer credentials and inode-safe cleanup
- [ ] broker-owned open/evidence acquisition and ioctl executor
- [ ] process separation, seccomp, concurrency limits and hostile integration qualification
