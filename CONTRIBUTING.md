# Contributing to Native GaussMeridian

Thank you for helping improve GaussMeridian. The supported community path is the
native Rust application in this repository. Keep changes small, test the failure
path as well as the happy path, and preserve the existing Meridian, BELLA, CARROT,
R2, and provider boundaries.

The detailed lifecycle, architecture map, qualification matrix, and troubleshooting
guide live in
[`docs/operations/native-community-preview.md`](docs/operations/native-community-preview.md).

## Licensing of contributions

GaussMeridian is licensed under the **GNU Affero General Public License v3.0
only** (`AGPL-3.0-only`). Read this section before you open your first pull
request — it is short, and it is the one thing here that is hard to undo later.

**Inbound = outbound.** By opening a pull request against this repository you
license your contribution under AGPL-3.0-only, on the same terms as the rest of
the project. You keep the copyright in what you wrote; you are granting the
project and everyone downstream the right to use it under that license. There is
no copyright assignment and no CLA — instead, sign off your commits under the
Developer Certificate of Origin described next.

### Sign your commits (DCO)

Every commit must carry a `Signed-off-by` line. Git adds it for you:

```text
git commit -s -m "fix: your message"
```

which appends:

```text
Signed-off-by: Your Name <your.email@example.com>
```

Use your real name and an email you can be reached at. Adding that line means
you certify the Developer Certificate of Origin 1.1, reproduced in full below.
It is not a copyright assignment and it grants the project nothing beyond what
the AGPL already covers — it is a statement, on the record, that you had the
right to contribute what you contributed.

Forgot to sign off? `git commit --amend -s` fixes the last commit;
`git rebase --signoff main` fixes a branch.

**One consequence worth understanding before you sign off.** This project
carries an AGPL Section 7 additional permission — a linking exception for two
GPL-incompatible dependencies, recorded in [`NOTICE`](NOTICE). Contributing
under AGPL-3.0-only means your contribution may be conveyed as part of a
combined work under that permission. If that is not acceptable to you, say so in
the pull request rather than signing off.

<details>
<summary><strong>Developer Certificate of Origin 1.1</strong> (full text)</summary>

```text
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.


Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

</details>

**Only contribute code you can license this way.** Specifically:

- Code you wrote yourself, or that your employer has agreed you may contribute.
  If you are contributing work done on company time or equipment, settle your
  employer's position before you open the PR — that is between you and them, and
  the project cannot resolve it for you.
- Code copied from elsewhere only if its license is compatible with AGPL-3.0 and
  you preserve its notices. MIT, BSD, ISC, Apache-2.0, MPL-2.0, and the
  GPL/LGPL/AGPL family are compatible. Say where it came from in the PR.
- **Never** paste in code under a proprietary, source-available, or
  non-commercial license — BUSL/BSL, SSPL, Elastic License, Commons Clause,
  CC-BY-NC, or anything a vendor calls "free for non-commercial use." These
  cannot be relicensed under AGPL, and their presence would make the whole
  distribution non-compliant.

**Dependencies follow the same rule, and the rule changed.** While this project
was Apache-2.0, AGPL and GPL crates were banned outright. Under AGPL-3.0 that ban
is lifted: GPL-2.0-or-later, GPL-3.0, LGPL, and AGPL-3.0 dependencies are now
permitted. What is banned instead is anything **GPL-incompatible** — the
proprietary and source-available licenses listed above, plus GPL-2.0-**only**,
which cannot be combined with AGPL-3.0 code. Check the license of any new
dependency before adding it, note it in the PR, and regenerate
`THIRD_PARTY_NOTICES.md` if the dependency graph moved. A dependency whose
license metadata is missing or unreadable counts as unconfirmed, not as
permitted.

**If your change is network-visible, check Section 13.** AGPL Section 13 requires
that users interacting with a running instance over a network be offered its
Corresponding Source. The gateway serves that offer from `SOURCE_OFFER_URL` and
the WebUI from `NEXT_PUBLIC_SOURCE_OFFER_URL`. If you add a new user-facing
surface — a new frontend route group, a new served application, a new public HTTP
entry point — carry the offer into it rather than leaving the new surface silent.

## Prerequisites

From a terminal, verify the tools before changing code:

```text
git --version
docker --version
docker compose version
rustc --version
cargo --version
python --version
```

Use a current stable Rust toolchain, Python 3.11 or newer, and Docker with the
Compose plugin. The local preview needs no commercial provider credentials.

## Clone and branch

```text
git clone https://github.com/Gaussian-id/gaussmeridian.git
cd gauss-meridian
git switch -c feat/your-topic
```

Do not develop directly on `main`. A pull request should contain one reviewable
change and its tests.

## Run the native preview

PowerShell, from the repository root, for the first manual start of a fresh preview
database volume:

```powershell
$env:NATIVE_PREVIEW_SOURCE_COMMIT = git rev-parse HEAD
$env:NATIVE_PREVIEW_DB_PASSWORD = python -c "import secrets; print(secrets.token_urlsafe(32))"
$env:NATIVE_PREVIEW_JWT_SECRET = python -c "import secrets; print(secrets.token_urlsafe(48))"
$env:NATIVE_PREVIEW_PROVIDER_TOKEN = python -c "import secrets; print(secrets.token_urlsafe(32))"
docker compose --project-name gaussmeridian-native-preview --file docker-compose.native-preview.yml up --detach --build --wait --remove-orphans
Invoke-RestMethod http://127.0.0.1:8020/health
Invoke-RestMethod http://127.0.0.1:8020/ready
```

POSIX shell, for that same first-start condition:

```bash
export NATIVE_PREVIEW_SOURCE_COMMIT="$(git rev-parse HEAD)"
export NATIVE_PREVIEW_DB_PASSWORD="$(python -c 'import secrets; print(secrets.token_urlsafe(32))')"
export NATIVE_PREVIEW_JWT_SECRET="$(python -c 'import secrets; print(secrets.token_urlsafe(48))')"
export NATIVE_PREVIEW_PROVIDER_TOKEN="$(python -c 'import secrets; print(secrets.token_urlsafe(32))')"
docker compose --project-name gaussmeridian-native-preview --file docker-compose.native-preview.yml up --detach --build --wait --remove-orphans
curl --fail --silent http://127.0.0.1:8020/health
curl --fail --silent http://127.0.0.1:8020/ready
```

The first build can take longer while Docker downloads and compiles dependencies.
After those assets are cached, this is a self-contained local path backed by the
repository's deterministic provider simulator.
These values are local-only and are never release or paid-provider credentials. The
database password is bound to the retained database volume: save it in your local
password manager and restore that exact value in later shells. Do not rerun the database
password line against an existing volume. JWT and provider-fixture values may rotate in
each shell. The full qualifier manages database-password reuse automatically through its
secure, Git-ignored `.runtime/native-preview-credentials.json`; never commit any of these
values.

Stop only this preview and retain its diagnostic data:

```text
docker compose --project-name gaussmeridian-native-preview --file docker-compose.native-preview.yml down --remove-orphans
```

## Test before opening a pull request

Run Rust checks from the workspace root:

```text
cd gaussmeridian
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Existing repository-wide formatting or lint debt is retained as an explicit
inherited failure, tracked separately, only when it does not introduce a new
diagnostic. A formatting path that also changed in your commit is only excusable as
inherited when formatting both the base and your change proves the delta itself is
format-neutral. Any new formatting or lint diagnostic your change introduces should
be fixed before opening the pull request; do not bulk-format unrelated files to clear
pre-existing debt.

## Pull-request checklist

- Explain the user-visible outcome and the failure behavior.
- Add a regression test before or with the implementation.
- Keep policy, orchestration, persistence, and provider changes in their owning modules.
- Include the exact commands run and their results.
- Do not commit secrets, local database contents, build outputs, or unrelated edits.
- Every commit signed off (`git commit -s`) under the DCO.
- Confirm you can license the change under AGPL-3.0-only, and name the source of
  any code you did not write yourself.
- If you added or bumped a dependency: state its license, confirm it is
  GPL-compatible, and regenerate `THIRD_PARTY_NOTICES.md`.
- If you added a new network-facing surface: confirm it carries the Section 13
  source offer.
- Treat a passing controlled matrix as evidence for that exact commit, not as a claim
  of universal crash freedom or production readiness.
