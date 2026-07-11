# US-MON-37 — Privileged Read-only Command Broker

**Sprint:** 10H | **Priority:** Must before native raw access | **Status:** 📋 Planned

แยก privileged protocol execution ออกจาก TUI เพื่อจำกัดผลกระทบหาก frontend/config/plugin ถูกโจมตี

## Acceptance Criteria

1. broker เป็น process แยกและ TUI ไม่มี `CAP_SYS_RAWIO`/`CAP_SYS_ADMIN`
2. IPC peer authentication และ typed request enum; ไม่มี raw path/opcode/buffer/pointer input
3. request bind กับ validated DeviceId+generation และ revalidate ก่อน execute
4. per-backend allowlist, direction/length/timeout/device limits และ audit trail
5. seccomp/resource/concurrency limits ตาม platform support
6. malicious config, replay, confused-deputy, path reuse และ oversized request tests ผ่าน
7. broker ไม่สามารถอ่าน filesystem contents/raw sectors ผ่าน public API
