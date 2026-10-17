# Axiom Fedora voice and screen-share launch requirements

**Launch scope:** persistent messaging and attachments, LiveKit-based voice, and screen sharing. This document is a staging and operator-readiness specification, not a public deployment recipe or claim of validated production service.

## Media architecture

The existing Stoat backend supplies signalling/token issuance and authorization while LiveKit carries real-time media using WebRTC. Do not implement a second custom voice stack or port the Nexus prototype backend. Screen sharing uses browser display capture and a LiveKit video track; browser permission prompts and secure contexts are required.

- Serve the web interface and API over HTTPS and the existing event gateway over WSS.
- LiveKit signalling needs a verified public WSS address matching the backend's configured `hosts.livekit.worldwide`. Keep `api.livekit.nodes.worldwide.url` private on the server-side service network.
- Verify the LiveKit node key and secret match those configured in the API, with fresh operator-generated secrets, never the example values in `Revolt.toml` and `livekit.example.yml`.
- **WebRTC media transport is not ordinary HTTPS.** The chosen external connectivity arrangement must support the media path negotiated by browsers. Assess LiveKit UDP reachability and a correctly configured TURN/TLS fallback where necessary. An HTTP-only outbound tunnel, including a working Tailscale Funnel web endpoint, does not by itself validate audio/video.
- Screen sharing must be consent-based, initiated from the browser's native picker, and stoppable by the sharer. Test stopping, disconnecting and changing channels.
- Keep datastore ports, object-storage administration, broker ports, LiveKit administrative interfaces, and debugging endpoints off the public network.
- Test with devices on **different internet connections**, not only two browsers on the same LAN. Browser and network compatibility differs across UDP-filtering networks and mobile devices.

## Staging acceptance scenarios

| Scenario | Pass condition |
| --- | --- |
| Join voice | Two test accounts on different networks can join an authorized voice channel |
| Audio | Both hear each other and mute/deafen status updates correctly |
| Permissions | Account denied channel access cannot join the session through a direct link or token request |
| Session security | Disconnected, expired and revoked sessions cannot mint unauthorized LiveKit access tokens |
| Screen share | Account A explicitly selects a screen/window and B can view the stream |
| Stop sharing | Browser stop button or app stop control ends the published track for both peers |
| Reconnect | Browser refresh and brief network loss do not leave ghost tracks or permanently stale presence |
| Network fallback | Separate tests for unrestricted UDP and a restricted-network TURN/TLS path |
| Device failure | No microphone, denied capture permission, unplugged input device handled without crash |
| Concurrency | Controlled load test measures CPU/RAM, bitrate, host disk and network saturation |
| Privacy | No cross-channel, cross-server or blocked unauthorized live room access |
| Quality | Monitor connection setup errors, packet loss, audio dropouts, media-server restarts and resource use |

## Fedora host limitations

A desktop-class Fedora PC may be sufficient for a **small staging deployment**, but concurrency limits depend heavily on CPU, available uplink bitrate, disk I/O, memory and media routing. No subscriber or stream count is established by the existing green frontend CI.

For planning, collect only nonsecret host facts with `python3 scripts/axiom_fedora_host_check.py`. A real public deployment also needs reliable host uptime, updates, backup/restore, a reviewed ingress method and a stable media transport arrangement. A residential uplink and consumer router may be limiting or introduce extra exposure. Prefer external professional help with firewall or router changes; keep all unnecessary services private.

## Configuration and release blockers

- Determine the public hostname(s) and the exact connectivity method for HTTPS/WSS and real-time media separately.
- Provision a dedicated **isolated staging environment** and two test accounts, with no private or live user data.
- Configure secure LiveKit media routing, fresh credentials and a compatible frontend/backend revision; run the staging matrix above.
- Validate messaging, attachments and voice **together** while other devices are connected.
- Confirm abuse reporting, legal/privacy documentation, monitoring, backups, recovery, and a no-downtime rollback process before invitations.

**No production network has been configured by this document.** Avoid launching with example credentials or publishing the host's personal/home network information in GitHub issues, chats or logs.
