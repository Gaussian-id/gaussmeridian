# Security Policy

## Supported versions

No public release is supported while the Technical Preview publication gate is closed. A version-support table will be added with the first release.

## Report privately

Do not open a public issue for suspected vulnerabilities, exposed credentials, authentication bypasses, provider-key leaks, or unsafe deployment defaults.

After publication, use GitHub private vulnerability reporting:

`https://github.com/Gaussian-id/gauss-meridian/security/advisories/new`

Until that channel is enabled, contact the repository owner through the verified contact listed on the GitHub organization profile. Include:

- affected component and revision;
- reproducible steps or a minimal proof of concept;
- expected and observed behavior;
- likely impact;
- suggested mitigation, if known.

Never include real provider keys, personal data, or third-party systems in a report.

## Response process

Maintainers will acknowledge a usable report, reproduce and classify it, prepare a fix, and coordinate disclosure. Timing depends on severity, affected versions, and dependency ownership. No bounty is promised unless a separate program explicitly states one.

## Scope

The intended scope includes the API, web console, official container definitions, authentication and authorization paths, credential handling, provider integrations, and supported upgrade paths. Social engineering, denial-of-service load generation, and testing systems you do not own are out of scope.
