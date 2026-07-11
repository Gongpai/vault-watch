# US-MON-28 — Privacy, Consent & Security Baseline

**Sprint:** 10A | **Priority:** Must | **Status:** ✅ Done

ในฐานะผู้ติดตั้ง ฉันต้องการรู้ว่า VaultWatch อ่านข้อมูลอะไร ใช้สิทธิ์อะไร และส่งข้อมูลออกไปที่ใด เพื่อมั่นใจว่าโปรแกรมไม่สามารถแอบอ่านข้อมูลส่วนตัวหรือใช้เป็นช่องทางโจมตีระบบ

## Acceptance Criteria

1. UI แสดงว่าอ่าน metadata/health/counters เท่านั้น และ raw user-content access ถูกปฏิเสธ
2. network egress ปิดเมื่อไม่มี explicit webhook; แสดงสถานะบน UI
3. config parse error แสดงต่อผู้ใช้ ไม่ fallback เงียบ
4. empty `[discord]` ไม่ทำให้ `[system]` สูญหาย
5. threat model ครอบคลุม malicious config, device replacement, command injection, arbitrary ioctl, SSRF/webhook leakage และ privilege escalation
6. ไม่มี raw path/opcode/taskfile/CDW จาก config หรือ UI
7. security policy มี unit tests และ documentation

## Implementation Evidence

- [x] TUI disclosure แสดง metadata-only, content access denied, explicit network state, legacy collector state และ privileged broker state
- [x] typed non-configurable policy อนุญาตเฉพาะ storage metadata/kernel counters/health metadata และปฏิเสธ filesystem content/raw sectors/arbitrary privileged commands
- [x] outbound notification ถูก deny จนกว่าจะมี validated explicit Discord webhook configuration
- [x] config errors แสดงต่อผู้ใช้; command/device/webhook injection และ empty Discord table มี regression tests
- [x] threat model ครอบคลุม config injection, device replacement, arbitrary ioctl, SSRF/webhook leakage, privilege escalation และ cross-layer counter confusion
- [x] production discovery/throughput/MD paths ใช้ read-only sysfs/procfs; protocol broker ยังปิดจนกว่า US-MON-37 ผ่าน
