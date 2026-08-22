"""Minimal Python worker implementing the Hank worker protocol.

The worker is an optional sidecar speaking JSON-RPC 2.0 with
``Content-Length`` framing (see ``transport.py``) over stdin/stdout. It
installs no dependencies, reads no process env, touches no filesystem and
executes no code from messages — a tool request is always answered with
``not_supported`` in this minimal stage.

States mirror the Rust ``WorkerSession``:
``awaiting_handshake -> handshaking -> ready -> shutdown``.
"""

from __future__ import annotations

import sys
from typing import BinaryIO

from . import transport
from .transport import (
    DUPLICATE_ID,
    INVALID_REQUEST,
    METHOD_NOT_FOUND,
    FrameRejected,
    SeenIds,
)

SCHEMA_VERSION = 1
PROTOCOL_VERSION = 1
MAX_WORKER_ID_CHARS = 128
MAX_CAPABILITIES = 32
MAX_DETAIL_CHARS = 256

EXIT_OK = 0
EXIT_HANDSHAKE_REJECTED = 1
EXIT_FORBIDDEN_ARGUMENT = 2

WORKER_IDENTITY = "hank-worker-minimal"


class HandshakeRejected(Exception):
    """Handshake phase violation; the process must exit fail-closed."""


class ProtocolReject(Exception):
    """Bounded protocol violation answered with a JSON-RPC error."""

    def __init__(self, code: int, detail: str) -> None:
        super().__init__(detail)
        self.code = code
        self.detail = detail[:MAX_DETAIL_CHARS]


def _is_valid_worker_id(value: object) -> bool:
    return (
        isinstance(value, str)
        and 0 < len(value) <= MAX_WORKER_ID_CHARS
        and not any(ord(character) < 0x20 or ord(character) == 0x7F for character in value)
    )


def handle_handshake(params: dict) -> dict:
    """Validates handshake params and returns the accepted result payload."""
    if params.get("schema_version") != SCHEMA_VERSION:
        raise ProtocolReject(transport.PARSE_ERROR, "schema version is not supported")
    if params.get("protocol_version") != PROTOCOL_VERSION:
        raise ProtocolReject(transport.PARSE_ERROR, "protocol version is not supported")
    if not _is_valid_worker_id(params.get("worker_id")):
        raise ProtocolReject(INVALID_REQUEST, "worker identity is invalid")
    capabilities = params.get("capabilities")
    if not isinstance(capabilities, list) or not 0 < len(capabilities) <= MAX_CAPABILITIES:
        raise ProtocolReject(INVALID_REQUEST, "handshake capabilities are invalid")
    return {"kind": "handshake_accepted", "schema_version": SCHEMA_VERSION, "worker_id": params["worker_id"], "protocol_version": PROTOCOL_VERSION}


def handle_request(params: dict) -> dict:
    """Answers a tool request without executing or echoing its payload."""
    request_id = params.get("request_id")
    context = params.get("context")
    if not isinstance(request_id, str) or not request_id.startswith("req-"):
        raise ProtocolReject(INVALID_REQUEST, "request id is not part of the protocol")
    if not isinstance(context, dict):
        raise ProtocolReject(INVALID_REQUEST, "request context is not part of the protocol")
    return {
        "kind": "response",
        "schema_version": SCHEMA_VERSION,
        "request_id": request_id,
        "context": context,
        "result": "not_supported",
        "value": None,
        "error": {
            "code": "invalid_message",
            "detail": "worker has no registered capabilities for execution",
        },
    }


def handle_ready(method: str, params: dict) -> tuple[dict | None, bool]:
    """Dispatches a post-handshake method; returns (result, keep_running)."""
    if method == "handshake":
        raise ProtocolReject(INVALID_REQUEST, "handshake already completed")
    if method == "request":
        return handle_request(params), True
    if method == "cancel":
        return None, True
    if method == "health":
        return {"kind": "health_report", "schema_version": SCHEMA_VERSION, "worker_id": WORKER_IDENTITY, "status": "healthy"}, True
    if method == "error":
        return None, True
    if method == "shutdown":
        return {"kind": "shutdown_ack", "schema_version": SCHEMA_VERSION}, False
    raise ProtocolReject(METHOD_NOT_FOUND, "message kind is not part of the protocol")


def run_transport_loop(stream_in: BinaryIO, stream_out: BinaryIO) -> int:
    seen = SeenIds()
    handshaked = False
    while True:
        try:
            message = transport.read_frame(stream_in)
        except FrameRejected:
            # Bounded framing violation: the channel stays usable and no
            # payload content is echoed.
            print("transport frame rejected", file=sys.stderr)
            continue
        if message is None:
            return EXIT_OK

        if transport.is_request(message):
            request_id = message["id"]
            code = transport.structural_error(message)
            if code is not None:
                transport.write_frame(
                    stream_out,
                    transport.error_message(request_id, code, "message rejected by transport"),
                )
                continue
            if not seen.register(request_id):
                transport.write_frame(
                    stream_out,
                    transport.error_message(request_id, DUPLICATE_ID, "request id was already used"),
                )
                continue
            method = message["method"]
            params = message.get("params", {})
            if not isinstance(params, dict):
                transport.write_frame(
                    stream_out,
                    transport.error_message(request_id, INVALID_REQUEST, "params must be an object"),
                )
                continue
            try:
                if not handshaked:
                    if method != "handshake":
                        raise HandshakeRejected("message arrived before handshake")
                    try:
                        reply = handle_handshake(params)
                    except ProtocolReject as rejection:
                        transport.write_frame(
                            stream_out,
                            transport.error_message(request_id, rejection.code, rejection.detail),
                        )
                        raise HandshakeRejected("handshake rejected by protocol") from None
                    handshaked = True
                else:
                    reply, keep_running = handle_ready(method, params)
                    if reply is not None:
                        transport.write_frame(stream_out, transport.result_message(request_id, reply))
                    if not keep_running:
                        return EXIT_OK
                    continue
            except ProtocolReject as rejection:
                transport.write_frame(
                    stream_out,
                    transport.error_message(request_id, rejection.code, rejection.detail),
                )
                continue
            transport.write_frame(stream_out, transport.result_message(request_id, reply))
            continue

        if transport.is_notification(message):
            method = message.get("method")
            code = transport.structural_error(message)
            if code is not None or method not in transport.KNOWN_METHODS:
                # Notifications have no reply target; bounded reject only.
                continue
            if not handshaked:
                raise HandshakeRejected("message arrived before handshake")
            _, keep_running = handle_ready(method, message.get("params", {}))
            if not keep_running:
                return EXIT_OK
            continue

        # Responses and anything else are ignored bounded control traffic.
        continue


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    # Argument allowlist: the minimal worker accepts none.
    if arguments:
        print("worker accepts no arguments", file=sys.stderr)
        return EXIT_FORBIDDEN_ARGUMENT
    try:
        return run_transport_loop(sys.stdin.buffer, sys.stdout.buffer)
    except HandshakeRejected as rejection:
        print(rejection, file=sys.stderr)
        return EXIT_HANDSHAKE_REJECTED


if __name__ == "__main__":
    raise SystemExit(main())
