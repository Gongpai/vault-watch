# Sprint 10E — Native SATA/ATA

**Status:** 🚧 In Progress | **Story:** US-MON-34 | **Started:** 0.24.0

## Tasks

- [ ] IDENTIFY/SMART/threshold/return-descriptor pure parsers
  - [x] 512-byte IDENTIFY words/strings/capacity/SMART/rotation foundation
  - [x] checksum-validated SMART attributes/threshold matching and ATA return descriptor/status signatures
  - [x] sector-size/capability validity, checked capacity and explicit threshold state
  - [x] capability-gated GPL log-directory command/parser
  - [ ] standardized health log pages and extended fixtures
- [x] typed SAT builders and strict initial command allowlist (IDENTIFY, SMART READ DATA/THRESHOLDS/RETURN STATUS)
- [ ] sourced vendor/model schema with unknown fallback
  - [x] provenance-required exact model/firmware schema framework and typed raw decoders
  - [ ] reviewed real vendor rules from official documentation
- [x] evidence-only USB/controller routing without unknown vendor probes
- [ ] fixtures/fuzzing/hardware qualification and broker integration
  - [x] standalone in-memory fuzz targets for ATA pages and return descriptors
  - [x] bounded nightly/ASan fuzz campaign for both ATA targets
  - [x] schema matching/conversion fuzz target
  - [x] bounded nightly/ASan vendor-schema campaign
  - [ ] curated synthetic seed corpus, hardware qualification and broker integration
    - [x] pure typed broker request/grant authorization contract
    - [x] bounded versioned wire envelope, peer policy and replay protection
    - [x] Linux `SO_PEERCRED` acquisition for connected Unix streams
    - [x] post-open whole-block/read-only/generation revalidation contract
    - [x] bounded per-session request budget and privacy-safe decision audit records
    - [x] guarded Unix socket bind/mode/peer-credential/identity-safe cleanup lifecycle
    - [x] broker-owned read-only whole-device open and post-open sysfs/fd evidence acquisition
    - [ ] typed ioctl executor
- [ ] BUG-06: เลิกบังคับ `-d scsi`; route SATA/ATA ตาม protocol และ parse ATA health/temperature/hours

## Exit Gate

Raw SMART valuesไม่มีการตีความข้าม vendor โดยไม่มี schema และไม่มี arbitrary taskfile API
