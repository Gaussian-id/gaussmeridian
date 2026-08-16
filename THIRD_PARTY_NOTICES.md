# Third-Party Notices

GaussMeridian is licensed under the Apache License, Version 2.0 (see `LICENSE`). This file lists
the third-party open-source components distributed with GaussMeridian, grouped by license.

**Provenance of this list:** compiled from a manual dependency review (direct dependencies of the
in-scope Rust crates and the webui's direct `package.json` dependencies), cross-checked against each
package's own `package.json` `license` field where installed locally, and against public crates.io/
npm listings otherwise. This is **not** the output of an automated SBOM tool and should be verified
with one (`cargo about generate` or `cargo license` for Rust; `pnpm licenses list` or
`license-checker` for the webui, including transitive dependencies, which this list does not cover)
before a final release. Four Rust entries below are marked "unverified" — public-knowledge best
guesses that should be confirmed by an automated tool rather than trusted as-is.

## Requires individual review before publication

These do not use a standard permissive license and need their own compatibility/attribution
decision — do not treat them as covered by the blanket permissive-license notice below.

- **gsap** (JavaScript, webui, shipped in the browser bundle) — Standard "No Charge" GSAP License
  (`https://gsap.com/standard-license`). Free for commercial use; not an OSI-approved open-source
  license. See GSAP's own license text for the exact terms governing redistribution of applications
  that bundle it.
- **@axe-core/playwright** (JavaScript, webui, development/test dependency only — not shipped in the
  built application) — Mozilla Public License 2.0 (MPL-2.0). File-level weak copyleft; not
  distributed as part of the product.

## Apache License 2.0

- prometheus, tracing-opentelemetry, opentelemetry, opentelemetry-otlp,
  opentelemetry-semantic-conventions, opentelemetry-stdout, opentelemetry-jaeger (Rust)
- class-variance-authority, @playwright/test, newman, typescript (JavaScript)

## MIT License

- tokio, tokio-util, tokio-stream, tower, tower-http, async-stream, dotenvy, tracing,
  tracing-subscriber, tracing-appender, dashmap, moka, sha2 (dual MIT/Apache-2.0 — see below),
  hmac, jsonwebtoken, bcrypt, metrics, metrics-exporter-prometheus, ratatui, crossterm, tokio-test,
  hex, lz4, zstd, bytes, rig-core, rustyline, shellexpand, indicatif, console, dotenv, sys-info,
  cached, winnow, bincode, validator, test-case, libloading (ISC — see below) (Rust)
- @tanstack/react-query, @tanstack/react-table, clsx, cmdk, lenis, next, next-themes, pdf-lib,
  qrcode, radix-ui, react, react-dom, recharts, tailwind-merge, three, [see note below], zod,
  @tailwindcss/postcss, @testing-library/dom, @testing-library/jest-dom, @testing-library/react,
  @testing-library/user-event, @types/newman, @types/node, @types/qrcode, @types/react,
  @types/react-dom, @types/three, @vitejs/plugin-react, eslint, eslint-config-next,
  eslint-plugin-import, eslint-plugin-jsx-a11y, husky, jsdom, lint-staged, prettier,
  prettier-plugin-tailwindcss, tailwindcss, vite-tsconfig-paths, vitest (JavaScript)

## MIT OR Apache-2.0 (dual-licensed)

serde, serde_json, uuid, chrono, reqwest, reqwest-eventsource, futures, pin-project, config, clap,
parking_lot, rayon, redis (BSD-3-Clause — see below), deadpool-redis, sha2, base64, url, secrecy,
aes-gcm, aes, argon2, thiserror, anyhow, dynamic_reload, mockall, wiremock, rand, rand_distr,
tempfile, async-trait, flate2, toml, fastrand, regex, lazy_static, redb, ndarray, linfa,
linfa-clustering, linfa-nn, num-traits, directories, once_cell, keyring, zeroize, num_cpus, syn,
quote, proc-macro2, approx (unverified — confirm with a license tool), semver, tikv-jemallocator,
cc, criterion, pretty_assertions, proptest, fake (Rust)

## BSD-3-Clause

redis (Rust crate), snap, brotli (BSD-3-Clause OR MIT) (Rust)

## ISC License

libloading, lucide-react (Rust / JavaScript)

## Believed permissive, unverified — confirm with an automated license tool

rustyline-derive (believed MIT), lz4_flex (believed MIT), test-log (believed MIT OR Apache-2.0)
(Rust)

## Note on the payment-checkout UI package

One `webui/package.json` dependency (a hosted-checkout UI SDK, MIT-licensed — see
`webui/package.json` for its exact package name) is deliberately not spelled out by name in this
file. The dependency itself still ships in `package.json` per the maintainers' decision to keep
model-pricing code working now rather than delay this release on a backend/frontend split (see the
project's internal STACK TRUTH classification notes — not part of this public candidate).

This project's private pre-publication scanner flags that provider's name as a protected term
anywhere it appears in candidate text; `webui/package.json` itself is registered as a reviewed,
scoped exception for that one rule in `scan-policy.json` (`protected-payment-provider.allow_paths`),
so the real scan will not flag it. This file still avoids repeating the name in prose, since the
exception is scoped to that exact file path and does not cover this document.

## Provenance note — gaussmoa

The `gaussmeridian-moa` crate (package name `gaussmoa`) was originally authored by Risman Adnan,
GaussMeridian's own original author, as in-house work — not a third-party dependency. It is licensed
Apache License, Version 2.0, the same as the rest of this project, and is covered by the primary
`LICENSE` and `NOTICE` files rather than this third-party list.

## Regenerating this file

Rust: `cargo install cargo-about && cargo about generate` (or `cargo install cargo-license &&
cargo license`), scoped to the crates actually shipped in this candidate.

JavaScript: `pnpm licenses list` (or `npx license-checker`) from `webui/`, which also covers
transitive dependencies that this manual pass does not.
