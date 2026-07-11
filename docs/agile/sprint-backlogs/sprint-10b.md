# Sprint 10B — Native Counters & MD RAID

**Status:** 🚧 In Progress | **Stories:** US-MON-30, US-MON-31

## Tasks

- [x] `/proc/diskstats` 11/15/17-counter batch parser + reset-safe delta metric core
- [x] monotonic sampler/cache keyed by block name + `dev_t` + `diskseq` generation
- [x] whole-device metric scope; exclude partition/virtual/MD/DM stacked double counting
- [x] MD sysfs snapshot/member model + state/action boundary retries (shadow backend)
- [x] iostat production cutover; legacy parser retained only as test oracle
- [x] MD operation-generation cache + delta speed/ETA + semantic legacy-oracle fixtures
- [x] partial/unavailable availability gate + native MD sysfs production cutover
- [x] operator regression after cutover: MD fixtures 6/6, full suite 52/52 and no-array runtime startup
- [x] BUG-11: repeated progress sample no longer erases kernel rebuild speed/ETA
- [x] BUG-11: sub-2-second event sample cannot create startup delta-speed spike
- [ ] live multi-array/rebuild qualification handoff
- [x] BUG-08: include eligible NVMe whole-device subjects without partition/stack double counting
- [x] live NVMe + removable throughput/add/remove qualification (sanitized evidence 2026-07-11)
- [x] Device Details exposes scoped native IOPS/utilization/latency/queue/in-flight metrics with explicit unavailable states

## Exit Gate

Production path ไม่ใช้ iostat หรือ mdstat parser แล้ว; เหลือ live MD qualification และ malformed data ไม่กลายเป็น healthy
