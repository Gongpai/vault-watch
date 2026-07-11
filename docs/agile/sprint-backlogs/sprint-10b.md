# Sprint 10B — Native Counters & MD RAID

**Status:** 🚧 In Progress | **Stories:** US-MON-30, US-MON-31

## Tasks

- [x] `/proc/diskstats` 11/15/17-counter batch parser + reset-safe delta metric core
- [ ] monotonic sampler/cache keyed by graph identity+generation
- [ ] metric source/scope และ cross-layer no-double-count policy
- [ ] MD sysfs snapshot/member model + race retries
- [ ] legacy iostat/mdstat parallel comparison then cutover
- [ ] fixture/property tests และ live MD qualification handoff
- [ ] BUG-08: include eligible NVMe whole-device subjects without partition/stack double counting

## Exit Gate

Production path ไม่ต้องใช้ iostat หรือ mdstat parser และ malformed data ไม่กลายเป็น healthy
