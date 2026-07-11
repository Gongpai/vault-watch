# Sprint 10B — Native Counters & MD RAID

**Status:** 📋 Planned | **Stories:** US-MON-30, US-MON-31

## Tasks

- [ ] `/proc/diskstats` batch snapshot + reset-safe delta metrics
- [ ] metric source/scope และ cross-layer no-double-count policy
- [ ] MD sysfs snapshot/member model + race retries
- [ ] legacy iostat/mdstat parallel comparison then cutover
- [ ] fixture/property tests และ live MD qualification handoff

## Exit Gate

Production path ไม่ต้องใช้ iostat หรือ mdstat parser และ malformed data ไม่กลายเป็น healthy
