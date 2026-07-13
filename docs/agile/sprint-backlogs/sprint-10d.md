# Sprint 10D — Native SAS/SCSI

**Status:** 🚧 In Progress | **Story:** US-MON-33 | **Started:** 0.18.0

## Tasks

- [ ] pure INQUIRY/VPD/LOG SENSE/sense parsers + fixtures/fuzzing
  - [x] typed read-only CDB builders, standard INQUIRY, supported VPD, temperature LOG SENSE and fixed/descriptor sense foundation
  - [x] VPD descriptor/rotation, error/non-medium/informational-exception logs, bounded sense actions and truncated-prefix fixtures
  - [x] standalone fuzz targets for pages and sense/completion (campaign execution pending dependency approval)
  - [ ] remaining optional pages
- [ ] verified SG_IO ABI wrapper in bounded blocking worker
  - [x] pure completion/residual/sense validation before ABI integration
  - [x] private Linux UAPI layout assertions, bounded typed request and semaphore-gated blocking worker foundation
  - [x] broker-owned startup capability ioctl with whole-device generation revalidation
  - [ ] broker-owned native SCSI health execution and hardware qualification
- [x] typed read-only allowlist and unsafe-command rejection tests
- [ ] capability discovery and SAS/SAT/controller-hidden routing
  - [x] pure evidence routing and advertised-page scheduling
  - [x] injectable sysfs SG mapping with hot-remove/no-interface/ambiguous states
  - [x] runtime standard INQUIRY → supported VPD → ATA Information VPD capability discovery through broker
- [ ] integrate through broker only after US-MON-37 gate

## Live/Operator Verification

- [x] initial pure SCSI suite 6/6 on 2026-07-12; evidence stores no host, path or device identity
- [x] expanded parser/sense suite 12/12 on 2026-07-12; evidence stores no host, path or device identity
- [x] command/routing/completion suite 16/16 on 2026-07-12; evidence stores no host, path or device identity
- [x] sysfs mapping/routing suite 20/20 on 2026-07-12; evidence stores no host, path or device identity
- [x] ABI/worker suite 23/23 on 2026-07-12; evidence stores no host, path or device identity

## Exit Gate

No arbitrary CDB/data-out surface; hardware matrix records exact supported devices
