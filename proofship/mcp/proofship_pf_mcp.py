#!/usr/bin/env python3
# ProofShip local ProofForge MCP (stdio) — Studio ship lane.
#
# Wraps the vendored / resolved `proof-forge-next` CLI so desktop ACP agents
# can call gate tools without a full proof_forge monorepo checkout.
# Full-catalog MCP (chain catalog, OnchainOS, …) still lives in proof_forge
# (`tools/mcp/proof_forge_mcp_server.py`); set PROOF_FORGE_ROOT to prefer it
# via the Studio resolver, or point agents at the remote HTTP MCP on web.
#
# Transport: newline-delimited JSON-RPC 2.0 on stdin/stdout (MCP stdio).
# Logs: stderr only. No network broadcast / private keys.

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import traceback
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

PROTOCOL_VERSION = "2024-11-05"
SERVER_NAME = "proofship-pf-mcp"
SERVER_VERSION = "0.1.0"
SCHEMA_WRAP = "proofship.mcp.tool-result.v1"


def _script_dir() -> Path:
    return Path(__file__).resolve().parent


def find_cli() -> Path:
    env = os.environ.get("PROOF_FORGE_CLI", "").strip() or os.environ.get(
        "PF_CLI", ""
    ).strip()
    if env:
        p = Path(env).expanduser().resolve()
        if p.is_file():
            return p
        raise RuntimeError(f"PROOF_FORGE_CLI/PF_CLI is not a file: {p}")

    which = shutil.which("proof-forge-next")
    if which:
        return Path(which).resolve()

    # proofship/mcp → proofship/ → repo root
    repo = _script_dir().parent.parent
    vendored = repo / "proofship" / "toolchain" / "bin" / "proof-forge-next"
    if vendored.is_file() and os.access(vendored, os.X_OK):
        return vendored.resolve()

    raise RuntimeError(
        "proof-forge-next not found; set PROOF_FORGE_CLI or install "
        "proofship/scripts/install-toolchain.sh"
    )


def find_cwd() -> Path:
    env = os.environ.get("PROOFSHIP_PROJECT_ROOT", "").strip()
    if env:
        return Path(env).expanduser().resolve()
    # Prefer inbox next to toolchain when present.
    repo = _script_dir().parent.parent
    inbox = repo / "proofship" / "inbox"
    if inbox.is_dir():
        return inbox.resolve()
    return Path.cwd().resolve()


def run_cli(cli: Path, args: Sequence[str], *, cwd: Path) -> Dict[str, Any]:
    env = os.environ.copy()
    cmd = [str(cli), *args]
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(cwd),
            env=env,
            capture_output=True,
            text=True,
            timeout=240,
            check=False,
        )
    except subprocess.TimeoutExpired as e:
        return {
            "schema": SCHEMA_WRAP,
            "ok": False,
            "exitCode": None,
            "command": cmd,
            "stdout": (e.stdout or "") if isinstance(e.stdout, str) else "",
            "stderr": "timeout",
            "error": "timeout",
        }
    return {
        "schema": SCHEMA_WRAP,
        "ok": proc.returncode == 0,
        "exitCode": proc.returncode,
        "command": cmd,
        "stdout": proc.stdout or "",
        "stderr": proc.stderr or "",
        "error": None if proc.returncode == 0 else "cli_failed",
    }


def _tool_result(payload: Dict[str, Any], *, is_error: bool = False) -> Dict[str, Any]:
    return {
        "content": [{"type": "text", "text": json.dumps(payload, ensure_ascii=False, indent=2)}],
        "isError": is_error or not payload.get("ok", False),
    }


def tool_pf_version(cli: Path, _args: Dict[str, Any], cwd: Path) -> Dict[str, Any]:
    return _tool_result(run_cli(cli, ["version", "--json"], cwd=cwd))


def tool_pf_check(cli: Path, args: Dict[str, Any], cwd: Path) -> Dict[str, Any]:
    source = args.get("source")
    module = args.get("module")
    if not source or not module:
        return _tool_result(
            {
                "schema": SCHEMA_WRAP,
                "ok": False,
                "error": "usage",
                "stderr": "pf_check requires source and module",
            },
            is_error=True,
        )
    root = args.get("root") or str(cwd)
    cmd = ["check", str(source), "--module", str(module), "--root", str(root)]
    if args.get("json"):
        cmd.append("--json")
    return _tool_result(run_cli(cli, cmd, cwd=Path(root)))


def tool_pf_build(cli: Path, args: Dict[str, Any], cwd: Path) -> Dict[str, Any]:
    source = args.get("source")
    module = args.get("module")
    target = args.get("target") or "evm"
    if not source or not module:
        return _tool_result(
            {
                "schema": SCHEMA_WRAP,
                "ok": False,
                "error": "usage",
                "stderr": "pf_build requires source and module",
            },
            is_error=True,
        )
    if args.get("broadcast") or args.get("network"):
        return _tool_result(
            {
                "schema": SCHEMA_WRAP,
                "ok": False,
                "error": "usage",
                "stderr": "pf_build rejects network/broadcast; deploy stays in ProofShip Studio",
            },
            is_error=True,
        )
    root = args.get("root") or str(cwd)
    out = args.get("outputDir") or args.get("output") or f"out-{str(module).lower()}"
    cmd = [
        "build",
        str(source),
        "--module",
        str(module),
        "--target",
        str(target),
        "-o",
        str(out),
        "--root",
        str(root),
    ]
    if args.get("json"):
        cmd.append("--json")
    return _tool_result(run_cli(cli, cmd, cwd=Path(root)))


def tool_pf_artifacts(cli: Path, args: Dict[str, Any], cwd: Path) -> Dict[str, Any]:
    output_dir = args.get("outputDir") or args.get("output")
    if not output_dir:
        return _tool_result(
            {
                "schema": SCHEMA_WRAP,
                "ok": False,
                "error": "usage",
                "stderr": "pf_artifacts requires outputDir",
            },
            is_error=True,
        )
    root = args.get("root") or str(cwd)
    cmd = ["inspect", "--output-dir", str(output_dir), "--root", str(root)]
    if args.get("json", True):
        cmd.append("--json")
    return _tool_result(run_cli(cli, cmd, cwd=Path(root)))


TOOLS = {
    "pf_version": tool_pf_version,
    "pf_check": tool_pf_check,
    "pf_build": tool_pf_build,
    "pf_artifacts": tool_pf_artifacts,
}


def tool_definitions() -> List[Dict[str, Any]]:
    return [
        {
            "name": "pf_version",
            "description": "ProofForge CLI version / identity (engineering dist).",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
        },
        {
            "name": "pf_check",
            "description": "Run proof-forge-next check on a ProgramV1 .lean source.",
            "inputSchema": {
                "type": "object",
                "required": ["source", "module"],
                "properties": {
                    "source": {"type": "string"},
                    "module": {"type": "string"},
                    "root": {"type": "string"},
                    "json": {"type": "boolean"},
                },
                "additionalProperties": False,
            },
        },
        {
            "name": "pf_build",
            "description": "Build ProgramV1 for a target (default evm). No broadcast.",
            "inputSchema": {
                "type": "object",
                "required": ["source", "module"],
                "properties": {
                    "source": {"type": "string"},
                    "module": {"type": "string"},
                    "target": {"type": "string", "default": "evm"},
                    "outputDir": {"type": "string"},
                    "root": {"type": "string"},
                    "json": {"type": "boolean"},
                },
                "additionalProperties": False,
            },
        },
        {
            "name": "pf_artifacts",
            "description": "Inspect a build output directory (digest / artifact list).",
            "inputSchema": {
                "type": "object",
                "required": ["outputDir"],
                "properties": {
                    "outputDir": {"type": "string"},
                    "root": {"type": "string"},
                    "json": {"type": "boolean", "default": True},
                },
                "additionalProperties": False,
            },
        },
    ]


def _read_message() -> Optional[Dict[str, Any]]:
    line = sys.stdin.readline()
    if not line:
        return None
    line = line.strip()
    if not line:
        return _read_message()
    return json.loads(line)


def _write_message(msg: Dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(msg, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def main(argv: Optional[Sequence[str]] = None) -> int:
    argv = list(argv or sys.argv[1:])
    if "--self-check" in argv:
        cli = find_cli()
        cwd = find_cwd()
        print(f"ok cli={cli} cwd={cwd}", file=sys.stderr)
        return 0
    if "--help" in argv or "-h" in argv:
        sys.stderr.write(
            f"{SERVER_NAME} {SERVER_VERSION}\n"
            "Tools: pf_version pf_check pf_build pf_artifacts\n"
        )
        return 0

    try:
        cli = find_cli()
        cwd = find_cwd()
    except Exception as e:
        sys.stderr.write(f"{SERVER_NAME}: {e}\n")
        return 2

    while True:
        try:
            msg = _read_message()
        except Exception:
            traceback.print_exc(file=sys.stderr)
            continue
        if msg is None:
            return 0
        method = msg.get("method")
        msg_id = msg.get("id")
        params = msg.get("params") or {}

        if method == "initialize":
            _write_message(
                {
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "result": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
                    },
                }
            )
        elif method == "notifications/initialized":
            continue
        elif method == "tools/list":
            _write_message(
                {"jsonrpc": "2.0", "id": msg_id, "result": {"tools": tool_definitions()}}
            )
        elif method == "tools/call":
            name = params.get("name")
            arguments = params.get("arguments") or {}
            handler = TOOLS.get(name)
            if not handler:
                _write_message(
                    {
                        "jsonrpc": "2.0",
                        "id": msg_id,
                        "error": {"code": -32601, "message": f"unknown tool: {name}"},
                    }
                )
                continue
            try:
                result = handler(cli, arguments, cwd)
            except Exception as e:
                traceback.print_exc(file=sys.stderr)
                result = _tool_result(
                    {"schema": SCHEMA_WRAP, "ok": False, "error": str(e)},
                    is_error=True,
                )
            _write_message({"jsonrpc": "2.0", "id": msg_id, "result": result})
        elif method == "ping":
            _write_message({"jsonrpc": "2.0", "id": msg_id, "result": {}})
        elif msg_id is not None:
            _write_message(
                {
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "error": {"code": -32601, "message": f"unsupported: {method}"},
                }
            )


if __name__ == "__main__":
    raise SystemExit(main())
