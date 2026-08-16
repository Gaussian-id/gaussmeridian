# Contributing

Thank you for helping build GaussMeridian. The project is in Technical Preview preparation, so interfaces may change while maintainers establish the first supported baseline.

## Before opening work

1. Search existing issues and discussions.
2. Use an issue for bugs or bounded feature requests.
3. Start a discussion before broad architecture, protocol, storage, or compatibility changes.
4. Keep credentials, customer data, and private infrastructure details out of all issues, logs, fixtures, and commits.

Security reports follow [SECURITY.md](SECURITY.md), not the public issue tracker.

## Development checks

API checks run from `gaussrouter/`:

```shell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Console checks run from `webui/`:

```shell
pnpm install --frozen-lockfile
pnpm format:check
pnpm typecheck
pnpm lint
pnpm test
pnpm build
```

The frozen release candidate may add stricter component-specific commands. CI is authoritative.

## Pull requests

- Make one coherent change per pull request.
- Add or update tests for changed behavior.
- Update public documentation when behavior or configuration changes.
- Explain compatibility, security, persistence, and deployment effects.
- Keep generated output and local state out of commits.
- Confirm every dependency has a compatible license and a clear purpose.
- Accept maintainer requests for scope reduction or design discussion.

Review is based on correctness, safety, maintainability, compatibility, and project direction. Passing CI does not guarantee acceptance.

## Commit history

Use clear imperative commit subjects. Maintainers may squash changes when merging. By contributing, you represent that you have the right to submit the work under the repository license once the legal gate opens.

## Community expectations

Participation requires following [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Good-faith disagreement is welcome; harassment, coercion, and disclosure of private information are not.
