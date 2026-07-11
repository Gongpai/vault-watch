# Sprint 10B — Native Counters & MD RAID

**Status:** 🚧 In Progress | **Stories:** US-MON-30, US-MON-31

## Tasks

- [x] `/proc/diskstats` 11/15/17-counter batch parser + reset-safe delta metric core
- [x] monotonic sampler/cache keyed by block name + `dev_t` + `diskseq` generation
- [x] whole-device metric scope; exclude partition/virtual/MD/DM stacked double counting
- [ ] MD sysfs snapshot/member model + race retries
- [x] iostat production cutover; legacy parser retained only as test oracle
- [ ] fixture/property tests และ live MD qualification handoff
- [x] BUG-08: include eligible NVMe whole-device subjects without partition/stack double counting

## Exit Gate

Production path ไม่ต้องใช้ iostat หรือ mdstat parser และ malformed data ไม่กลายเป็น healthy
