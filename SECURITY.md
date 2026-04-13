# Security Policy

## Supported Versions

Spectoru is currently in pre-release (v0.x). Only the latest commit on `main`
receives security updates.

## Reporting a Vulnerability

Please report security issues through **GitHub Private Vulnerability Reporting**
rather than opening a public issue:

1. Navigate to the repository's
   [Security tab](https://github.com/hidari/spectoru/security)
2. Click **Report a vulnerability**
3. Provide a clear description, reproduction steps, and an impact assessment

We will acknowledge receipt within 7 days and aim to publish a fix or mitigation
within 30 days for confirmed vulnerabilities.

## Threat Model and Scope

Spectoru is a build-time CLI tool that reads developer-supplied source files
and produces a static HTML site. It does not handle untrusted network input or
process credentials. The following are explicitly **out of scope**:

- Vulnerabilities that require local code-execution access (Spectoru only runs
  on developer or CI machines that already have full source-code access).
- Issues in third-party dependencies — please report those upstream. Spectoru
  uses `cargo-deny` and Dependabot to track and respond to advisories.

For all other issues, the private vulnerability report channel above is the
preferred contact.
