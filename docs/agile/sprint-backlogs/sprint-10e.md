# Sprint 10E — Native SATA/ATA

**Status:** 📋 Planned | **Story:** US-MON-34

## Tasks

- [ ] IDENTIFY/SMART/threshold/return-descriptor pure parsers
- [ ] typed SAT builders and strict command allowlist
- [ ] sourced vendor/model schema with unknown fallback
- [ ] bounded USB/controller routing without unknown vendor probes
- [ ] fixtures/fuzzing/hardware qualification and broker integration
- [ ] BUG-06: เลิกบังคับ `-d scsi`; route SATA/ATA ตาม protocol และ parse ATA health/temperature/hours

## Exit Gate

Raw SMART valuesไม่มีการตีความข้าม vendor โดยไม่มี schema และไม่มี arbitrary taskfile API
