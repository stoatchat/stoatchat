# Axiom production launch gate (operator runbook)

**Status: NOT CLEARED FOR PUBLIC LAUNCH.** This is an operational checklist,
not proof of a secured production installation. The Stoat-derived Rust
backend is the source of truth; Nexus v12 is a UI reference only.

## Deployment principles

1. Run the backend behind a TLS-terminating reverse proxy with separate HTTPS
   application/API/media URLs and WSS real-time events. Do not expose database,
   KeyDB/Redis, RabbitMQ, object-store management, internal telemetry, or
   MongoDB admin ports to the public Internet.
2. **Do not deploy `compose.yml` or `Revolt.toml` as-is.** They are local
   development files containing fixed/test credentials, loopback endpoints,
   exposed ports, non-TLS URLs, and a mock mail service.
3. Use a separately managed production configuration. Keep effective config,
   SMTP/storage credentials, encryption secrets, signing keys, and integration
   credentials out of Git, build logs, and static web files. Restrict file
   permissions; rotate any default credentials.
4. Supply production SMTP and validate outbound verification/recovery emails.
   Configure storage bucket ACLs and verify that users cannot download private
   attachments from other users or servers by guessing a URL.
5. Pin and scan deployable container images; keep dependencies patched, and
   limit service capabilities/resources. Validate live instance hostnames and
   protect metrics, debugging endpoints, administration, and rate-limit controls.

## Configuration preflight

The validator examines **the effective production TOML**, including overrides
resolved by deployment automation. It does not automatically merge
`Revolt.toml` and `Revolt.overrides.toml`.

```bash
python3 scripts/axiom_production_preflight.py /secure/path/effective-production.toml
```

The script rejects insecure public URLs, local/placeholder domains, known
development secrets, and mock/non-TLS SMTP defaults. Never paste credentials
into chat, issues, CI output, or committed files. Do not assume a passing
preflight means that your containers, reverse proxy, or access policies are
correctly secured. Also verify the live WebSocket/media endpoints.

## Release verification matrix

| Gate | Required evidence before release |
| --- | --- |
| Frontend CI | Latest PR commit passes TypeScript, lint, i18n, Chromium/Firefox Playwright and Docker build |
| Rust CI | Latest backend PR commit passes Rust compile/tests and container checks |
| Auth/session | Dedicated isolated staging accounts: create, verify, log in/out, reset password, revoke session, invalid and expired tokens |
| User privacy | Block/unblock and friend visibility enforced server-side, including direct APIs |
| Servers | Role permission matrix; unauthorized create/edit/delete channel and moderation requests fail |
| Messaging | Two staging accounts verify delivery, reconnect, edits, deletes, mentions, historical pagination and visibility boundaries |
| Attachments | Upload limits, private access, cross-server leakage checks, MIME handling, cleanup and storage failure paths |
| Real-time | Reconnect behavior, websocket TLS, duplicate/out-of-order events, backpressure and disconnect recovery |
| Abuse protection | Account creation, login, upload and message-rate limits; reporting/moderation escalation path |
| Observability | Alerts for API errors, message backlog, disk, database health, object-store errors, latency and TLS expiry |
| Disaster recovery | Encrypted database/object-store backups with retention and a completed restore drill on staging |
| Web security | CSP and other headers validated at reverse proxy; CORS, cookies, CSRF protections, TLS and dependency vulnerability review |
| Branding/legal | Axiom-specific terms, privacy notices, contact/reporting path, upstream license and attribution audit |
| Rollout | Canary/staging deployment, rollback image + schema plan, and owner approval of release checklist |

## Environment and release policy

- Provision an isolated **staging** deployment with its own test-only
  accounts, storage bucket, databases and test messages. Do not use live
  user data for integration tests.
- Establish a real deployment hostname, operator-owned TLS, backup storage,
  service monitoring, and Axiom terms/privacy/support contact.
- Run the front- and backend PRs against compatible API revisions and sign
  off on a tagged, immutable pair of image digests.
- Keep both pull requests in draft until all required evidence has been
  captured; never interpret a green source check as release authorization.
- Avoid self-hosting public services directly from a personal device without
  reliable availability, backups, network isolation, patching, and adult
  supervision for ongoing operations.

## Remaining work

Actual infrastructure provisioning, staging accounts, secrets, backups,
mail credentials, external-service registration, legal approvals, abuse
operations and production traffic tests require the system operator. None
have been performed by the code commits in this branch.
