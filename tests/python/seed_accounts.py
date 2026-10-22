# 覆盖面：种子 regress_* 六账号（管理员经邀请码通道创建），供整个回归套件使用。
"""Seed the six regress_* accounts over HTTP (no direct DB access).

Uses the admin account (INIT_ADMIN_* / seed defaults) to create one
invitation per target role, then consumes each invitation with the shared
REGRESSION_PASSWORD. Idempotent: existing invitations/users are skipped.

Usage:
    REGRESSION_PASSWORD=... python tests/python/seed_accounts.py
"""
import json
import os
import pathlib
import sys
import urllib.error
import urllib.request

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from common import BASE, UA, PASSWORD, call, login, multipart

# (username, role_id) —— role 对照见 README 的种子账号表。
TARGETS = [
    ("regress_admin", 0),
    ("regress_manager", 1),
    ("regress_beta", 2),
    ("regress_punctuate", 3),
    ("regress_user", 4),
    ("regress_newuser", 4),
]


def main() -> int:
    if not PASSWORD:
        print("skipped: set REGRESSION_PASSWORD")
        return 0
    admin_user = os.environ.get("INIT_ADMIN_USERNAME", "admin")
    admin_pass = os.environ.get("INIT_ADMIN_PASSWORD", "admin123")
    body, ct = multipart({"grant_type": "password", "username": admin_user, "password": admin_pass})
    req = urllib.request.Request(
        BASE + "/oauth/token", data=body, headers={"Content-Type": ct, "User-Agent": UA}
    )
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            admin = json.loads(r.read().decode())["access_token"]
    except urllib.error.HTTPError as e:
        print(f"admin login failed: HTTP {e.code} {e.read().decode('utf-8', 'replace')[:200]}")
        return 1

    ok = True
    for username, role in TARGETS:
        # 已存在则跳过（幂等）。
        st, pr = call("POST", "/system/invitation/info", token=admin, body={"code": f"seed_{username}"})
        if st == 200 and isinstance(pr, dict) and not pr.get("error"):
            print(f"  {username}: invitation already consumed — skip")
            continue
        st, pr = call("POST", "/system/invitation/update", token=admin, body={
            "code": f"seed_{username}", "username": username,
            "roleId": role, "remark": "regression seed", "accessPolicy": [],
        })
        if not (st == 200 and isinstance(pr, dict) and not pr.get("error")):
            print(f"  {username}: invitation create failed st={st} {str(pr)[:150]}")
            ok = False
            continue
        st, pr = call("POST", "/system/invitation/consume", token=admin, body={
            "code": f"seed_{username}", "password": PASSWORD, "nickname": f"{username} nick",
        })
        if st == 200 and isinstance(pr, dict) and not pr.get("error"):
            print(f"  {username}: seeded (role {role})")
        else:
            print(f"  {username}: consume failed st={st} {str(pr)[:150]}")
            ok = False
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
