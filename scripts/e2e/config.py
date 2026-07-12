"""Shared configuration for e2e orchestration scripts."""

import os
from pathlib import Path

# ── Paths ────────────────────────────────────────────────────────────────────

# This script lives at <repo>/scripts/e2e/config.py
REPO_ROOT = Path(__file__).resolve().parent.parent.parent

# The Vue3 frontend is a sibling directory
VUE_FRONTEND = REPO_ROOT.parent / "vue_map_register_v3"

# Rust backend binary name
RUST_BIN = "_router"

# ── Ports ────────────────────────────────────────────────────────────────────

RUST_PORT = int(os.environ.get("E2E_RUST_PORT", "8101"))
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
