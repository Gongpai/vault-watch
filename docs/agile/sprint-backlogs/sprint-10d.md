# Sprint 10D — Native SAS/SCSI

**Status:** 📋 Planned | **Story:** US-MON-33

## Tasks

- [ ] pure INQUIRY/VPD/LOG SENSE/sense parsers + fixtures/fuzzing
- [ ] verified SG_IO ABI wrapper in bounded blocking worker
- [ ] typed read-only allowlist and unsafe-command rejection tests
- [ ] capability discovery and SAS/SAT/controller-hidden routing
- [ ] integrate through broker only after US-MON-37 gate

## Exit Gate

No arbitrary CDB/data-out surface; hardware matrix records exact supported devices
