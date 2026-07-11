# Sprint 10D — Native SAS/SCSI

**Status:** 🚧 In Progress | **Story:** US-MON-33 | **Started:** 0.18.0

## Tasks

- [ ] pure INQUIRY/VPD/LOG SENSE/sense parsers + fixtures/fuzzing
  - [x] typed read-only CDB builders, standard INQUIRY, supported VPD, temperature LOG SENSE and fixed/descriptor sense foundation
  - [ ] remaining VPD/log pages and fuzz targets
- [ ] verified SG_IO ABI wrapper in bounded blocking worker
- [ ] typed read-only allowlist and unsafe-command rejection tests
- [ ] capability discovery and SAS/SAT/controller-hidden routing
- [ ] integrate through broker only after US-MON-37 gate

## Exit Gate

No arbitrary CDB/data-out surface; hardware matrix records exact supported devices
