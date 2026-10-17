#!/usr/bin/env python3
"""Read-only Fedora host readiness checks for an Axiom self-hosted deployment.

Run this on the actual Fedora host. Nothing is installed, modified or opened.
This is a host inventory, NOT evidence of production security.
"""
from __future__ import annotations

import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys


def command(name: str) -> bool:
    return shutil.which(name) is not None


def report(label: str, status: str, detail: str) -> None:
    print(f"[{status}] {label}: {detail}")


def memory_gib() -> float | None:
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemTotal:"):
                return int(line.split()[1]) / 1024**2
    except (OSError, ValueError, IndexError):
        pass
    return None


def main() -> int:
    release = ""
    try:
        release = Path("/etc/os-release").read_text()
    except OSError:
        pass
    is_fedora = 'ID=fedora' in release or 'ID="fedora"' in release
    report("Operating system", "OK" if is_fedora else "REVIEW", platform.platform())

    total = memory_gib()
    if total is not None:
        report("Physical RAM", "OK" if total >= 8 else "REVIEW",
               f"{total:.1f} GiB detected; resource sizing requires staging load tests")
    else:
        report("Physical RAM", "REVIEW", "Unable to read /proc/meminfo")

    try:
        root = shutil.disk_usage("/")
        free = root.free / 1024**3
        report("Root filesystem", "REVIEW" if free < 30 else "OK",
               f"{free:.1f} GiB available; size for database, uploads, logs and backups separately")
    except OSError:
        report("Root filesystem", "REVIEW", "Unable to obtain free space")

    for cmd in ("podman", "docker", "systemctl", "firewall-cmd", "python3"):
        report(cmd, "FOUND" if command(cmd) else "MISSING",
               "command available" if command(cmd) else "install/configure if needed")

    if command("getenforce"):
        try:
            result = subprocess.run(["getenforce"], capture_output=True,
                                    text=True, timeout=4, check=False)
            mode = result.stdout.strip() or "unknown"
            report("SELinux", "OK" if mode == "Enforcing" else "REVIEW", mode)
        except (OSError, subprocess.TimeoutExpired):
            report("SELinux", "REVIEW", "Could not read status")
    else:
        report("SELinux", "REVIEW", "getenforce unavailable")

    report("Public network", "MANUAL", "No firewall rules, ports or tunnel settings were changed")
    report("Backups", "MANUAL", "Encrypted off-host backup and restore rehearsal required")
    report("Uptime", "MANUAL", "Power, sleep, updates, ISP limits and external reachability require real tests")
    report("Voice/video", "MANUAL", "LiveKit UDP/TURN support is not established by HTTP-only tunnels")
    report("Launch", "BLOCKED", "Host inventory is not a production approval")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
