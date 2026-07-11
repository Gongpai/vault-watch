# US-MON-30 — Native Block Throughput Backend

**Sprint:** 10B | **Priority:** Must | **Status:** 🚧 In Progress

แทน `iostat` ด้วย `/proc/diskstats` หรือ sysfs `stat` พร้อม delta calculator ที่แยก metric scope ถูกต้อง

## Acceptance Criteria

1. รองรับ base/discard/flush field variants และ sector unit 512 bytes
2. คำนวณ MiB/s, IOPS, utilization, latency และ queue depth จาก monotonic interval
3. reset/replacement ไม่สร้าง throughput spike
4. ไม่รวม counters ข้าม partition/DM/MD/member layers แบบ additive
5. fixture tests ครอบคลุม malformed, idle, reset, hot-remove และ multiple devices
6. production runtime ไม่เรียก `iostat`

## Implementation Progress

- [x] AC1–AC3 core: defensive 11/15/17-counter parser, 512-byte sector conversion, MiB/s/IOPS/utilization/latency/queue metrics, zero-denominator availability และ reset/`dev_t` interval rejection
- [x] AC5 partial: malformed, multiple-device, idle, reset, zero-interval fixtures
- [ ] generation-keyed sampler, removal/reappearance fixtures, scope selection, parallel oracle comparison and production cutover
