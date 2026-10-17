#!/usr/bin/env python3
"""Validate the *effective* Axiom production TOML before a deployment.

The checked-in Revolt.toml is development-only. Run this on the resolved
production override/config (with secrets supplied privately), never on a
template containing unexpanded placeholders.
"""
import argparse
import ipaddress
from pathlib import Path
import sys
import tomllib
from urllib.parse import urlsplit

DEV_SECRETS = {
    "minioautumn", "rabbitpass", "rabbituser", "smtp", "worldwide",
    "ZjCofRlfm6GGtjlifmNpCDkcQbEIIVC0",
}
DEV_HOSTS = {"localhost", "127.0.0.1", "local.revolt.chat", "host.docker.internal"}

def check(config: dict) -> list[str]:
    problems: list[str] = []
    hosts = config.get("hosts", {})
    if not isinstance(hosts, dict):
        return ["[hosts] section is required"]

    # Public-facing endpoints must be TLS-protected and publicly addressable.
    for name in ("app", "api", "events", "autumn", "january"):
        value = hosts.get(name)
        if not isinstance(value, str) or not value.strip():
            problems.append(f"hosts.{name}: HTTPS/WSS endpoint required")
            continue
        try:
            url = urlsplit(value)
            required_scheme = "wss" if name == "events" else "https"
            if url.scheme != required_scheme or not url.hostname or url.username or url.password:
                problems.append(f"hosts.{name}: must use {required_scheme} without URL credentials")
                continue
            hostname = url.hostname.lower()
            if hostname in DEV_HOSTS or hostname.endswith((".local", ".localhost", ".invalid", ".example", ".test")):
                problems.append(f"hosts.{name}: development or placeholder host forbidden")
            try:
                if not ipaddress.ip_address(hostname).is_global:
                    problems.append(f"hosts.{name}: private/loopback address forbidden")
            except ValueError:
                pass
        except ValueError:
            problems.append(f"hosts.{name}: malformed URL")

    # Reject known development credentials. Do not print actual credential values.
    secret_fields = (
        ("api", "smtp", "password"),
        ("api", "livekit", "nodes", "worldwide", "secret"),
        ("files", "s3", "secret_access_key"),
    )
    for path in secret_fields:
        target = config
        for piece in path:
            target = target.get(piece, {}) if isinstance(target, dict) else {}
        label = ".".join(path)
        if not isinstance(target, str) or not target.strip():
            problems.append(f"{label}: configured credential required")
        elif target in DEV_SECRETS or target.lower().startswith(("changeme", "replace_me", "example", "placeholder")):
            problems.append(f"{label}: known development/placeholder credential forbidden")

    smtp = config.get("api", {}).get("smtp", {})
    if not isinstance(smtp, dict) or not smtp.get("host") or str(smtp.get("host")).lower() in DEV_HOSTS:
        problems.append("api.smtp.host: real mail delivery host required")
    if isinstance(smtp, dict) and not (smtp.get("use_tls") or smtp.get("use_starttls")):
        problems.append("api.smtp: TLS or STARTTLS required")
    if isinstance(smtp, dict) and str(smtp.get("from_address", "")).endswith("@revolt.chat"):
        problems.append("api.smtp.from_address: replace upstream development identity")

    s3 = config.get("files", {}).get("s3", {})
    if isinstance(s3, dict) and str(s3.get("access_key_id", "")) in DEV_SECRETS:
        problems.append("files.s3.access_key_id: development credential forbidden")

    return problems

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", type=Path, help="Effective production TOML (not the dev template)")
    args = parser.parse_args()
    try:
        with args.config.open("rb") as stream:
            config = tomllib.load(stream)
        problems = check(config)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        print(f"Preflight failed: cannot parse supplied config ({type(exc).__name__})", file=sys.stderr)
        return 2
    if problems:
        print("Axiom production preflight BLOCKED:", file=sys.stderr)
        for issue in problems:
            print(f" - {issue}", file=sys.stderr)
        return 1
    print("Axiom configuration preflight passed. This is not a security audit or deployment approval.")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
