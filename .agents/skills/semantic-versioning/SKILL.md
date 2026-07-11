---
name: semantic-versioning
description: >
  Decide and apply the correct project version number using Semantic Versioning
  2.0.0. Use this skill whenever a change needs a version bump — finishing a
  sprint, adding a feature, fixing a bug, updating docs, cutting a release, or
  editing `package.json` / `docs/changelog.md`. Trigger on any request like
  "bump the version", "what version should this be", "fix the version number",
  "release", "update changelog", or when a sprint/feature is completed. Prevents
  the common mistake of bumping PATCH for new functionality (which must be a
  MINOR bump).
---

# Semantic Versioning Decision Skill (SemVer 2.0.0)

Authoritative ruleset: **Semantic Versioning 2.0.0** — https://semver.org.
This skill turns "what number do I use?" into a deterministic decision and a
consistent update across `package.json`, `docs/changelog.md`, and any doc that
cites the version.

---

## 1. The format

```
MAJOR.MINOR.PATCH            e.g. 0.5.0
        └ optional: -PRERELEASE   +BUILD   e.g. 0.5.0-rc.1+build.7
```

- `MAJOR`, `MINOR`, `PATCH` are non-negative integers, **no leading zeroes**.
- A version, once committed/released, **MUST NOT be edited**. Ship a new version
  instead (SemVer rule 3). Only the *current, unreleased/uncommitted* version may
  be corrected.
- `v0.5.0` is **not** a SemVer string — `0.5.0` is. The `v` is only a tag prefix
  (e.g. `git tag v0.5.0`).

---

## 2. The core rule (memorize this)

Given `MAJOR.MINOR.PATCH`, increment the:

| Part | Increment when… | Then |
| :--- | :--- | :--- |
| **MAJOR** | You make a **backward-incompatible** (breaking) change to the public API | reset MINOR and PATCH to 0 |
| **MINOR** | You add **new, backward-compatible functionality** (or deprecate API) | reset PATCH to 0 |
| **PATCH** | You make a **backward-compatible bug fix** only (no new behavior) | — |

> A "bug fix" is an internal change that fixes incorrect behavior. New features,
> new files/modules, new tools/scripts, new config surfaces = **functionality**,
> not a bug fix → **MINOR**, never PATCH.

---

## 3. This project is pre-1.0 (`0.MINOR.PATCH`)

The TikTok Live Sandbox Game is a prototype before a stable public API, so it
stays in **major version 0** (SemVer rule 4: "anything MAY change"). Until the
first stable/production release that declares a frozen public API, **do not bump
to `1.0.0`**. Within `0.x`, apply this mapping:

| Change type | Bump | Example |
| :--- | :--- | :--- |
| A sprint / feature that **adds functionality** | **MINOR** `0.Y.0` | `0.4.7 → 0.5.0` (Sprint 04 content system) |
| New runtime system, module, manager, or npm script | **MINOR** | turret system, data-driven content layer |
| Backward-compatible bug fix, no new behavior | **PATCH** `0.y.Z` | fix wrong HP clamp → `0.5.0 → 0.5.1` |
| Docs-only / changelog / validation notes, no code behavior change | **PATCH** (or no bump) | record a playtest result |
| Breaking change while still in 0.x | **MINOR** (still 0.x) | rename/replace a public schema field |

> Reaching `1.0.0`: only when the public API (content schema, IPC events, npm
> script contract, game behavior others depend on) is declared stable for
> production. That is a deliberate decision, not an automatic bump.

---

## 4. Decision procedure

1. **Classify the change.** Is it: (a) a bug fix only, (b) new functionality, or
   (c) a breaking change to something others depend on?
2. **Pick the part** using §2/§3. When a change set mixes types, the **highest**
   applicable part wins (feature + bugfix together → MINOR).
3. **Apply the reset rule** (bumping MINOR resets PATCH to 0; bumping MAJOR
   resets MINOR and PATCH to 0).
4. **Never edit an already-committed version.** Correct only the current one.
5. **Synchronize every place the version appears** (§5).

Common trap → the reason this skill exists: *"I finished a sprint that added a
whole new system, so I'll bump the last digit (PATCH)."* **Wrong.** New
functionality is a **MINOR** bump. A sprint that ships features goes
`0.4.x → 0.5.0`, not `0.4.7 → 0.4.8`.

---

## 5. Update protocol (keep these in lockstep)

When the version changes, update **all** of:

1. `tiktok-live-game-prototype/package.json` → `"version"`.
2. `docs/changelog.md` → new top entry `## [X.Y.Z] - YYYY-MM-DD` with
   `Added` / `Changed` / `Fixed` / `Validated` sections, plus a one-line bump
   rationale (which part and why).
3. Any doc that cites the version (user stories, sprint backlog, software design
   headers, `docs/index.md`). Grep for the old number to find them:
   `grep -rn "X\.Y\.Z" docs tiktok-live-game-prototype/package.json`.
4. Follow the project's existing Document Update Protocol (CLAUDE.md): bump the
   doc's own `**Version:**` header and add a changelog entry.

> Document version headers (e.g. `software/11` at `0.2.0`) version that *document*
> independently of the project/package version — do not force them equal.

---

## 6. Pre-release and build metadata (when needed)

- **Pre-release:** append `-` + dot-separated identifiers: `0.5.0-alpha`,
  `0.5.0-rc.1`. Lower precedence than the normal version
  (`0.5.0-rc.1 < 0.5.0`). Use for release candidates / unstable test builds.
- **Build metadata:** append `+` + identifiers: `0.5.0+build.2026-06-18`.
  **Ignored for precedence.** Use for build/CI info only.
- Precedence order example:
  `0.5.0-alpha < 0.5.0-alpha.1 < 0.5.0-beta < 0.5.0-rc.1 < 0.5.0`.

---

## 7. Validation regex (SemVer 2.0.0, official)

```
^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$
```

Quick check: `node -e "console.log(/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test('0.5.0'))"`.

---

## 8. Worked examples (this repo)

| Situation | Correct version | Why |
| :--- | :--- | :--- |
| Sprint 04 ships data-driven content + donation turret | `0.4.7 → 0.5.0` | new functionality → MINOR, reset PATCH |
| Fix a wrong fallback in `ConfigRegistry`, no new behavior | `0.5.0 → 0.5.1` | backward-compatible bug fix → PATCH |
| Add a new theme preset + new ThemeManager method | `0.5.1 → 0.6.0` | new functionality → MINOR |
| Correct a typo in the changelog of an unreleased version | edit in place | not yet released; no new number |
| Change the committed `0.4.7` entry's meaning | **don't** — ship `0.x.(z+1)` | released versions are immutable (rule 3) |
| Declare the content/IPC API stable for production | `0.x → 1.0.0` | deliberate stable-API milestone |

---

## 9. Checklist before finalizing a bump

```
[ ] Classified the change (fix / feature / breaking)
[ ] Picked the highest applicable part; applied reset rule
[ ] Did NOT edit any already-committed version
[ ] package.json version updated
[ ] changelog.md: new entry + bump rationale line
[ ] grep'd old version; updated every doc reference
[ ] version string passes the SemVer regex
[ ] stayed in 0.x (did not jump to 1.0.0 without a stable-API decision)
```

---

## Reference

- Semantic Versioning 2.0.0: https://semver.org
- Project version policy note: top of [`docs/changelog.md`](../../../docs/changelog.md)
- Document Update Protocol: `CLAUDE.md` → "Document Update Protocol"
