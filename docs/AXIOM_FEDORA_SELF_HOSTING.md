# Axiom on Fedora — self-hosted deployment plan

**Target:** An always-on Fedora Workstation PC, initially for isolated
staging, followed by a controlled public deployment only after the release
gates in [AXIOM_PRODUCTION_RUNBOOK.md](AXIOM_PRODUCTION_RUNBOOK.md) pass.

**Not a turnkey production compose file.** Stoat's upstream services include
a Rust API, WebSocket events, MongoDB, KeyDB/Redis, RabbitMQ, object storage
and media/voice dependencies. The checked-in `compose.yml` and
`Revolt.toml` are for development and contain fixed credentials and exposed
ports. Never launch those unchanged as a public deployment.

## 1. Assess the host before installing services

From a terminal on **the actual Fedora PC**:

```bash
cat /etc/fedora-release
python3 scripts/axiom_fedora_host_check.py
free -h
df -h
```

The inventory is read-only. It checks available RAM, root disk space, relevant
tools, and SELinux status but cannot confirm database IOPS, outbound network
policy, upload capacity, thermal behavior, real uptime or security.
A system with 16 GiB RAM and spinning-disk storage may handle a limited
private pilot but must be load-tested; MongoDB and upload storage are
I/O-intensive. Do not promise production capacity based on RAM alone.

Review power/sleep behavior so services do not stop when the desktop idles.
Provide an uninterruptible power supply if availability matters, and plan for
ISP outages and host OS security updates. Keep Fedora, container images and
the application patched.

## 2. Network isolation and public endpoints

- Keep databases, RabbitMQ, KeyDB/Redis, object-store management and all
  telemetry/admin UIs **private**. Publish only reviewed reverse-proxy entry
  points; never forward MongoDB, Redis, AMQP, S3 admin or debugging ports.
- Use a dedicated service account or rootless containers with persistent
  storage and least-privilege mount permissions. Respect SELinux contexts
  instead of disabling SELinux to work around volume denials.
- Run a reverse proxy for the separate app, API, WebSocket, upload and
  proxy endpoints required by Stoat. Public app/API/media must use HTTPS and
  events must use WSS. Set a reviewed allowlist of origin hosts and an
  operator-managed domain.
- An outbound tunnel is preferable to exposing residential router ports for
  **HTTP(S) application endpoints**, but tunnel selection must be verified
  against the required WebSocket and upload behavior. Tailscale Funnel may be
  suitable for a small HTTP(S) experiment; it is **not evidence** that
  LiveKit real-time voice/video works, because UDP/TURN transport requires a
  separate reviewed strategy. Do not publish voice/video until tested.
- Avoid publishing a home address or unneeded home-network details in
  repositories, screenshots, logs or support tickets.

## 3. Separation of staging from production

Create dedicated, isolated **staging** databases, buckets, credentials,
test accounts and test messages. Do not copy real user information.
Validate account creation/verification/recovery, real-time delivery between
two accounts, attachment permissions, channel roles, blocking, moderation,
uploads/quotas and reconnection. The opt-in frontend Playwright test can
run only after staging is operational and its account credentials are
supplied privately as environment variables.

Build frontend and Rust backend artifacts from reviewed compatible commits.
Pin container image digests and verify architecture and trust.
Do not assume an upstream image has the Axiom branding or supports the
specific frontend revision.

## 4. Service operation on Fedora

For the eventual hardened services:

- Choose either Podman with systemd/Quadlet or a supported Docker Compose
  setup; do **not** mix their stateful volume ownership accidentally.
- Use named/persistent volumes, service restart policies, health checks,
  an ordered startup process, resource limits and a backup-friendly database
  layout. Run stateful processes without exposing raw ports publicly.
- Keep local SMTP testing tools separate; configure a real verified
  transactional email service before inviting users.
- Treat reverse-proxy access logs, application logs and monitoring endpoints
  as potentially sensitive; define retention and restricted access.
- Test both clean startup after power loss and graceful restart after OS
  updates. Restore the database *and* object store into a fresh isolated
  environment to prove recovery.

## 5. Release gates

```bash
# Validate the operator-generated, EFFECTIVE production config only:
python3 scripts/axiom_production_preflight.py /secure/path/effective-production.toml
```

This does **not** create or validate a production deployment automatically.
See the [production runbook](AXIOM_PRODUCTION_RUNBOOK.md) for the full
ship/no-ship matrix, including Docker/Rust CI, password resets, access
control, backups, performance, legal privacy/terms, moderation/reporting,
monitoring/alerts and staged rollback verification.

**Until all gates pass, do not publicly advertise Axiom as reliable or
production-ready.** A private LAN/staging deployment can be used to exercise
the app without accepting public user data.

## Unresolved operator inputs

- Public hostname/domain or a specific verified tunnel arrangement
- Available disk space and real disk throughput
- Expected users, simultaneous connections, upload quotas, and whether
  voice/video is required at launch
- Secure credentials, outgoing email provider and private backup destination
- Host power/uptime, remote recovery plan and monitoring contact

The GitHub connector cannot operate your Fedora desktop or provision your
router, tunnel, DNS, SMTP provider or backups. These must be configured and
verified with the host operator before the final launch sign-off.

## Voice and screen sharing are mandatory at launch

Axiom must support **messaging, voice calls, and screen sharing** from the first public release. The detailed staging and transport requirements are in [AXIOM_VOICE_SCREENSHARE_LAUNCH.md](AXIOM_VOICE_SCREENSHARE_LAUNCH.md).

The public HTTP/WSS tunnel choice does not answer whether WebRTC voice and screen sharing will work. Prior to release, verify LiveKit media connectivity and network fallback using two separate external networks. Do not turn off these requirements to ship earlier without explicit product approval. Existing frontend CI does not exercise these live media paths.
