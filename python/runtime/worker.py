"""Minimal Python worker implementing the Hank worker protocol.

The worker is an optional sidecar: it speaks the versioned contract defined
in ``crates/agent-protocol/src/worker.rs`` over newline-delimited JSON on
stdin/stdout. It installs no dependencies, reads no process env,
touches no filesystem paths and executes no code from messages — a request
is always answered with ``not_supported`` in this minimal stage.

States mirror the Rust ``WorkerSession``:
``awaiting_handshake -> handshaking -> ready -> shutdown``.
"""

from __future__ import annotations

import json
import sys

SCHEMA_VERSION = 1
PROTOCOL_VERSION = 1
MAX_LINE_BYTES = 131_072
MAX_WORKER_ID_CHARS = 128
MAX_CAPABILITIES = 32
MAX_DETAIL_CHARS = 256

EXIT_OK = 0
EXIT_HANDSHAKE_REJECTED = 1
EXIT_FORBIDDEN_ARGUMENT = 2


def message(kind: str, **fields: object) -> dict[str, object]:
    return {"kind": kind, "schema_version": SCHEMA_VERSION, **fields}


def error_reply(code: str, detail: str) -> dict[str, object]:
    trimmed = detail[:MAX_DETAIL_CHARS]
    return message("error", code=code, detail=trimmed)


def dump(value: dict) -> str:
    return json.dumps(value, separators=(",", ":"))


def is_valid_worker_id(value: object) -> bool:
    return (
        isinstance(value, str)
        and 0 < len(value) <= MAX_WORKER_ID_CHARS
        and not any(ord(character) < 0x20 or ord(character) == 0x7F for character in value)
    )


def validate_handshake(payload: dict[str, object]) -> str | None:
    if payload.get("schema_version") != SCHEMA_VERSION:
        return "unsupported_version"
    if payload.get("protocol_version") != PROTOCOL_VERSION:
        return "unsupported_version"
    if not is_valid_worker_id(payload.get("worker_id")):
        return "invalid_message"
    capabilities = payload.get("capabilities")
    if not isinstance(capabilities, list) or not 0 < len(capabilities) <= MAX_CAPABILITIES:
        return "invalid_message"
    return None


def handle_handshake(payload: dict[str, object]) -> tuple[dict[str, object], bool]:
    rejection = validate_handshake(payload)
    if rejection is not None:
        return error_reply(rejection, "handshake rejected by worker protocol"), False
    accepted = message(
        "handshake_accepted",
        worker_id=payload["worker_id"],
        protocol_version=PROTOCOL_VERSION,
    )
    return accepted, True


def handle_ready(payload: dict[str, object]) -> tuple[dict[str, object], bool]:
    """Dispatch a message accepted after the handshake phase.

    Returns the reply (or ``None`` for silent control messages) and whether
    the worker keeps running.
    """
    kind = payload.get("kind")
    if kind == "request":
        request_id = payload.get("request_id")
        context = payload.get("context")
        if not isinstance(request_id, str) or not request_id.startswith("req-"):
            return error_reply("invalid_message", "request id is not part of the protocol"), True
        if not isinstance(context, dict):
            return error_reply("invalid_message", "request context is not part of the protocol"), True
        # The minimal worker executes nothing: every request is answered
        # fail-closed without echoing the payload.
        response = message(
            "response",
            request_id=request_id,
            context=context,
            result="not_supported",
            value=None,
            error={
                "code": "invalid_message",
                "detail": "worker has no registered capabilities for execution",
            },
        )
        return response, True
    if kind == "cancel":
        return None, True
    if kind == "health":
        report = message(
            "health_report",
            worker_id=WORKER_IDENTITY,
            status="healthy",
        )
        return report, True
    if kind == "error":
        return None, True
    if kind == "shutdown":
        return message("shutdown_ack"), False
    if kind == "handshake":
        return error_reply("invalid_state", "handshake already completed"), True
    return error_reply("invalid_message", "message kind is not part of the protocol"), True


WORKER_IDENTITY = "hank-worker-minimal"


def run_loop(stream_in, stream_out) -> int:
    handshaked = False
    for raw_line in stream_in:
        if len(raw_line) > MAX_LINE_BYTES:
            stream_out.write(dump(error_reply("invalid_message", "line exceeds the bounded size")) + "\n")
            stream_out.flush()
            continue
        try:
            payload = json.loads(raw_line)
        except ValueError:
            payload = None
        if not isinstance(payload, dict):
            stream_out.write(dump(error_reply("invalid_message", "line is not a JSON object")) + "\n")
            stream_out.flush()
            continue
        if payload.get("schema_version") != SCHEMA_VERSION:
            stream_out.write(dump(error_reply("unsupported_version", "schema version is not supported")) + "\n")
            stream_out.flush()
            if not handshaked:
                return EXIT_HANDSHAKE_REJECTED
            continue
        if not handshaked:
            if payload.get("kind") != "handshake":
                stream_out.write(dump(error_reply("invalid_state", "message arrived before handshake")) + "\n")
                stream_out.flush()
                return EXIT_HANDSHAKE_REJECTED
            reply, accepted = handle_handshake(payload)
            stream_out.write(dump(reply) + "\n")
            stream_out.flush()
            if not accepted:
                return EXIT_HANDSHAKE_REJECTED
            handshaked = True
            continue
        reply, keep_running = handle_ready(payload)
        if reply is not None:
            stream_out.write(dump(reply) + "\n")
            stream_out.flush()
        if not keep_running:
            return EXIT_OK
    # stdin closed without shutdown: the channel ended cleanly.
    return EXIT_OK


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    # Argument allowlist: the minimal worker accepts none.
    if arguments:
        print("worker accepts no arguments", file=sys.stderr)
        return EXIT_FORBIDDEN_ARGUMENT
    return run_loop(sys.stdin, sys.stdout)


if __name__ == "__main__":
    raise SystemExit(main())
