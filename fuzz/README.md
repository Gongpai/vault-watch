# Storage protocol parser fuzzing

These targets consume arbitrary in-memory bytes only. They do not enumerate
sysfs, open device nodes, execute CDBs, or perform ioctls.

`cargo-fuzz` uses sanitizer and coverage flags that require a nightly Rust
toolchain. Install it without changing the project's default toolchain:

```bash
rustup toolchain install nightly
```

Build all targets, then run bounded 60-second campaigns with `+nightly`:

```bash
cargo +nightly fuzz build
cargo +nightly fuzz run scsi_pages -- -max_total_time=60
cargo +nightly fuzz run scsi_sense_completion -- -max_total_time=60
cargo +nightly fuzz run ata_pages -- -max_total_time=60
cargo +nightly fuzz run ata_return_descriptor -- -max_total_time=60
cargo +nightly fuzz run ata_vendor_schema -- -max_total_time=60
```

Using `+nightly` selects nightly only for that command; do not run
`rustup default nightly` merely for this project.

Generated corpora, artifacts, coverage and target directories are ignored.
Never seed the corpus with captures containing serial numbers, VPD identifiers,
WWNs, ATA IDENTIFY captures, SMART pages, host paths, or other real device
identity. Use synthetic seeds only.
