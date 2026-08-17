# Governance

GaussMeridian is pre-1.0 and currently maintained under a **single-maintainer
model**: the project owner has final authority over scope, architecture, and
release decisions. This document describes how that works today and how it
is expected to change as the project and its contributor base grow.

## Decision-making

- **Project owner.** Holds final approval over architecture decisions,
  provider-catalog changes, security-sensitive changes (authentication,
  BYOK key handling, secrets), release timing, and anything that changes
  what public claims this project makes about itself (efficacy, performance,
  readiness).
- **Maintainers / reviewers.** Review and merge day-to-day contributions —
  bug fixes, documentation, test coverage, non-architectural features —
  within the boundaries the project owner has already set.
- **Contributors.** Anyone who opens a pull request following
  [`CONTRIBUTING.md`](CONTRIBUTING.md). Contributions are reviewed against
  this repository's own qualification and test gates before merge; passing
  those gates does not by itself authorize a scope, architecture, or
  provider-catalog change — those still require project-owner sign-off.

## Release gates

Certain changes are explicitly gated and require project-owner approval
before they take effect, regardless of who authored the change:

- **Provider catalog** — which models/providers are enabled by default and
  what the routing catalog allows.
- **The license itself.** GaussMeridian is AGPL-3.0-only. Changing the
  outbound license — to a different license, to a dual-license or open-core
  split, or to a permissive carve-out for any subset of crates — is an
  owner-only decision. It is also, in practice, a one-way door: with no CLA and
  no copyright assignment, every outside contributor retains copyright in their
  contribution, so a future relicense would need each of them to agree. Treat
  the license line in `gaussmeridian/Cargo.toml` and `webui/package.json` as
  owner-controlled, not as ordinary metadata.
- **Legal review (governance gate G3)** — `LICENSE`, `NOTICE`, and
  `THIRD_PARTY_NOTICES.md` are generated and kept current by contributors,
  but their legal sufficiency for a public release is a separate,
  owner-approved review, not automatically granted by merging a PR. The
  relicense from Apache-2.0 to AGPL-3.0-only, the AGPL-compatibility of every
  bundled dependency, and the Section 7(e) trademark term in `TRADEMARKS.md`
  all sit inside G3's scope, and none has cleared it yet.
- **AGPL Section 13 coverage.** Every network-facing surface must offer its
  Corresponding Source to the users who reach it. Removing, disabling, or
  failing to extend that offer to a newly added surface is a compliance
  regression, not a product decision, and is gated accordingly.
- **The AGPL Section 7 additional permission** in `NOTICE` — the linking
  exception for GSAP and the SurrealDB client. Widening it to cover a new
  GPL-incompatible dependency is an owner decision, not a consequence of merging
  the dependency, and the reviewing question is always whether the dependency
  can be avoided instead. Narrowing it, once a covered dependency is removed, is
  encouraged and needs no gate.
- **Public repository visibility (governance gate G6)** — when and whether
  this repository (or a given branch/tag of it) is made publicly visible is
  an explicit project-owner decision, tracked separately from code
  readiness.
- **Efficacy, benchmark, and market-comparison claims** — no change may add
  a representative-efficacy claim, a performance benchmark presented as
  measured fact, or a comparison against a named competitor without
  project-owner sign-off. See [`ROADMAP.md`](ROADMAP.md) for what is
  currently in scope versus explicitly deferred.

## Proposing a change to this document

Open a pull request against `GOVERNANCE.md` explaining the proposed change
and why. As with any governance change, the project owner has final approval.

## Where this is headed

As the contributor base grows, the intent is to move toward a more
distributed maintainer model with defined areas of ownership (routing core,
providers, WebUI, infrastructure). That transition has not happened yet —
this document will be updated when it does, not before.
