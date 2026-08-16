# GaussMeridian — Codebase Report

> **Prepared for:** new-contributor and AI-agent onboarding  
> **Generated:** 2026-04-04  
> **Codebase version:** workspace `3.0.0` (docs reference `3.1.0` in places)  
> **Status:** Legacy handover — not yet verified to compile or run by current maintainer

---

## Report Structure

| # | File | What It Covers |
| --- | --- | --- |
| 01 | [PROJECT-OVERVIEW.md](./01-PROJECT-OVERVIEW.md) | What GaussMeridian is, goals, tech stack, high-level architecture |
| 02 | [DIRECTORY-STRUCTURE.md](./02-DIRECTORY-STRUCTURE.md) | Full repo tree with annotations |
| 03 | [RUST-BACKEND.md](./03-RUST-BACKEND.md) | Cargo workspace, crates, API endpoints, auth, routing, providers, DB |
| 04 | [FRONTEND-WEBUI.md](./04-FRONTEND-WEBUI.md) | Next.js app, pages, components, state management, API client |
| 05 | [INFRASTRUCTURE.md](./05-INFRASTRUCTURE.md) | Docker Compose, Dockerfile, CI/CD, monitoring, load tests |
| 06 | [SETUP-GUIDE.md](./06-SETUP-GUIDE.md) | Step-by-step: how to install deps, configure, and run locally |
| 07 | [KNOWN-ISSUES.md](./07-KNOWN-ISSUES.md) | Contradictions in docs, unfinished features, anti-patterns, risks |
| 08 | [GLOSSARY.md](./08-GLOSSARY.md) | Key terms, acronyms, and domain concepts |
| 09 | [MVP-HUMAN-STATUS.md](./09-MVP-HUMAN-STATUS.md) | Human-readable M1 status vs the approved MVP build plan |

---

## How to Use This Report

1. **New to the project?** Read files 01 → 02 → 03 → 04 in order for a full picture.
2. **Want to run it?** Jump to `06-SETUP-GUIDE.md`.
3. **Looking for bugs or tech debt?** See `07-KNOWN-ISSUES.md`.
4. **Checking MVP execution status?** Read `09-MVP-HUMAN-STATUS.md` before starting M1 work.
5. **Full onboarding:** Read all files sequentially. Each is self-contained but cross-references the others.

---

## Important Disclaimer

This report was generated from **static code analysis only**. The project has **not been compiled or run** as part of this review. Some findings (especially around build health) need to be verified by actually running `cargo check` and `npm run build`.
