# VaultWatch Security & Privacy Threat Model

**Status:** Sprint 10 baseline | **Last Updated:** 2026-07-11

## Protected Assets

- user files, filesystem contents and raw storage sectors
- device integrity and availability
- root/raw-I/O privileges
- device identifiers and operational telemetry
- Discord webhook secret and outbound notification contents

## Trust Boundaries

The TUI, configuration file, protocol command broker, kernel device interfaces, external legacy tools and Discord endpoint are separate trust domains. The TUI and configuration are never trusted to provide raw privileged commands.

## Privacy Contract

VaultWatch collects storage topology, kernel counters, RAID state and device health metadata. It must not mount filesystems, enumerate user files, read file contents, scan raw sectors or expose an API capable of doing so. Network output is disabled unless the operator explicitly configures a notification endpoint.

## Primary Threats and Controls

| Threat | Required control |
|:---|:---|
| Malicious config injects a shell command | no shell interpolation; typed config; executable paths never accept argument strings |
| Frontend/plugin submits destructive ioctl | broker accepts typed allowlisted operations only; no raw opcode/CDB/taskfile/CDW |
| Privilege escalation through TUI | TUI runs unprivileged; raw capabilities isolated in broker |
| Device path reused after hot-plug | bind requests to scoped DeviceId + generation and revalidate before execution |
| USB bridge reset/DoS | probe budgets, per-controller concurrency, cooldown/quarantine and NoWake policy |
| Unsupported/malformed response shown healthy | explicit availability/error states; parser failure never defaults to zero/healthy |
| Metrics leak device identity externally | outbound notifications explicit; minimize identifiers and never include raw payloads/secrets |
| Webhook abuse/SSRF | accept only explicit HTTPS Discord endpoint policy in hardening phase; redact URL from logs/UI |
| Cross-layer counter confusion | metric source/scope mandatory; no additive aggregation across stacked nodes |
| Dependency or GPL contamination | dependency/license audit; external GPL tools remain optional out-of-process adapters |

## Security Gates Before Native Raw Access

1. US-MON-28 privacy/config baseline passes.
2. US-MON-29 stable identity/generation foundation passes.
3. US-MON-37 broker API, authentication, allowlists and adversarial tests pass.
4. Each protocol backend passes unsafe-command rejection, parser fuzzing and hardware qualification.

Until then, Sprint 10 native discovery is limited to read-only sysfs/procfs metadata and the existing legacy collectors remain visibly labelled.
