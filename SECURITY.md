# Security Policy

Earmark Hardened Kernel is pre-1.0 software. Security reports should go through a private reporting route first rather than through public issues.

## Supported Versions

Security fixes target the default branch unless a later release process states otherwise. There is no separate supported maintenance line at this stage.

## Reporting a Vulnerability

Do not open a public issue for a suspected vulnerability.

Report privately to `earmark@posteo.de`. Include:

- the affected commit, tag, or branch if known
- reproduction steps
- expected impact
- relevant command output, logs, or minimal proof material

Useful reports include problems involving:

- repository storage and file handling
- path handling or archive import/export
- declaration parsing and validation
- index rebuild behavior
- corruption or loss of durable state
- dependency vulnerabilities with practical impact on this repository

## Disclosure

Public disclosure should wait until the issue has been reviewed and a fix or mitigation is available.

This policy does not promise a fixed response time. Reports are reviewed as maintainer capacity allows.

Copyright (c) 2026 Mikhail Shakhnazarov
