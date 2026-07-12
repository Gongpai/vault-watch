# SCSI parser fuzzing

These targets consume arbitrary in-memory bytes only. They do not enumerate
sysfs, open device nodes, execute CDBs, or perform ioctls.

Run after installing `cargo-fuzz` and allowing Cargo to fetch `libfuzzer-sys`:

```bash
cargo fuzz run scsi_pages
cargo fuzz run scsi_sense_completion
```

Generated corpora, artifacts, coverage and target directories are ignored.
Never seed the corpus with captures containing serial numbers, VPD identifiers,
WWNs, host paths, or other real device identity.
