# US-MON-34 — Native SATA/ATA SMART Backend

**Sprint:** 10E | **Priority:** Should | **Status:** 🚧 In Progress

สร้าง typed SAT ATA PASS-THROUGH backend สำหรับ IDENTIFY/SMART โดยแยก vendor interpretation และไม่รับ arbitrary taskfile

## Acceptance Criteria

1. allowlist เฉพาะ read-only IDENTIFY/SMART/log operations ที่ผ่าน capability gate
2. ATA return descriptor/status/checksum/length ถูก validate
3. SMART raw attributes ไม่ถูกตีความโดยไม่มี sourced model schema
4. USB/controller fallback explicit และ bounded; unknown bridge ไม่รับ vendor probe
5. privilege helper ใช้ typed requests เท่านั้น
6. vendor fixtures/fuzzing และ hardware qualification ผ่าน

## Implementation Progress

- [x] typed APT-16 builders expose only IDENTIFY, SMART READ DATA, SMART READ THRESHOLDS and SMART RETURN STATUS
- [x] pure 512-byte IDENTIFY parser decodes ATA strings, 28/48-bit capacity, SMART support and rotation/SSD state
- [x] SMART parser validates checksum, preserves six raw vendor bytes, keeps invalid normalized values unavailable and matches thresholds by ID
- [x] descriptor-sense ATA return registers and SMART pass/fail/unknown signatures are bounds checked
- [x] truncated/missing/bad-checksum responses never become success
- [x] IDENTIFY validity markers gate SMART/GPL/LBA48 and logical/physical sector geometry; capacity uses checked byte arithmetic
- [x] SMART threshold evaluation distinguishes unavailable/not-applicable/passing/exceeded without assigning raw vendor semantics
- [x] evidence-only routing distinguishes native SAT, qualified USB SAT, native SCSI, controller-hidden, ambiguous and unsupported USB bridges
- [x] capability-gated READ LOG EXT exposes only page 0 directory and preserves unknown page addresses without vendor interpretation
- [x] standalone fuzz targets cover IDENTIFY/SMART/GPL parsers, threshold evaluation and ATA return descriptors using in-memory bytes only
- [x] bounded nightly/ASan campaigns complete for both ATA fuzz targets with no crash artifact
- [x] vendor schema framework requires model+firmware match, source provenance, typed decoder/unit/direction and checked conversion
- [x] schema mismatch and multiple matches remain unknown; invalid schema and overflow are explicit errors
- [x] typed broker contract binds ATA operations to broker-owned grants, whole-device inventory nodes and exact diskseq/dev_t generations
- [x] broker request surface has no raw path/CDB/taskfile/timeout/length fields; execution limits derive from fixed operations
- [x] versioned exact-length broker wire codec rejects malformed/trailing frames and exposes no arbitrary payload
- [x] peer UID/GID/PID policy and monotonically increasing per-session request IDs provide authentication/replay gates before authorization
- [ ] standardized health log pages, reviewed real vendor rules, curated seeds, broker and hardware qualification

## Operator Evidence

- 2026-07-12: sanitized `cargo test --lib ata::tests` foundation run passed 6/6; no device identity was recorded
- 2026-07-12: operator rerun passed ATA 8/8, library 31/31 and binary 75/75; output contained test names only
- 2026-07-12: operator rerun passed ATA 10/10, library 33/33 and binary 75/75; output contained test names only
- 2026-07-12: production checks passed after adding ATA fuzz targets; fuzz build remains pending because `libfuzzer-sys` is not cached offline
- 2026-07-12: `cargo-fuzz 0.13.2` installed; stable attempts correctly stopped at the nightly-only sanitizer gate before target execution
- 2026-07-12: `ata_pages` completed 14,898,840 executions in 61 seconds (`cov 470`, `ft 853`, peak RSS 501 MiB) without a sanitizer finding
- 2026-07-12: `ata_return_descriptor` completed 60,864,910 executions in 61 seconds (`cov 56`, `ft 80`, peak RSS 502 MiB) without a sanitizer finding
- 2026-07-12: `fuzz/artifacts/` remained empty; generated corpus is ignored and contains no captured device data
- 2026-07-12: `ata_vendor_schema` completed 26,657,285 executions in 61 seconds (`cov 86`, `ft 88`, peak RSS 445 MiB) with `DONE` and no sanitizer finding/artifact
