"""JSON-RPC 2.0 transport with Content-Length framing for the Hank worker.

Mirrors ``crates/agent-protocol/src/json_rpc.rs``: frames are
``Content-Length: N\\r\\n\\r\\n<payload>`` with bounded size, compact JSON
payloads and deterministic, redacted errors. Stdlib only; no env, no fs, no
execution of message content.
"""

from __future__ import annotations

import json
from collections import deque
from typing import BinaryIO

JSON_RPC_VERSION = "2.0"
MAX_FRAME_BYTES = 131_072
MAX_PAYLOAD_BYTES = 65_536
MAX_SEEN_IDS = 256
HEADER_SEPARATOR = b"\r\n\r\n"

PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603
OVERSIZE_FRAME = -32010
DUPLICATE_ID = -32011

KNOWN_METHODS = {"handshake", "request", "cancel", "health", "error", "shutdown"}


class FrameRejected(ValueError):
    """A frame violated the bounded framing; the channel stays usable."""


def result_message(request_id: int, result: dict) -> dict:
    return {"jsonrpc": JSON_RPC_VERSION, "id": request_id, "result": result}


def error_message(request_id: int, code: int, detail: str) -> dict:
    return {
        "jsonrpc": JSON_RPC_VERSION,
        "id": request_id,
        "error": {"code": code, "message": detail[:256]},
    }


def dump(payload: dict) -> bytes:
    text = json.dumps(payload, separators=(",", ":"))
    encoded = text.encode("utf-8")
    if len(encoded) > MAX_PAYLOAD_BYTES:
        raise FrameRejected("payload exceeds the bounded size")
    return (
        b"Content-Length: "
        + str(len(encoded)).encode("ascii")
        + HEADER_SEPARATOR
        + encoded
    )


def write_frame(stream_out: BinaryIO, payload: dict) -> None:
    stream_out.write(dump(payload))
    stream_out.flush()


def _read_exact(stream_in: BinaryIO, count: int) -> bytes | None:
    chunks = bytearray()
    while len(chunks) < count:
        chunk = stream_in.read(count - len(chunks))
        if not chunk:
            return None
        chunks.extend(chunk)
    return bytes(chunks)


def read_frame(stream_in: BinaryIO) -> dict | None:
    """Reads one framed message; ``None`` on clean EOF.

    Malformed/oversized frames raise :class:`FrameRejected` so the caller
    applies its fail-closed policy without leaking payload content.
    """
    header = bytearray()
    while HEADER_SEPARATOR not in header:
        byte = stream_in.read(1)
        if not byte:
            return None
        header.extend(byte)
        if len(header) > MAX_FRAME_BYTES:
            raise FrameRejected("frame header exceeds the bounded size")
    length = None
    for line in header.decode("ascii", errors="replace").split("\r\n"):
        name, _, value = line.partition(":")
        if name.strip().lower() == "content-length":
            try:
                length = int(value.strip())
            except ValueError:
                length = None
    if length is None or length <= 0 or length > MAX_PAYLOAD_BYTES:
        raise FrameRejected("content-length header is invalid")
    payload = _read_exact(stream_in, length)
    if payload is None:
        raise FrameRejected("frame payload ended mid-stream")
    try:
        message = json.loads(payload.decode("utf-8"))
    except ValueError:
        raise FrameRejected("frame payload is not valid JSON") from None
    if not isinstance(message, dict):
        raise FrameRejected("frame payload is not a JSON object")
    return message


class SeenIds:
    """Bounded FIFO of recently used request ids (replay protection)."""

    def __init__(self, capacity: int = MAX_SEEN_IDS) -> None:
        self._ids: deque[int] = deque(maxlen=capacity)

    def register(self, request_id: int) -> bool:
        if request_id in self._ids:
            return False
        self._ids.append(request_id)
        return True


def structural_error(message: dict) -> int | None:
    """Maps a structurally invalid message to a JSON-RPC error code."""
    if message.get("jsonrpc") != JSON_RPC_VERSION:
        return INVALID_REQUEST
    if "method" in message and message.get("method") not in KNOWN_METHODS:
        return METHOD_NOT_FOUND
    if "id" in message and not isinstance(message["id"], int):
        return INVALID_REQUEST
    return None


def is_request(message: dict) -> bool:
    return "id" in message and "method" in message and "result" not in message and "error" not in message


def is_notification(message: dict) -> bool:
    return "id" not in message and "method" in message
