#!/usr/bin/env python3
"""E2E service orchestrator — manages Rust backend + Vue frontend.

Usage:
    python scripts/e2e/serve.py start    # launch all services
    python scripts/e2e/serve.py stop     # stop all services
    python scripts/e2e/serve.py status   # print JSON status
    python scripts/e2e/serve.py restart  # stop + start
"""

from __future__ import annotations

import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

# Add parent for imports
sys.path.insert(0, str(Path(__file__).resolve().parent))
from config import (  # noqa: E402
    PID_FILE,
    REPO_ROOT,
    RUST_PORT,
    RUST_URL,
    STATE_DIR,
    VUE_FRONTEND,
    VUE_PORT,
    VUE_URL,
)
from log import info, warn, error  # noqa: E402

TARGET = "e2e::serve"

# ── Helpers ──────────────────────────────────────────────────────────────────


def _load_env() -> dict[str, str]:
    """Load .env from REPO_ROOT into a dict."""
    env_file = REPO_ROOT / ".env"
    env: dict[str, str] = {}
    if env_file.exists():
        for line in env_file.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" in line:
                k, v = line.split("=", 1)
                env[k.strip()] = v.strip()
    return env


def _wait_for_http(url: str, timeout: float = 60) -> bool:
    """Poll a URL until it returns any HTTP response (even 404 = server alive)."""
    import urllib.error
    import urllib.request

    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            urllib.request.urlopen(url, timeout=3)
            return True
        except urllib.error.HTTPError:
            return True
        except Exception:
            time.sleep(1)
    return False


def _save_state(processes: dict) -> None:
    PID_FILE.write_text(json.dumps(processes, indent=2), encoding="utf-8")


def _load_state() -> dict:
    if PID_FILE.exists():
        return json.loads(PID_FILE.read_text(encoding="utf-8"))
    return {}


def _kill_proc(pid: int) -> bool:
    """Kill a process by PID. Returns True if it was alive."""
    try:
        if os.name == "nt":
            subprocess.run(
                ["taskkill", "/PID", str(pid), "/T", "/F"],
                capture_output=True,
                timeout=10,
            )
        else:
            os.kill(pid, signal.SIGTERM)
            time.sleep(2)
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        return True
    except Exception:
        return False


def _have_malkuth() -> str | None:
    """Return the malkuth binary path if available, else None."""
    return shutil.which("malkuth") or shutil.which("npx")


# ── Service definitions ──────────────────────────────────────────────────────


def start_rust() -> dict | None:
    """Start the Rust backend, optionally via malkuth for auto-restart."""
    env = {**os.environ, **_load_env()}
    env["PORT"] = str(RUST_PORT)

    # Check if malkuth CLI is available for file-watch auto-restart
    malkuth = shutil.which("malkuth")
    if malkuth:
        info(TARGET, f"Starting Rust backend via malkuth (file-watch + auto-restart)...")
        cmd = [
            malkuth,
            "--watch", str(REPO_ROOT / "packages"),
            "--",
            "cargo", "run", "--bin", "_router",
        ]
    else:
        # Try npx malkuth
        npx = shutil.which("npx")
        if npx:
            info(TARGET, f"Starting Rust backend via npx malkuth (file-watch + auto-restart)...")
            cmd = [
                npx, "@celestia-island/malkuth",
                "--watch", str(REPO_ROOT / "packages"),
                "--",
                "cargo", "run", "--bin", "_router",
            ]
        else:
            info(TARGET, f"Starting Rust backend (no malkuth, plain cargo run)...")
            cmd = ["cargo", "run", "--bin", "_router"]

    proc = subprocess.Popen(
        cmd,
        cwd=str(REPO_ROOT),
        env=env,
        stdout=open(STATE_DIR / "rust.log", "w", encoding="utf-8"),
        stderr=subprocess.STDOUT,
    )

    if _wait_for_http(f"{RUST_URL}/", timeout=120):
        info(TARGET, f"Rust backend ready at {RUST_URL}")
        return {"pid": proc.pid, "url": RUST_URL, "name": "rust"}
    else:
        error(TARGET, f"Rust backend did not become ready in 120s")
        _kill_proc(proc.pid)
        return None


def start_vue() -> dict | None:
    """Start the Vue dev server."""
    info(TARGET, f"Starting Vue dev server on port {VUE_PORT}...")
    proc = subprocess.Popen(
        ["pnpm", "dev"],
        cwd=str(VUE_FRONTEND),
        shell=True,
        stdout=open(STATE_DIR / "vue.log", "w", encoding="utf-8"),
        stderr=subprocess.STDOUT,
    )

    if _wait_for_http(VUE_URL, timeout=60):
        info(TARGET, f"Vue frontend ready at {VUE_URL}")
        return {"pid": proc.pid, "url": VUE_URL, "name": "vue"}
    else:
        error(TARGET, f"Vue frontend did not become ready in 60s")
        _kill_proc(proc.pid)
        return None


# ── Commands ─────────────────────────────────────────────────────────────────


def cmd_start() -> int:
    # Ensure frontend is configured
    setup_script = Path(__file__).parent / "setup_frontend.py"
    subprocess.run([sys.executable, str(setup_script)], check=True)

    processes = {}

    rust_info = start_rust()
    if rust_info is None:
        return 1
    processes["rust"] = rust_info

    vue_info = start_vue()
    if vue_info is None:
        _kill_proc(rust_info["pid"])
        return 1
    processes["vue"] = vue_info

    _save_state(processes)
    info(TARGET, f"All services running — Rust: {RUST_URL}  Vue: {VUE_URL}")
    return 0


def cmd_stop() -> int:
    state = _load_state()
    if not state:
        info(TARGET, "No services recorded.")
        return 0

    for name, svc_info in state.items():
        pid = svc_info.get("pid")
        if pid:
            killed = _kill_proc(pid)
            status = "killed" if killed else "already dead"
            info(TARGET, f"{name} (pid {pid}): {status}")

    PID_FILE.unlink(missing_ok=True)
    info(TARGET, "All services stopped.")
    return 0


def cmd_status() -> int:
    state = _load_state()
    if not state:
        print(json.dumps({"running": False}))
        return 0

    alive = {}
    for name, info in state.items():
        pid = info.get("pid", 0)
        try:
            if os.name == "nt":
                subprocess.run(
                    ["tasklist", "/FI", f"PID eq {pid}"],
                    capture_output=True,
                    timeout=5,
                )
                alive[name] = info
            else:
                os.kill(pid, 0)
                alive[name] = info
        except Exception:
            pass

    output = {
        "running": len(alive) > 0,
        "services": alive,
        "rust_url": RUST_URL,
        "vue_url": VUE_URL,
    }
    print(json.dumps(output, indent=2))
    return 0 if alive else 1


def main() -> int:
    if len(sys.argv) < 2:
        print("Usage: python serve.py {start|stop|status|restart}")
        return 1

    cmd = sys.argv[1]
    if cmd == "start":
        return cmd_start()
    elif cmd == "stop":
        return cmd_stop()
    elif cmd == "status":
        return cmd_status()
    elif cmd == "restart":
        cmd_stop()
        time.sleep(2)
        return cmd_start()
    else:
        error(TARGET, f"Unknown command: {cmd}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
