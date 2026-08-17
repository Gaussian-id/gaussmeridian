# Third-Party Notices

**Status: PENDING LEGAL REVIEW (governance gate G3).** This is a machine-generated
dependency inventory, not hand-authored prose, and it has not yet been reviewed by
counsel. It exists to make what is actually bundled auditable before that review
happens — treat it as accurate-but-provisional, not as a cleared attribution
document.

**The compatibility question this file answers has changed.** GaussMeridian is now
licensed `AGPL-3.0-only`; it was previously Apache-2.0. Under Apache-2.0 the relevant
test was attribution — reproduce the notices, and almost any permissive or weak-copyleft
dependency was fine. Under AGPL-3.0 the test is **GPL compatibility**: the combined work
is conveyed under AGPL-3.0, so every dependency's license must permit that. This inverts
one specific rule. Copyleft dependencies that were previously banned (GPL-3.0, LGPL,
AGPL-3.0) are now permitted. Dependencies that were previously merely "flagged" —
proprietary, source-available, or non-commercial terms — are now blocking defects,
because no amount of attribution makes them redistributable under AGPL-3.0.

The inventory below is unchanged tool output from the generation run described next.
The **assessment** of that inventory against AGPL-3.0 has been rewritten, and has not
been re-generated against a fresh dependency graph — see the flagged items.

## How this file was generated

- **Rust workspace** (`gaussmeridian/`): `cargo tree --workspace -e normal --prefix none -f "{p} {l}"`, deduplicated by package name + version.
- **WebUI production dependencies** (`webui/`): `pnpm licenses list --prod`, one row per package as reported by pnpm.

Both commands were run against this branch on 2026-08-15. Neither inventory was
typed by hand or inferred from package names — every line below is tool output.
Re-run both commands and regenerate this file whenever dependencies change; it is
a point-in-time snapshot, not a live gate.

**This snapshot has already drifted and needs regeneration.** One stale row was found and
removed by hand: `xendit-components-web` was listed as a webui production dependency but is
absent from `webui/package.json` and from `pnpm-lock.yaml`, and is imported by no source file —
the payment integration it belonged to is not part of this repository. That one correction does
not make the rest of the inventory current. Regenerate both commands before G3 signs anything
off; a hand-patched inventory is evidence that the generation is overdue, not a substitute for it.

## Summary

- Rust workspace: **646** unique crate/version pairs (`cargo tree --workspace -e normal`).
- WebUI production dependencies: **225** unique packages (`pnpm licenses list --prod`; devDependencies such as Playwright/ESLint/Vitest are excluded because they do not ship in the built application).
- **No AGPL or GPL-family license was found in either inventory** at generation time (`grep -i gpl` across both — the only match was the LGPL item flagged below). Under the previous Apache-2.0 license this was the headline result, because GPL-family dependencies were banned outright. Under `AGPL-3.0-only` it is no longer a compliance requirement at all: GPL-3.0, LGPL, and AGPL-3.0 dependencies are compatible with the outbound license and may be added. The fact is retained because it is true of this snapshot, not because it is still a gate.
- **The gate that replaces it: no GPL-incompatible license may appear in either inventory.** That means no proprietary or vendor-specific terms, no source-available licenses (BUSL/BSL, SSPL, Elastic, Commons Clause), no non-commercial licenses (CC-BY-NC), and no GPL-2.0-**only** dependency — GPL-2.0-only cannot be combined with AGPL-3.0 code, and it is the one copyleft license this project must still refuse. Two items in the current inventory fail or may fail this gate; see "Items flagged for legal review" below. Re-verify on every dependency change.
- **Every permissive license in the tables below is GPL-compatible and passes.** MIT, ISC, 0BSD, BSD-2-Clause, BSD-3-Clause, Apache-2.0, Zlib, BSL-1.0, Unlicense, CC0-1.0, Unicode-3.0, CDLA-Permissive-2.0, and MPL-2.0 all permit the combined work to be conveyed under AGPL-3.0. Apache-2.0 is one-way compatible with the GPLv3 family — it flows into an AGPL-3.0 work, which is why the Apache-2.0 rows below raise no issue; the reverse direction does not hold, which is why this project can no longer be consumed by an Apache-2.0-only work. The attribution and notice-preservation obligations of these licenses survive the relicense and are satisfied by this file.

### Rust license families (by unique crate/version pair)

| License (as reported by `cargo tree`) | Count |
| --- | --- |
| MIT OR Apache-2.0 | 254 |
| MIT | 141 |
| MIT/Apache-2.0 | 43 |
| Apache-2.0 OR MIT | 31 |
| Apache-2.0 | 30 |
| (proc-macro) MIT OR Apache-2.0 | 19 |
| Unicode-3.0 | 15 |
| (proc-macro) MIT | 15 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 8 |
| ISC | 7 |
| BSD-3-Clause | 7 |
| Apache-2.0 OR ISC OR MIT | 6 |
| MPL-2.0 | 5 |
| Unlicense OR MIT | 4 |
| (proc-macro) Apache-2.0 | 4 |
| Unlicense/MIT | 3 |
| (proc-macro) Unicode-3.0 | 3 |
| Zlib OR Apache-2.0 OR MIT | 2 |
| MIT / Apache-2.0 | 2 |
| CDLA-Permissive-2.0 | 2 |
| BSL-1.0 | 2 |
| BSD-2-Clause | 2 |
| Apache-2.0 OR BSD-2-Clause | 2 |
| *(blank — no license metadata)* | 2 |
| Zlib | 1 |
| Unlicense | 1 |
| MIT/BSD-3-Clause | 1 |
| MIT OR Zlib OR Apache-2.0 | 1 |
| MIT OR Apache-2.0 OR Zlib | 1 |
| MIT AND BSD-3-Clause | 1 |

(Counts sum to 646; remaining single-occurrence license strings are listed as
encountered in the full inventory below rather than repeated here.)

### WebUI production license families (by unique package)

| License (as reported by `pnpm licenses list --prod`) | Count |
| --- | --- |
| MIT | 185 |
| ISC | 24 |
| Apache-2.0 | 8 |
| BSD-3-Clause | 2 |
| MIT AND ISC | 1 |
| CC-BY-4.0 | 1 |
| Apache-2.0 AND LGPL-3.0-or-later | 1 |
| 0BSD | 1 |
| (MIT AND Zlib) | 1 |
| Standard "no charge" license (gsap.com) — not an OSI-approved identifier | 1 |

## Items flagged for legal review

Two of the three items below were raised under Apache-2.0 as questions of
attribution. Under `AGPL-3.0-only` the first two become questions of whether the
project may be distributed at all in its current form, and the third resolves in the
project's favour. Severity is stated per item.

- **`surrealdb` v2.4.0 / `surrealdb-core` v2.4.0 — CONFIRMED BSL 1.1; covered by the
  Section 7 permission.** `cargo tree` reported no SPDX identifier because these crates
  ship a `license-file` rather than a `license` field. That file has now been read
  directly from the vendored crate source rather than inferred: it is the **Business
  Source License 1.1**, Licensor SurrealDB Ltd., Licensed Work "SurrealDB 2.0",
  **Change Date 2029-09-17**, Change License Apache-2.0. BSL 1.1 is source-available and
  **not** GPL-compatible, so it cannot simply be conveyed under AGPL-3.0. Two facts
  narrow the exposure: the Additional Use Grant permits any use that is not a "Database
  Service", which GaussMeridian is not; and the SurrealDB **server** runs as a separate
  program in a separate container reached over `ws://`, which is aggregation rather than
  combination. Only the linked Rust **client** crate forms a combined work, and that is
  covered by the AGPL Section 7 additional permission recorded in `NOTICE`. On the Change
  Date the crate converts to Apache-2.0 and the permission becomes unnecessary for it.
- **`gsap` (webui, production dependency) — proprietary; covered by the Section 7
  permission, removal still preferred.** Reports `Standard 'no charge' license:
  https://gsap.com/standard-license.` Free of charge is not the same as open source:
  this is a vendor-specific proprietary term, not OSI-approved, and not GPL-compatible.
  It was verified to be genuinely in the shipped bundle rather than stale metadata —
  declared in `webui/package.json` (`^3.15.0`), present in `pnpm-lock.yaml`, and imported
  by five source files (`components/motion/{pipeline-scrollytelling,reveal,scroll-scene}.tsx`
  and `components/onboarding/{conversational-stage,onboarding-step-survey}.tsx`). The
  combination is covered by the AGPL Section 7 additional permission recorded in `NOTICE`.
  **That permission is a remedy, not a resolution.** The used API surface is small —
  `gsap.to/from/fromTo`, `gsap.context`, `gsap.utils`, `ScrollTrigger`, and `Observer`
  across 656 lines — and is replaceable by the Web Animations API plus
  `IntersectionObserver`, or by an MIT-licensed library. Removing it would let this entry
  and half the Section 7 clause be deleted, which is a materially better end state than
  carrying a permanent exception for an animation library.
- **`@img/sharp-win32-x64` (webui) — RESOLVED BY THE RELICENSE.**
  `Apache-2.0 AND LGPL-3.0-or-later`. Under Apache-2.0 this needed an LGPL §4
  linking-model analysis, because LGPL's relinking requirement sits awkwardly beside a
  permissive outbound license. Under AGPL-3.0 that analysis is unnecessary:
  LGPL-3.0-or-later is upward-compatible with GPL-3.0/AGPL-3.0, so the LGPL-covered
  portion may simply be conveyed under AGPL-3.0 along with everything else. No further
  action; keep the attribution.
- **No GPL-family dependency is present, and that is no longer required.** The
  previous entry here recorded the absence of AGPL/GPL dependencies as consistent with
  a project non-negotiable banning them. That non-negotiable was a consequence of the
  Apache-2.0 outbound license and has been retired along with it — see
  `CONTRIBUTING.md` for the rule that replaced it. This remains a point-in-time
  snapshot, not a continuously-enforced gate: the repository has no `cargo-deny`
  configuration and no CI license check, which is itself a gap worth closing now that a
  single incompatible dependency is a distribution blocker rather than a footnote.

## Full Rust dependency inventory

`cargo tree --workspace -e normal --prefix none`, run from `gaussmeridian/`,
deduplicated by package + version. Includes this workspace's own crates
(`gaussmeridian-*`). Those rows are stale as printed: they were generated while the
workspace declared `license = "Apache-2.0"`, and every one of them is now
`AGPL-3.0-only` — inherited from `[workspace.package]` in `gaussmeridian/Cargo.toml`,
except `gaussmeridian-cache`, `gaussmeridian-core`, `gaussmeridian-db`, and
`gaussmeridian-moa`, which declare it directly. No first-party crate is dual-licensed
or permissively licensed. Third-party rows below are unaffected by the relicense and
remain accurate as printed.

- `Inflector` `v0.11.4` — BSD-2-Clause
- `addr` `v0.15.6` — MIT/Apache-2.0
- `adler2` `v2.0.1` — 0BSD OR MIT OR Apache-2.0
- `aead` `v0.5.2` — MIT OR Apache-2.0
- `aes` `v0.8.4` — MIT OR Apache-2.0
- `aes-gcm` `v0.10.3` — Apache-2.0 OR MIT
- `affinitypool` `v0.3.1` — MIT/Apache-2.0
- `ahash` `v0.7.8` — MIT OR Apache-2.0
- `ahash` `v0.8.12` — MIT OR Apache-2.0
- `aho-corasick` `v1.1.4` — Unlicense OR MIT
- `allocator-api2` `v0.2.21` — MIT OR Apache-2.0
- `ambient-authority` `v0.0.2` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `ammonia` `v4.1.2` — MIT OR Apache-2.0
- `anstream` `v0.6.21` — MIT OR Apache-2.0
- `anstyle` `v1.0.13` — MIT OR Apache-2.0
- `anstyle-parse` `v0.2.7` — MIT OR Apache-2.0
- `anstyle-query` `v1.1.5` — MIT OR Apache-2.0
- `anstyle-wincon` `v3.0.11` — MIT OR Apache-2.0
- `any_ascii` `v0.3.3` — ISC
- `anyhow` `v1.0.100` — MIT OR Apache-2.0
- `approx` `v0.4.0` — Apache-2.0
- `approx` `v0.5.1` — Apache-2.0
- `arc-swap` `v1.7.1` — MIT OR Apache-2.0
- `argon2` `v0.5.3` — MIT OR Apache-2.0
- `arraydeque` `v0.5.1` — MIT/Apache-2.0
- `arrayref` `v0.3.9` — BSD-2-Clause
- `arrayvec` `v0.7.6` — MIT OR Apache-2.0
- `async-channel` `v2.5.0` — Apache-2.0 OR MIT
- `async-compression` `v0.4.34` — MIT OR Apache-2.0
- `async-executor` `v1.13.3` — Apache-2.0 OR MIT
- `async-graphql` `v7.0.17` — MIT OR Apache-2.0
- `async-graphql-derive` `v7.0.17` — (proc-macro) MIT OR Apache-2.0
- `async-graphql-parser` `v7.0.17` — MIT OR Apache-2.0
- `async-graphql-value` `v7.0.17` — MIT OR Apache-2.0
- `async-lock` `v3.4.2` — Apache-2.0 OR MIT
- `async-stream` `v0.3.6` — MIT
- `async-stream-impl` `v0.3.6` — (proc-macro) MIT
- `async-task` `v4.7.1` — Apache-2.0 OR MIT
- `async-trait` `v0.1.89` — (proc-macro) MIT OR Apache-2.0
- `atomic-waker` `v1.1.2` — Apache-2.0 OR MIT
- `axum` `v0.6.20` — MIT
- `axum` `v0.7.9` — MIT
- `axum-core` `v0.3.4` — MIT
- `axum-core` `v0.4.5` — MIT
- `axum-macros` `v0.4.2` — (proc-macro) MIT
- `backtrace` `v0.3.76` — MIT OR Apache-2.0
- `base64` `v0.13.1` — MIT/Apache-2.0
- `base64` `v0.21.7` — MIT OR Apache-2.0
- `base64` `v0.22.1` — MIT OR Apache-2.0
- `base64ct` `v1.8.0` — Apache-2.0 OR MIT
- `bcrypt` `v0.15.1` — MIT
- `bincode` `v1.3.3` — MIT
- `bit-vec` `v0.5.1` — MIT/Apache-2.0
- `bitflags` `v1.3.2` — MIT/Apache-2.0
- `bitflags` `v2.10.0` — MIT OR Apache-2.0
- `blake2` `v0.10.6` — MIT OR Apache-2.0
- `blake3` `v1.8.2` — CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception
- `block-buffer` `v0.10.4` — MIT OR Apache-2.0
- `blowfish` `v0.9.1` — MIT OR Apache-2.0
- `boxcar` `v0.2.14` — MIT
- `bytemuck` `v1.24.0` — Zlib OR Apache-2.0 OR MIT
- `byteorder` `v1.5.0` — Unlicense OR MIT
- `bytes` `v1.11.0` — MIT
- `cached` `v0.44.0` — MIT
- `cached_proc_macro` `v0.17.0` — (proc-macro) MIT
- `cached_proc_macro_types` `v0.1.1` — MIT
- `cap-fs-ext` `v4.0.2` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `cap-primitives` `v4.0.2` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `cap-std` `v4.0.2` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `cassowary` `v0.3.0` — MIT / Apache-2.0
- `castaway` `v0.2.4` — MIT
- `cedar-policy` `v2.4.2` — Apache-2.0
- `cedar-policy-core` `v2.4.2` — Apache-2.0
- `cedar-policy-validator` `v2.4.2` — Apache-2.0
- `cfg-if` `v1.0.4` — MIT OR Apache-2.0
- `chrono` `v0.4.42` — MIT OR Apache-2.0
- `ciborium` `v0.2.2` — Apache-2.0
- `ciborium-io` `v0.2.2` — Apache-2.0
- `ciborium-ll` `v0.2.2` — Apache-2.0
- `cipher` `v0.4.4` — MIT OR Apache-2.0
- `clap` `v4.5.53` — MIT OR Apache-2.0
- `clap_builder` `v4.5.53` — MIT OR Apache-2.0
- `clap_derive` `v4.5.49` — (proc-macro) MIT OR Apache-2.0
- `clap_lex` `v0.7.6` — MIT OR Apache-2.0
- `clipboard-win` `v5.4.1` — BSL-1.0
- `color-eyre` `v0.6.5` — MIT OR Apache-2.0
- `color-spantrace` `v0.3.0` — MIT OR Apache-2.0
- `colorchoice` `v1.0.4` — MIT OR Apache-2.0
- `combine` `v4.6.7` — MIT
- `compact_str` `v0.7.1` — MIT
- `compression-codecs` `v0.4.33` — MIT OR Apache-2.0
- `compression-core` `v0.4.31` — MIT OR Apache-2.0
- `concurrent-queue` `v2.5.0` — Apache-2.0 OR MIT
- `config` `v0.13.4` — MIT/Apache-2.0
- `config` `v0.14.1` — MIT OR Apache-2.0
- `console` `v0.15.11` — MIT
- `const-random` `v0.1.18` — MIT OR Apache-2.0
- `const-random-macro` `v0.1.16` — (proc-macro) MIT OR Apache-2.0
- `constant_time_eq` `v0.3.1` — CC0-1.0 OR MIT-0 OR Apache-2.0
- `convert_case` `v0.6.0` — MIT
- `cpufeatures` `v0.2.17` — MIT OR Apache-2.0
- `crc32fast` `v1.5.0` — MIT OR Apache-2.0
- `crossbeam-channel` `v0.5.15` — MIT OR Apache-2.0
- `crossbeam-deque` `v0.8.6` — MIT OR Apache-2.0
- `crossbeam-epoch` `v0.9.18` — MIT OR Apache-2.0
- `crossbeam-utils` `v0.8.21` — MIT OR Apache-2.0
- `crossterm` `v0.27.0` — MIT
- `crossterm_winapi` `v0.9.1` — MIT
- `crunchy` `v0.2.4` — MIT
- `crypto-common` `v0.1.7` — MIT OR Apache-2.0
- `cssparser` `v0.35.0` — MPL-2.0
- `cssparser-macros` `v0.6.1` — (proc-macro) MPL-2.0
- `ctr` `v0.9.2` — MIT OR Apache-2.0
- `darling` `v0.14.4` — MIT
- `darling` `v0.20.11` — MIT
- `darling` `v0.21.3` — MIT
- `darling_core` `v0.14.4` — MIT
- `darling_core` `v0.20.11` — MIT
- `darling_core` `v0.21.3` — MIT
- `darling_macro` `v0.14.4` — (proc-macro) MIT
- `darling_macro` `v0.20.11` — (proc-macro) MIT
- `darling_macro` `v0.21.3` — (proc-macro) MIT
- `dashmap` `v5.5.3` — MIT
- `data-encoding` `v2.9.0` — MIT
- `deadpool` `v0.10.0` — MIT OR Apache-2.0
- `deadpool` `v0.12.3` — MIT OR Apache-2.0
- `deadpool-redis` `v0.14.0` — MIT OR Apache-2.0
- `deadpool-redis` `v0.15.1` — MIT OR Apache-2.0
- `deadpool-runtime` `v0.1.4` — MIT OR Apache-2.0
- `deranged` `v0.5.5` — MIT OR Apache-2.0
- `deunicode` `v1.6.2` — BSD-3-Clause
- `digest` `v0.10.7` — MIT OR Apache-2.0
- `directories` `v5.0.1` — MIT OR Apache-2.0
- `dirs` `v6.0.0` — MIT OR Apache-2.0
- `dirs-sys` `v0.4.1` — MIT OR Apache-2.0
- `dirs-sys` `v0.5.0` — MIT OR Apache-2.0
- `displaydoc` `v0.2.5` — (proc-macro) MIT OR Apache-2.0
- `dlv-list` `v0.3.0` — MIT
- `dlv-list` `v0.5.2` — MIT OR Apache-2.0
- `dmp` `v0.2.3` — MIT
- `dotenv` `v0.15.0` — MIT
- `dotenvy` `v0.15.7` — MIT
- `double-ended-peekable` `v0.1.0` — MIT
- `dtoa` `v1.0.10` — MIT OR Apache-2.0
- `dtoa-short` `v0.3.5` — MPL-2.0
- `dyn-clone` `v1.0.20` — MIT OR Apache-2.0
- `earcutr` `v0.4.3` — ISC
- `either` `v1.15.0` — MIT OR Apache-2.0
- `email-encoding` `v0.4.1` — MIT OR Apache-2.0
- `email_address` `v0.2.9` — MIT
- `encode_unicode` `v1.0.0` — Apache-2.0 OR MIT
- `encoding_rs` `v0.8.35` — (Apache-2.0 OR MIT) AND BSD-3-Clause
- `endian-type` `v0.1.2` — MIT
- `equivalent` `v1.0.2` — Apache-2.0 OR MIT
- `error-code` `v3.3.2` — BSL-1.0
- `event-listener` `v5.4.1` — Apache-2.0 OR MIT
- `event-listener-strategy` `v0.5.4` — Apache-2.0 OR MIT
- `eventsource-stream` `v0.2.3` — MIT OR Apache-2.0
- `ext-sort` `v0.1.5` — Unlicense
- `eyre` `v0.6.12` — MIT OR Apache-2.0
- `fastrand` `v2.3.0` — Apache-2.0 OR MIT
- `fd-lock` `v4.0.4` — MIT OR Apache-2.0
- `filetime` `v0.2.29` — MIT/Apache-2.0
- `flate2` `v1.1.5` — MIT OR Apache-2.0
- `float_next_after` `v1.0.0` — MIT
- `fnv` `v1.0.7` — Apache-2.0 / MIT
- `foldhash` `v0.1.5` — Zlib
- `form_urlencoded` `v1.2.2` — MIT OR Apache-2.0
- `fs-set-times` `v0.20.3` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `fst` `v0.4.7` — Unlicense/MIT
- `futf` `v0.1.5` — MIT / Apache-2.0
- `futures` `v0.3.31` — MIT OR Apache-2.0
- `futures-channel` `v0.3.31` — MIT OR Apache-2.0
- `futures-core` `v0.3.31` — MIT OR Apache-2.0
- `futures-executor` `v0.3.31` — MIT OR Apache-2.0
- `futures-io` `v0.3.31` — MIT OR Apache-2.0
- `futures-lite` `v2.6.1` — Apache-2.0 OR MIT
- `futures-macro` `v0.3.31` — (proc-macro) MIT OR Apache-2.0
- `futures-sink` `v0.3.31` — MIT OR Apache-2.0
- `futures-task` `v0.3.31` — MIT OR Apache-2.0
- `futures-timer` `v3.0.3` — MIT/Apache-2.0
- `futures-util` `v0.3.31` — MIT OR Apache-2.0
- `fuzzy-matcher` `v0.3.7` — MIT
- `gaussmeridian-auth` `v3.0.0` — (gaussmeridian/crates/gaussmeridian-auth) AGPL-3.0-only
- `gaussmeridian-cache` `v0.1.0` — (gaussmeridian/crates/gaussmeridian-cache) AGPL-3.0-only
- `gaussmeridian-core` `v0.1.0` — (gaussmeridian/crates/gaussmeridian-core) AGPL-3.0-only
- `gaussmeridian-metrics` `v3.0.0` — (gaussmeridian/crates/gaussmeridian-metrics) AGPL-3.0-only
- `gaussmeridian-models` `v3.0.0` — (gaussmeridian/crates/gaussmeridian-models) AGPL-3.0-only
- `gaussmeridian-plugins` `v3.0.0` — (gaussmeridian/crates/gaussmeridian-plugins) AGPL-3.0-only
- `gaussmeridian-providers` `v3.0.0` — (gaussmeridian/crates/gaussmeridian-providers) AGPL-3.0-only
- `gaussmeridian-tui` `v3.0.0` — (gaussmeridian/services/tui) AGPL-3.0-only
- `gaussmeridian-utils` `v3.0.0` — (gaussmeridian/crates/gaussmeridian-utils) AGPL-3.0-only
- `gaussmoa` `v0.1.0` — (gaussmeridian/crates/gaussmeridian-moa) AGPL-3.0-only
- `generic-array` `v0.14.7` — MIT
- `geo` `v0.28.0` — MIT OR Apache-2.0
- `geo-types` `v0.7.17` — MIT OR Apache-2.0
- `geographiclib-rs` `v0.2.5` — MIT
- `getrandom` `v0.2.16` — MIT OR Apache-2.0
- `getrandom` `v0.3.4` — MIT OR Apache-2.0
- `ghash` `v0.5.1` — Apache-2.0 OR MIT
- `h2` `v0.3.27` — MIT
- `h2` `v0.4.12` — MIT
- `half` `v2.7.1` — MIT OR Apache-2.0
- `hash32` `v0.3.1` — MIT OR Apache-2.0
- `hashbrown` `v0.12.3` — MIT OR Apache-2.0
- `hashbrown` `v0.13.2` — MIT OR Apache-2.0
- `hashbrown` `v0.14.5` — MIT OR Apache-2.0
- `hashbrown` `v0.15.5` — MIT OR Apache-2.0
- `hashbrown` `v0.16.1` — MIT OR Apache-2.0
- `hashlink` `v0.8.4` — MIT OR Apache-2.0
- `heapless` `v0.8.0` — MIT OR Apache-2.0
- `heck` `v0.5.0` — MIT OR Apache-2.0
- `hex` `v0.4.3` — MIT OR Apache-2.0
- `hmac` `v0.12.1` — MIT OR Apache-2.0
- `home` `v0.5.12` — MIT OR Apache-2.0
- `html5ever` `v0.35.0` — MIT OR Apache-2.0
- `http` `v0.2.12` — MIT OR Apache-2.0
- `http` `v1.4.0` — MIT OR Apache-2.0
- `http-body` `v0.4.6` — MIT
- `http-body` `v1.0.1` — MIT
- `http-body-util` `v0.1.3` — MIT
- `httparse` `v1.10.1` — MIT OR Apache-2.0
- `httpdate` `v1.0.3` — MIT OR Apache-2.0
- `humantime` `v2.3.0` — MIT OR Apache-2.0
- `hyper` `v0.14.32` — MIT
- `hyper` `v1.8.1` — MIT
- `hyper-rustls` `v0.24.2` — Apache-2.0 OR ISC OR MIT
- `hyper-rustls` `v0.27.7` — Apache-2.0 OR ISC OR MIT
- `hyper-timeout` `v0.4.1` — MIT/Apache-2.0
- `hyper-tls` `v0.5.0` — MIT/Apache-2.0
- `hyper-util` `v0.1.18` — MIT
- `icu_collections` `v2.1.1` — Unicode-3.0
- `icu_locale_core` `v2.1.1` — Unicode-3.0
- `icu_normalizer` `v2.1.1` — Unicode-3.0
- `icu_normalizer_data` `v2.1.1` — Unicode-3.0
- `icu_properties` `v2.1.1` — Unicode-3.0
- `icu_properties_data` `v2.1.1` — Unicode-3.0
- `icu_provider` `v2.1.1` — Unicode-3.0
- `ident_case` `v1.0.1` — MIT/Apache-2.0
- `idna` `v0.4.0` — MIT OR Apache-2.0
- `idna` `v1.1.0` — MIT OR Apache-2.0
- `idna_adapter` `v1.2.1` — Apache-2.0 OR MIT
- `if_chain` `v1.0.3` — MIT/Apache-2.0
- `indenter` `v0.3.4` — MIT OR Apache-2.0
- `indexmap` `v1.9.3` — Apache-2.0 OR MIT
- `indexmap` `v2.12.1` — Apache-2.0 OR MIT
- `indicatif` `v0.17.11` — MIT
- `inout` `v0.1.4` — MIT OR Apache-2.0
- `instant` `v0.1.13` — BSD-3-Clause
- `io-extras` `v0.19.0` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `io-lifetimes` `v2.0.4` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `io-lifetimes` `v3.0.1` — Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `ipnet` `v2.11.0` — MIT OR Apache-2.0
- `iri-string` `v0.7.9` — MIT OR Apache-2.0
- `is_terminal_polyfill` `v1.70.2` — MIT OR Apache-2.0
- `itertools` `v0.10.5` — MIT/Apache-2.0
- `itertools` `v0.11.0` — MIT OR Apache-2.0
- `itertools` `v0.12.1` — MIT OR Apache-2.0
- `itertools` `v0.13.0` — MIT OR Apache-2.0
- `itertools` `v0.14.0` — MIT OR Apache-2.0
- `itoa` `v1.0.15` — MIT OR Apache-2.0
- `json5` `v0.4.1` — ISC
- `jsonwebtoken` `v9.3.1` — MIT
- `kdtree` `v0.6.0` — MIT OR Apache-2.0
- `kdtree` `v0.7.0` — MIT OR Apache-2.0
- `keyring` `v2.3.3` — MIT OR Apache-2.0
- `lalrpop-util` `v0.20.2` — Apache-2.0 OR MIT
- `lazy_static` `v1.5.0` — MIT OR Apache-2.0
- `lettre` `v0.11.22` — MIT
- `lexicmp` `v0.1.0` — MIT OR Apache-2.0
- `libc` `v0.2.177` — MIT OR Apache-2.0
- `libm` `v0.2.15` — MIT
- `linfa` `v0.6.1` — MIT/Apache-2.0
- `linfa` `v0.7.1` — MIT OR Apache-2.0
- `linfa-clustering` `v0.6.1` — MIT/Apache-2.0
- `linfa-linalg` `v0.1.0` — MIT/Apache-2.0
- `linfa-nn` `v0.6.1` — MIT/Apache-2.0
- `linfa-nn` `v0.7.2` — MIT OR Apache-2.0
- `linked-hash-map` `v0.5.6` — MIT/Apache-2.0
- `litemap` `v0.8.1` — Unicode-3.0
- `lock_api` `v0.4.14` — MIT OR Apache-2.0
- `log` `v0.4.28` — MIT OR Apache-2.0
- `lru` `v0.12.5` — MIT
- `lz4_flex` `v0.11.5` — MIT
- `mac` `v0.1.1` — MIT/Apache-2.0
- `maplit` `v1.0.2` — MIT/Apache-2.0
- `markup5ever` `v0.35.0` — MIT OR Apache-2.0
- `match_token` `v0.35.0` — (proc-macro) MIT OR Apache-2.0
- `matchers` `v0.2.0` — MIT
- `matchit` `v0.7.3` — MIT AND BSD-3-Clause
- `matrixmultiply` `v0.3.10` — MIT/Apache-2.0
- `maybe-owned` `v0.3.4` — MIT OR Apache-2.0
- `md-5` `v0.10.6` — MIT OR Apache-2.0
- `memchr` `v2.7.6` — Unlicense OR MIT
- `metrics` `v0.21.1` — MIT
- `metrics` `v0.22.4` — MIT
- `metrics-exporter-prometheus` `v0.12.2` — MIT
- `metrics-exporter-prometheus` `v0.13.1` — MIT
- `metrics-macros` `v0.7.1` — (proc-macro) MIT
- `metrics-util` `v0.15.0` — MIT
- `metrics-util` `v0.16.3` — MIT
- `miette` `v5.10.0` — Apache-2.0
- `miette-derive` `v5.10.0` — (proc-macro) Apache-2.0
- `mime` `v0.3.17` — MIT OR Apache-2.0
- `mime_guess` `v2.0.5` — MIT
- `minimal-lexical` `v0.2.1` — MIT/Apache-2.0
- `miniz_oxide` `v0.8.9` — MIT OR Zlib OR Apache-2.0
- `mio` `v1.1.0` — MIT
- `moka` `v0.12.15` — (MIT OR Apache-2.0) AND Apache-2.0
- `multer` `v3.1.0` — MIT
- `nanoid` `v0.4.0` — MIT
- `native-tls` `v0.2.14` — MIT OR Apache-2.0
- `ndarray` `v0.15.6` — MIT OR Apache-2.0
- `ndarray-rand` `v0.14.0` — MIT OR Apache-2.0
- `ndarray-stats` `v0.5.1` — MIT/Apache-2.0
- `new_debug_unreachable` `v1.0.6` — MIT
- `nibble_vec` `v0.1.0` — MIT
- `noisy_float` `v0.2.0` — Apache-2.0
- `nom` `v7.1.3` — MIT
- `nom` `v8.0.0` — MIT
- `ntapi` `v0.4.1` — Apache-2.0 OR MIT
- `nu-ansi-term` `v0.50.3` — MIT
- `num-bigint` `v0.4.6` — MIT OR Apache-2.0
- `num-complex` `v0.4.6` — MIT OR Apache-2.0
- `num-conv` `v0.1.0` — MIT OR Apache-2.0
- `num-integer` `v0.1.46` — MIT OR Apache-2.0
- `num-traits` `v0.2.19` — MIT OR Apache-2.0
- `num_cpus` `v1.17.0` — MIT OR Apache-2.0
- `number_prefix` `v0.4.0` — MIT
- `object_store` `v0.12.4` — MIT/Apache-2.0
- `once_cell` `v1.21.3` — MIT OR Apache-2.0
- `once_cell_polyfill` `v1.70.2` — MIT OR Apache-2.0
- `opaque-debug` `v0.3.1` — MIT OR Apache-2.0
- `opentelemetry` `v0.20.0` — Apache-2.0
- `opentelemetry-otlp` `v0.13.0` — Apache-2.0
- `opentelemetry-proto` `v0.3.0` — Apache-2.0
- `opentelemetry-semantic-conventions` `v0.12.0` — Apache-2.0
- `opentelemetry-semantic-conventions` `v0.14.0` — Apache-2.0
- `opentelemetry_api` `v0.20.0` — Apache-2.0
- `opentelemetry_sdk` `v0.20.0` — Apache-2.0
- `option-ext` `v0.2.0` — MPL-2.0
- `order-stat` `v0.1.3` — MIT/Apache-2.0
- `ordered-float` `v3.9.2` — MIT
- `ordered-float` `v4.6.0` — MIT
- `ordered-multimap` `v0.4.3` — MIT
- `ordered-multimap` `v0.7.3` — MIT
- `owo-colors` `v4.2.3` — MIT
- `parking` `v2.2.1` — Apache-2.0 OR MIT
- `parking_lot` `v0.12.5` — MIT OR Apache-2.0
- `parking_lot_core` `v0.9.12` — MIT OR Apache-2.0
- `partitions` `v0.2.4` — Apache-2.0
- `password-hash` `v0.5.0` — MIT OR Apache-2.0
- `paste` `v1.0.15` — (proc-macro) MIT OR Apache-2.0
- `path-clean` `v1.0.1` — MIT OR Apache-2.0
- `pathdiff` `v0.2.3` — MIT/Apache-2.0
- `pbkdf2` `v0.12.2` — MIT OR Apache-2.0
- `pem` `v3.0.6` — MIT
- `pep440_rs` `v0.7.3` — Apache-2.0 OR BSD-2-Clause
- `pep508_rs` `v0.9.2` — Apache-2.0 OR BSD-2-Clause
- `percent-encoding` `v2.3.2` — MIT OR Apache-2.0
- `pest` `v2.8.4` — MIT OR Apache-2.0
- `pest_derive` `v2.8.4` — (proc-macro) MIT OR Apache-2.0
- `pest_generator` `v2.8.4` — MIT OR Apache-2.0
- `pest_meta` `v2.8.4` — MIT OR Apache-2.0
- `phf` `v0.11.3` — MIT
- `phf_generator` `v0.11.3` — MIT
- `phf_macros` `v0.11.3` — (proc-macro) MIT
- `phf_shared` `v0.11.3` — MIT
- `pin-project` `v1.1.10` — Apache-2.0 OR MIT
- `pin-project-internal` `v1.1.10` — (proc-macro) Apache-2.0 OR MIT
- `pin-project-lite` `v0.2.16` — Apache-2.0 OR MIT
- `pin-utils` `v0.1.0` — MIT OR Apache-2.0
- `polyval` `v0.6.2` — Apache-2.0 OR MIT
- `portable-atomic` `v1.11.1` — Apache-2.0 OR MIT
- `potential_utf` `v0.1.4` — Unicode-3.0
- `powerfmt` `v0.2.0` — MIT OR Apache-2.0
- `ppv-lite86` `v0.2.21` — MIT OR Apache-2.0
- `precomputed-hash` `v0.1.1` — MIT
- `proc-macro-crate` `v3.4.0` — MIT OR Apache-2.0
- `proc-macro-error` `v1.0.4` — MIT OR Apache-2.0
- `proc-macro-error-attr` `v1.0.4` — (proc-macro) MIT OR Apache-2.0
- `proc-macro2` `v1.0.103` — MIT OR Apache-2.0
- `prometheus` `v0.13.4` — Apache-2.0
- `prost` `v0.11.9` — Apache-2.0
- `prost-derive` `v0.11.9` — (proc-macro) Apache-2.0
- `protobuf` `v2.28.0` — MIT
- `psl-types` `v2.0.11` — MIT/Apache-2.0
- `psm` `v0.1.28` — MIT OR Apache-2.0
- `quanta` `v0.11.1` — MIT
- `quanta` `v0.12.6` — MIT
- `quick_cache` `v0.5.2` — MIT
- `quick_cache` `v0.6.18` — MIT
- `quote` `v1.0.42` — MIT OR Apache-2.0
- `quoted_printable` `v0.5.2` — 0BSD
- `radix_trie` `v0.2.1` — MIT
- `rand` `v0.8.5` — MIT OR Apache-2.0
- `rand` `v0.9.2` — MIT OR Apache-2.0
- `rand_chacha` `v0.3.1` — MIT OR Apache-2.0
- `rand_chacha` `v0.9.0` — MIT OR Apache-2.0
- `rand_core` `v0.6.4` — MIT OR Apache-2.0
- `rand_core` `v0.9.3` — MIT OR Apache-2.0
- `rand_distr` `v0.4.3` — MIT OR Apache-2.0
- `rand_xoshiro` `v0.6.0` — MIT OR Apache-2.0
- `ratatui` `v0.26.3` — MIT
- `raw-cpuid` `v10.7.0` — MIT
- `raw-cpuid` `v11.6.0` — MIT
- `rawpointer` `v0.2.1` — MIT/Apache-2.0
- `rayon` `v1.11.0` — MIT OR Apache-2.0
- `rayon-core` `v1.13.0` — MIT OR Apache-2.0
- `reblessive` `v0.4.3` — MIT
- `redb` `v1.5.2` — MIT OR Apache-2.0
- `redis` `v0.24.1` — BSD-3-Clause
- `redis` `v0.25.4` — BSD-3-Clause
- `ref-cast` `v1.0.25` — MIT OR Apache-2.0
- `ref-cast-impl` `v1.0.25` — (proc-macro) MIT OR Apache-2.0
- `regex` `v1.12.2` — MIT OR Apache-2.0
- `regex-automata` `v0.4.13` — MIT OR Apache-2.0
- `regex-syntax` `v0.8.8` — MIT OR Apache-2.0
- `reqwest` `v0.11.27` — MIT OR Apache-2.0
- `reqwest` `v0.12.24` — MIT OR Apache-2.0
- `reqwest-eventsource` `v0.4.0` — MIT OR Apache-2.0
- `revision` `v0.10.0` — Apache-2.0
- `revision` `v0.11.0` — Apache-2.0
- `revision-derive` `v0.10.0` — (proc-macro) Apache-2.0
- `revision-derive` `v0.11.0` — (proc-macro) Apache-2.0
- `rig-core` `v0.3.0` — MIT
- `ring` `v0.17.14` — Apache-2.0 AND ISC
- `rmp` `v0.8.14` — MIT
- `rmp-serde` `v1.3.0` — MIT
- `rmpv` `v1.3.0` — MIT
- `roaring` `v0.10.12` — MIT OR Apache-2.0
- `robust` `v1.2.0` — MIT OR Apache-2.0
- `ron` `v0.7.1` — MIT/Apache-2.0
- `ron` `v0.8.1` — MIT OR Apache-2.0
- `rstar` `v0.12.2` — MIT OR Apache-2.0
- `rust-ini` `v0.18.0` — MIT
- `rust-ini` `v0.20.0` — MIT
- `rust-stemmers` `v1.2.0` — MIT/BSD-3-Clause
- `rust_decimal` `v1.39.0` — MIT
- `rustc-demangle` `v0.1.26` — MIT/Apache-2.0
- `rustc-hash` `v2.1.1` — Apache-2.0 OR MIT
- `rustc_lexer` `v0.1.0` — MIT OR Apache-2.0
- `rustls` `v0.21.12` — Apache-2.0 OR ISC OR MIT
- `rustls` `v0.23.35` — Apache-2.0 OR ISC OR MIT
- `rustls-pemfile` `v1.0.4` — Apache-2.0 OR ISC OR MIT
- `rustls-pki-types` `v1.13.0` — MIT OR Apache-2.0
- `rustls-webpki` `v0.101.7` — ISC
- `rustls-webpki` `v0.103.8` — ISC
- `rustversion` `v1.0.22` — (proc-macro) MIT OR Apache-2.0
- `rustyline` `v13.0.0` — MIT
- `rustyline-derive` `v0.10.0` — (proc-macro) MIT
- `ryu` `v1.0.20` — Apache-2.0 OR BSL-1.0
- `salsa20` `v0.10.2` — MIT OR Apache-2.0
- `same-file` `v1.0.6` — Unlicense/MIT
- `schannel` `v0.1.28` — MIT
- `schemars` `v0.8.22` — MIT
- `schemars_derive` `v0.8.22` — (proc-macro) MIT
- `scopeguard` `v1.2.0` — MIT OR Apache-2.0
- `scrypt` `v0.11.0` — MIT OR Apache-2.0
- `sct` `v0.7.1` — Apache-2.0 OR ISC OR MIT
- `secrecy` `v0.8.0` — Apache-2.0 OR MIT
- `semver` `v1.0.27` — MIT OR Apache-2.0
- `serde` `v1.0.228` — MIT OR Apache-2.0
- `serde-content` `v0.1.2` — MIT/Apache-2.0
- `serde_core` `v1.0.228` — MIT OR Apache-2.0
- `serde_derive` `v1.0.228` — (proc-macro) MIT OR Apache-2.0
- `serde_derive_internals` `v0.29.1` — MIT OR Apache-2.0
- `serde_json` `v1.0.145` — MIT OR Apache-2.0
- `serde_path_to_error` `v0.1.20` — MIT OR Apache-2.0
- `serde_spanned` `v0.6.9` — MIT OR Apache-2.0
- `serde_urlencoded` `v0.7.1` — MIT/Apache-2.0
- `serde_with` `v3.16.0` — MIT OR Apache-2.0
- `serde_with_macros` `v3.16.0` — (proc-macro) MIT OR Apache-2.0
- `sha1` `v0.10.6` — MIT OR Apache-2.0
- `sha1_smol` `v1.0.1` — BSD-3-Clause
- `sha2` `v0.10.9` — MIT OR Apache-2.0
- `sharded-slab` `v0.1.7` — MIT
- `shellexpand` `v3.1.1` — MIT/Apache-2.0
- `simd-adler32` `v0.3.7` — MIT
- `simple_asn1` `v0.6.3` — ISC
- `siphasher` `v1.0.1` — MIT/Apache-2.0
- `sketches-ddsketch` `v0.2.2` — Apache-2.0
- `slab` `v0.4.11` — MIT
- `smallvec` `v1.15.1` — MIT OR Apache-2.0
- `smol_str` `v0.2.2` — MIT OR Apache-2.0
- `snap` `v1.1.1` — BSD-3-Clause
- `socket2` `v0.4.10` — MIT OR Apache-2.0
- `socket2` `v0.5.10` — MIT OR Apache-2.0
- `socket2` `v0.6.1` — MIT OR Apache-2.0
- `space` `v0.12.1` — MIT
- `spade` `v2.15.0` — MIT OR Apache-2.0
- `spin` `v0.9.8` — MIT
- `sprs` `v0.11.1` — MIT OR Apache-2.0
- `stability` `v0.2.1` — (proc-macro) MIT
- `stable_deref_trait` `v1.2.1` — MIT OR Apache-2.0
- `stacker` `v0.1.22` — MIT OR Apache-2.0
- `static_assertions` `v1.1.0` — MIT OR Apache-2.0
- `static_assertions_next` `v1.1.2` — MIT OR Apache-2.0
- `storekey` `v0.5.0` — Apache-2.0
- `string_cache` `v0.8.9` — MIT OR Apache-2.0
- `strsim` `v0.10.0` — MIT
- `strsim` `v0.11.1` — MIT
- `strum` `v0.26.3` — MIT
- `strum_macros` `v0.26.4` — (proc-macro) MIT
- `subtle` `v2.6.1` — BSD-3-Clause
- `surrealdb` `v2.4.0` — UNKNOWN (no license metadata returned by cargo tree)
- `surrealdb-core` `v2.4.0` — UNKNOWN (no license metadata returned by cargo tree)
- `surrealkv` `v0.9.3` — Apache-2.0
- `syn` `v1.0.109` — MIT OR Apache-2.0
- `syn` `v2.0.111` — MIT OR Apache-2.0
- `sync_wrapper` `v0.1.2` — Apache-2.0
- `sync_wrapper` `v1.0.2` — Apache-2.0
- `synstructure` `v0.13.2` — MIT
- `sys-info` `v0.9.1` — MIT
- `sysinfo` `v0.33.1` — MIT
- `tagptr` `v0.2.0` — MIT/Apache-2.0
- `tar` `v0.4.46` — MIT OR Apache-2.0
- `tempfile` `v3.23.0` — MIT OR Apache-2.0
- `tendril` `v0.4.3` — MIT/Apache-2.0
- `thiserror` `v1.0.69` — MIT OR Apache-2.0
- `thiserror` `v2.0.18` — MIT OR Apache-2.0
- `thiserror-impl` `v1.0.69` — (proc-macro) MIT OR Apache-2.0
- `thiserror-impl` `v2.0.18` — (proc-macro) MIT OR Apache-2.0
- `thread_local` `v1.1.9` — MIT OR Apache-2.0
- `time` `v0.3.44` — MIT OR Apache-2.0
- `time-core` `v0.1.6` — MIT OR Apache-2.0
- `time-macros` `v0.2.24` — (proc-macro) MIT OR Apache-2.0
- `tiny-keccak` `v2.0.2` — CC0-1.0
- `tinystr` `v0.8.2` — Unicode-3.0
- `tinyvec` `v1.10.0` — Zlib OR Apache-2.0 OR MIT
- `tinyvec_macros` `v0.1.1` — MIT OR Apache-2.0 OR Zlib
- `tokio` `v1.48.0` — MIT
- `tokio-io-timeout` `v1.2.1` — MIT/Apache-2.0
- `tokio-macros` `v2.6.0` — (proc-macro) MIT
- `tokio-native-tls` `v0.3.1` — MIT
- `tokio-retry` `v0.3.0` — MIT
- `tokio-rustls` `v0.24.1` — MIT/Apache-2.0
- `tokio-rustls` `v0.26.4` — MIT OR Apache-2.0
- `tokio-stream` `v0.1.17` — MIT
- `tokio-tungstenite` `v0.23.1` — MIT
- `tokio-tungstenite` `v0.24.0` — MIT
- `tokio-util` `v0.7.17` — MIT
- `toml` `v0.5.11` — MIT/Apache-2.0
- `toml` `v0.8.23` — MIT OR Apache-2.0
- `toml_datetime` `v0.6.11` — MIT OR Apache-2.0
- `toml_datetime` `v0.7.3` — MIT OR Apache-2.0
- `toml_edit` `v0.22.27` — MIT OR Apache-2.0
- `toml_edit` `v0.23.7` — MIT OR Apache-2.0
- `toml_parser` `v1.0.4` — MIT OR Apache-2.0
- `toml_write` `v0.1.2` — MIT OR Apache-2.0
- `tonic` `v0.9.2` — MIT
- `tower` `v0.4.13` — MIT
- `tower` `v0.5.2` — MIT
- `tower-http` `v0.5.2` — MIT
- `tower-http` `v0.6.7` — MIT
- `tower-layer` `v0.3.3` — MIT
- `tower-service` `v0.3.3` — MIT
- `tracing` `v0.1.41` — MIT
- `tracing-appender` `v0.2.4` — MIT
- `tracing-attributes` `v0.1.30` — (proc-macro) MIT
- `tracing-core` `v0.1.34` — MIT
- `tracing-error` `v0.2.1` — MIT
- `tracing-log` `v0.2.0` — MIT
- `tracing-serde` `v0.2.0` — MIT
- `tracing-subscriber` `v0.3.20` — MIT
- `trice` `v0.4.0` — Apache-2.0
- `try-lock` `v0.2.5` — MIT
- `tungstenite` `v0.23.0` — MIT OR Apache-2.0
- `tungstenite` `v0.24.0` — MIT OR Apache-2.0
- `twox-hash` `v2.1.2` — MIT
- `typenum` `v1.19.0` — MIT OR Apache-2.0
- `ucd-trie` `v0.1.7` — MIT OR Apache-2.0
- `ulid` `v1.2.1` — MIT
- `unicase` `v2.8.1` — MIT OR Apache-2.0
- `unicode-bidi` `v0.3.18` — MIT OR Apache-2.0
- `unicode-ident` `v1.0.22` — (MIT OR Apache-2.0) AND Unicode-3.0
- `unicode-normalization` `v0.1.25` — MIT OR Apache-2.0
- `unicode-script` `v0.5.7` — MIT OR Apache-2.0
- `unicode-security` `v0.1.2` — MIT/Apache-2.0
- `unicode-segmentation` `v1.12.0` — MIT OR Apache-2.0
- `unicode-truncate` `v1.1.0` — MIT OR Apache-2.0
- `unicode-width` `v0.1.14` — MIT OR Apache-2.0
- `unicode-width` `v0.2.2` — MIT OR Apache-2.0
- `unicode-xid` `v0.2.6` — MIT OR Apache-2.0
- `universal-hash` `v0.5.1` — MIT OR Apache-2.0
- `unscanny` `v0.1.0` — MIT OR Apache-2.0
- `untrusted` `v0.9.0` — ISC
- `url` `v2.5.7` — MIT OR Apache-2.0
- `urlencoding` `v2.1.3` — MIT
- `utf-8` `v0.7.6` — MIT OR Apache-2.0
- `utf8_iter` `v1.0.4` — Apache-2.0 OR MIT
- `utf8parse` `v0.2.2` — Apache-2.0 OR MIT
- `uuid` `v1.18.1` — Apache-2.0 OR MIT
- `validator` `v0.16.1` — MIT
- `validator_derive` `v0.16.0` — (proc-macro) MIT
- `validator_types` `v0.16.0` — MIT
- `vart` `v0.8.1` — Apache-2.0
- `vart` `v0.9.3` — Apache-2.0
- `version-ranges` `v0.1.3` — MPL-2.0
- `walkdir` `v2.5.0` — Unlicense/MIT
- `want` `v0.3.1` — MIT
- `web_atoms` `v0.1.3` — MIT OR Apache-2.0
- `webpki-roots` `v0.25.4` — MPL-2.0
- `webpki-roots` `v0.26.11` — CDLA-Permissive-2.0
- `webpki-roots` `v1.0.4` — CDLA-Permissive-2.0
- `winapi` `v0.3.9` — MIT/Apache-2.0
- `winapi-util` `v0.1.11` — Unlicense OR MIT
- `windows` `v0.57.0` — MIT OR Apache-2.0
- `windows-core` `v0.57.0` — MIT OR Apache-2.0
- `windows-implement` `v0.57.0` — (proc-macro) MIT OR Apache-2.0
- `windows-interface` `v0.57.0` — (proc-macro) MIT OR Apache-2.0
- `windows-link` `v0.2.1` — MIT OR Apache-2.0
- `windows-result` `v0.1.2` — MIT OR Apache-2.0
- `windows-sys` `v0.48.0` — MIT OR Apache-2.0
- `windows-sys` `v0.52.0` — MIT OR Apache-2.0
- `windows-sys` `v0.59.0` — MIT OR Apache-2.0
- `windows-sys` `v0.60.2` — MIT OR Apache-2.0
- `windows-sys` `v0.61.2` — MIT OR Apache-2.0
- `windows-targets` `v0.48.5` — MIT OR Apache-2.0
- `windows-targets` `v0.52.6` — MIT OR Apache-2.0
- `windows-targets` `v0.53.5` — MIT OR Apache-2.0
- `windows_x86_64_msvc` `v0.48.5` — MIT OR Apache-2.0
- `windows_x86_64_msvc` `v0.52.6` — MIT OR Apache-2.0
- `windows_x86_64_msvc` `v0.53.1` — MIT OR Apache-2.0
- `winnow` `v0.5.40` — MIT
- `winnow` `v0.7.13` — MIT
- `winreg` `v0.50.0` — MIT
- `winx` `v0.36.4` — Apache-2.0 WITH LLVM-exception
- `writeable` `v0.6.2` — Unicode-3.0
- `yaml-rust` `v0.4.5` — MIT/Apache-2.0
- `yaml-rust2` `v0.8.1` — MIT OR Apache-2.0
- `yoke` `v0.8.1` — Unicode-3.0
- `yoke-derive` `v0.8.1` — (proc-macro) Unicode-3.0
- `zerocopy` `v0.8.30` — BSD-2-Clause OR Apache-2.0 OR MIT
- `zerocopy-derive` `v0.8.30` — (proc-macro) BSD-2-Clause OR Apache-2.0 OR MIT
- `zerofrom` `v0.1.6` — Unicode-3.0
- `zerofrom-derive` `v0.1.6` — (proc-macro) Unicode-3.0
- `zeroize` `v1.8.2` — Apache-2.0 OR MIT
- `zerotrie` `v0.2.3` — Unicode-3.0
- `zerovec` `v0.11.5` — Unicode-3.0
- `zerovec-derive` `v0.11.2` — (proc-macro) Unicode-3.0

## Full WebUI production dependency inventory

`pnpm licenses list --prod`, run from `webui/`. This lists only packages that
ship in the built application (the `dependencies` graph) — build- and
test-only tooling (ESLint, Prettier, Playwright, Vitest, TypeScript, Husky,
and their transitive dependencies) is intentionally excluded because it is
never distributed to an end user.

- `pako` — (MIT AND Zlib)
- `tslib` — 0BSD
- `@playwright/test` — Apache-2.0
- `@swc/helpers` — Apache-2.0
- `baseline-browser-mapping` — Apache-2.0
- `class-variance-authority` — Apache-2.0
- `detect-libc` — Apache-2.0
- `playwright` — Apache-2.0
- `playwright-core` — Apache-2.0
- `sharp` — Apache-2.0
- `@img/sharp-win32-x64` — Apache-2.0 AND LGPL-3.0-or-later
- `d3-ease` — BSD-3-Clause
- `source-map-js` — BSD-3-Clause
- `caniuse-lite` — CC-BY-4.0
- `cliui` — ISC
- `d3-array` — ISC
- `d3-color` — ISC
- `d3-format` — ISC
- `d3-interpolate` — ISC
- `d3-path` — ISC
- `d3-scale` — ISC
- `d3-shape` — ISC
- `d3-time` — ISC
- `d3-time-format` — ISC
- `d3-timer` — ISC
- `electron-to-chromium` — ISC
- `get-caller-file` — ISC
- `internmap` — ISC
- `lru-cache` — ISC
- `lucide-react` — ISC
- `picocolors` — ISC
- `require-main-filename` — ISC
- `semver` — ISC
- `set-blocking` — ISC
- `which-module` — ISC
- `y18n` — ISC
- `yallist` — ISC
- `yargs-parser` — ISC
- `@babel/code-frame` — MIT
- `@babel/compat-data` — MIT
- `@babel/core` — MIT
- `@babel/generator` — MIT
- `@babel/helper-compilation-targets` — MIT
- `@babel/helper-globals` — MIT
- `@babel/helper-module-imports` — MIT
- `@babel/helper-module-transforms` — MIT
- `@babel/helper-string-parser` — MIT
- `@babel/helper-validator-identifier` — MIT
- `@babel/helper-validator-option` — MIT
- `@babel/helpers` — MIT
- `@babel/parser` — MIT
- `@babel/template` — MIT
- `@babel/traverse` — MIT
- `@babel/types` — MIT
- `@floating-ui/core` — MIT
- `@floating-ui/dom` — MIT
- `@floating-ui/react-dom` — MIT
- `@floating-ui/utils` — MIT
- `@img/colour` — MIT
- `@jridgewell/gen-mapping` — MIT
- `@jridgewell/remapping` — MIT
- `@jridgewell/resolve-uri` — MIT
- `@jridgewell/sourcemap-codec` — MIT
- `@jridgewell/trace-mapping` — MIT
- `@next/env` — MIT
- `@next/swc-win32-x64-msvc` — MIT
- `@pdf-lib/standard-fonts` — MIT
- `@pdf-lib/upng` — MIT
- `@radix-ui/number` — MIT
- `@radix-ui/primitive` — MIT
- `@radix-ui/react-accessible-icon` — MIT
- `@radix-ui/react-accordion` — MIT
- `@radix-ui/react-alert-dialog` — MIT
- `@radix-ui/react-arrow` — MIT
- `@radix-ui/react-aspect-ratio` — MIT
- `@radix-ui/react-avatar` — MIT
- `@radix-ui/react-checkbox` — MIT
- `@radix-ui/react-collapsible` — MIT
- `@radix-ui/react-collection` — MIT
- `@radix-ui/react-compose-refs` — MIT
- `@radix-ui/react-context` — MIT
- `@radix-ui/react-context-menu` — MIT
- `@radix-ui/react-dialog` — MIT
- `@radix-ui/react-direction` — MIT
- `@radix-ui/react-dismissable-layer` — MIT
- `@radix-ui/react-dropdown-menu` — MIT
- `@radix-ui/react-focus-guards` — MIT
- `@radix-ui/react-focus-scope` — MIT
- `@radix-ui/react-form` — MIT
- `@radix-ui/react-hover-card` — MIT
- `@radix-ui/react-id` — MIT
- `@radix-ui/react-label` — MIT
- `@radix-ui/react-menu` — MIT
- `@radix-ui/react-menubar` — MIT
- `@radix-ui/react-navigation-menu` — MIT
- `@radix-ui/react-one-time-password-field` — MIT
- `@radix-ui/react-password-toggle-field` — MIT
- `@radix-ui/react-popover` — MIT
- `@radix-ui/react-popper` — MIT
- `@radix-ui/react-portal` — MIT
- `@radix-ui/react-presence` — MIT
- `@radix-ui/react-primitive` — MIT
- `@radix-ui/react-progress` — MIT
- `@radix-ui/react-radio-group` — MIT
- `@radix-ui/react-roving-focus` — MIT
- `@radix-ui/react-scroll-area` — MIT
- `@radix-ui/react-select` — MIT
- `@radix-ui/react-separator` — MIT
- `@radix-ui/react-slider` — MIT
- `@radix-ui/react-slot` — MIT
- `@radix-ui/react-switch` — MIT
- `@radix-ui/react-tabs` — MIT
- `@radix-ui/react-toast` — MIT
- `@radix-ui/react-toggle` — MIT
- `@radix-ui/react-toggle-group` — MIT
- `@radix-ui/react-toolbar` — MIT
- `@radix-ui/react-tooltip` — MIT
- `@radix-ui/react-use-callback-ref` — MIT
- `@radix-ui/react-use-controllable-state` — MIT
- `@radix-ui/react-use-effect-event` — MIT
- `@radix-ui/react-use-escape-keydown` — MIT
- `@radix-ui/react-use-is-hydrated` — MIT
- `@radix-ui/react-use-layout-effect` — MIT
- `@radix-ui/react-use-previous` — MIT
- `@radix-ui/react-use-rect` — MIT
- `@radix-ui/react-use-size` — MIT
- `@radix-ui/react-visually-hidden` — MIT
- `@radix-ui/rect` — MIT
- `@reduxjs/toolkit` — MIT
- `@standard-schema/spec` — MIT
- `@standard-schema/utils` — MIT
- `@tanstack/query-core` — MIT
- `@tanstack/react-query` — MIT
- `@tanstack/react-table` — MIT
- `@tanstack/table-core` — MIT
- `@types/d3-array` — MIT
- `@types/d3-color` — MIT
- `@types/d3-ease` — MIT
- `@types/d3-interpolate` — MIT
- `@types/d3-path` — MIT
- `@types/d3-scale` — MIT
- `@types/d3-shape` — MIT
- `@types/d3-time` — MIT
- `@types/d3-timer` — MIT
- `@types/react` — MIT
- `@types/react-dom` — MIT
- `@types/use-sync-external-store` — MIT
- `ansi-regex` — MIT
- `ansi-styles` — MIT
- `aria-hidden` — MIT
- `browserslist` — MIT
- `camelcase` — MIT
- `classnames` — MIT
- `client-only` — MIT
- `clsx` — MIT
- `cmdk` — MIT
- `color-convert` — MIT
- `color-name` — MIT
- `convert-source-map` — MIT
- `csstype` — MIT
- `debug` — MIT
- `decamelize` — MIT
- `decimal.js-light` — MIT
- `detect-node-es` — MIT
- `dijkstrajs` — MIT
- `emoji-regex` — MIT
- `es-toolkit` — MIT
- `escalade` — MIT
- `eventemitter3` — MIT
- `find-up` — MIT
- `gensync` — MIT
- `get-nonce` — MIT
- `immer` — MIT
- `is-fullwidth-code-point` — MIT
- `js-tokens` — MIT
- `jsbarcode` — MIT
- `jsesc` — MIT
- `json5` — MIT
- `lenis` — MIT
- `libphonenumber-js` — MIT
- `locate-path` — MIT
- `ms` — MIT
- `nanoid` — MIT
- `next` — MIT
- `next-themes` — MIT
- `node-releases` — MIT
- `p-limit` — MIT
- `p-locate` — MIT
- `p-try` — MIT
- `path-exists` — MIT
- `pdf-lib` — MIT
- `pngjs` — MIT
- `postcss` — MIT
- `preact` — MIT
- `qrcode` — MIT
- `radix-ui` — MIT
- `react` — MIT
- `react-dom` — MIT
- `react-is` — MIT
- `react-redux` — MIT
- `react-remove-scroll` — MIT
- `react-remove-scroll-bar` — MIT
- `react-style-singleton` — MIT
- `recharts` — MIT
- `redux` — MIT
- `redux-thunk` — MIT
- `require-directory` — MIT
- `reselect` — MIT
- `scheduler` — MIT
- `string-width` — MIT
- `strip-ansi` — MIT
- `styled-jsx` — MIT
- `tailwind-merge` — MIT
- `three` — MIT
- `tiny-invariant` — MIT
- `update-browserslist-db` — MIT
- `use-callback-ref` — MIT
- `use-sidecar` — MIT
- `use-sync-external-store` — MIT
- `wrap-ansi` — MIT
- `yargs` — MIT
- `zod` — MIT
- `victory-vendor` — MIT AND ISC
- `gsap` — Standard 'no charge' license: https://gsap.com/standard-license.
