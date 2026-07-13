"""Shared configuration for e2e orchestration scripts.

Windows-native: all tools (python, cargo, pnpm, node) run on Windows.
Redis can be provided via WSL Docker (see README).
"""

import os
from pathlib import Path

# ── Paths ────────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
RUST_BIN = "_router"


def _load_dotenv() -> None:
    """Load .env from REPO_ROOT into os.environ (if not already set)."""
    env_file = REPO_ROOT / ".env"
    if not env_file.exists():
        return
    for line in env_file.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        k, v = line.split("=", 1)
        k, v = k.strip(), v.strip()
        if k and k not in os.environ:
            os.environ[k] = v


_load_dotenv()

# ── Vue frontend path (required in .env) ─────────────────────────────────────


def _resolve_vue_frontend() -> Path:
    env_path = os.environ.get("E2E_VUE_FRONTEND")
    if not env_path:
        raise RuntimeError(
            "E2E_VUE_FRONTEND is not set.\n"
            "Add it to .env, e.g.:\n"
            "  E2E_VUE_FRONTEND=D:\\code\\vue_map_register_v3"
        )
    p = Path(env_path).resolve()
    if not (p / "package.json").exists():
        raise RuntimeError(
            f"E2E_VUE_FRONTEND={env_path}\n"
            f"  Does not contain package.json — wrong path?"
        )
    return p


VUE_FRONTEND = _resolve_vue_frontend()

# ── Ports ────────────────────────────────────────────────────────────────────

RUST_PORT = int(os.environ.get("E2E_RUST_PORT", os.environ.get("PORT", "8101")))
VUE_PORT = int(os.environ.get("E2E_VUE_PORT", "9000"))
SHIRABE_PORT = int(os.environ.get("E2E_SHIRABE_PORT", "3100"))

# ── URLs ─────────────────────────────────────────────────────────────────────

RUST_URL = f"http://127.0.0.1:{RUST_PORT}"
VUE_URL = f"http://127.0.0.1:{VUE_PORT}"
SHIRABE_URL = f"http://127.0.0.1:{SHIRABE_PORT}"

# ── Process state directory ──────────────────────────────────────────────────

STATE_DIR = REPO_ROOT / "target" / "e2e"
STATE_DIR.mkdir(parents=True, exist_ok=True)
PID_FILE = STATE_DIR / "processes.json"
