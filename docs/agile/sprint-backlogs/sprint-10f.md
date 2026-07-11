# Sprint 10F — Native NVMe

**Status:** 📋 Planned | **Story:** US-MON-35

## Tasks

- [ ] controller/subsystem/path/namespace topology and identity
- [ ] ABI-verified Identify/Get Log Page transport
- [ ] SMART/error/endurance parsers with u128 and unit tests
- [ ] multipath/NVMe-oF scope and AER-triggered reconciliation
- [ ] destructive/admin rejection, fuzzing, QEMU and hardware tests
- [ ] BUG-05: Intel DC P4618 one-card/two-controller topology; health/endurance แยก controller

## Exit Gate

No arbitrary opcode/CDW; ioctl status/result and metric scopes are correct
