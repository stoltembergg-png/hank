"""Small, bounded client for the versioned Hank worker protocol.

The client wraps the existing JSON-RPC framing only. It does not spawn
processes, execute tools, persist secrets, or grant capabilities.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import BinaryIO

from python.runtime import transport

from .errors import SdkError

SCHEMA_VERSION = 1
PROTOCOL_VERSION = 1
MAX_ID_LENGTH = 128
MAX_CAPABILITIES = 32
MAX_PAYLOAD_BYTES = 65_536


@dataclass(frozen=True)
class WorkerContext:
    project_id: str
    session_id: str
    task_id: str | None
    trace_id: str

    def as_dict(self) -> dict[str, str | None]:
        for name, value in (
            ("project_id", self.project_id),
            ("session_id", self.session_id),
            ("trace_id", self.trace_id),
        ):
            _validate_id(name, value)
        if self.task_id is not None:
            _validate_id("task_id", self.task_id)
        return {
            "project_id": self.project_id,
            "session_id": self.session_id,
            "task_id": self.task_id,
            "trace_id": self.trace_id,
        }


class PythonWorkerClient:
    """Protocol-only client; lifecycle ownership remains in Rust runtime."""

    def __init__(
        self,
        stream_out: BinaryIO,
        stream_in: BinaryIO,
        worker_id: str,
        capabilities: list[str],
    ) -> None:
        _validate_id("worker_id", worker_id)
        if not 0 < len(capabilities) <= MAX_CAPABILITIES:
            raise SdkError("invalid_capabilities", "capability list is outside bounds")
        if any(not isinstance(value, str) or not value or len(value) > MAX_ID_LENGTH for value in capabilities):
            raise SdkError("invalid_capabilities", "capability list is outside bounds")
        self._out = stream_out
        self._in = stream_in
        self._worker_id = worker_id
        self._capabilities = list(capabilities)
        self._next_id = 1
        self._handshaked = False
        self._closed = False

    def handshake(self) -> dict:
        self._ensure_open()
        result = self._call(
            "handshake",
            {
                "schema_version": SCHEMA_VERSION,
                "protocol_version": PROTOCOL_VERSION,
                "worker_id": self._worker_id,
                "capabilities": self._capabilities,
            },
        )
        if result.get("kind") != "handshake_accepted":
            raise SdkError("invalid_handshake", "worker handshake was not accepted")
        self._handshaked = True
        return result

    def request(
        self,
        request_id: str,
        context: WorkerContext,
        capability: str,
        payload: dict,
    ) -> dict:
        self._ensure_ready()
        _validate_request_id(request_id)
        if not capability or len(capability) > MAX_ID_LENGTH or any(ord(char) < 32 for char in capability):
            raise SdkError("invalid_capability", "capability is outside bounds")
        context_dict = context.as_dict()
        _validate_payload(payload)
        return self._call(
            "request",
            {
                "request_id": request_id,
                "context": context_dict,
                "capability": capability,
                "payload": payload,
            },
        )

    def health(self) -> dict:
        self._ensure_ready()
        return self._call("health", {})

    def cancel(self, request_id: str, reason: str) -> None:
        self._ensure_ready()
        _validate_request_id(request_id)
        if reason not in {"user", "deadline", "session_closed", "shutdown"}:
            raise SdkError("invalid_cancel_reason", "cancel reason is not supported")
        transport.write_frame(
            self._out,
            {
                "jsonrpc": transport.JSON_RPC_VERSION,
                "method": "cancel",
                "params": {"request_id": request_id, "reason": reason},
            },
        )

    def shutdown(self) -> dict:
        self._ensure_ready()
        result = self._call("shutdown", {})
        if result.get("kind") != "shutdown_ack":
            raise SdkError("invalid_shutdown", "worker shutdown was not acknowledged")
        self._closed = True
        return result

    def _call(self, method: str, params: dict) -> dict:
        request_id = self._next_id
        self._next_id += 1
        transport.write_frame(
            self._out,
            {
                "jsonrpc": transport.JSON_RPC_VERSION,
                "id": request_id,
                "method": method,
                "params": params,
            },
        )
        response = transport.read_frame(self._in)
        if response is None:
            raise SdkError("worker_eof", "worker closed the protocol channel")
        if "error" in response:
            error = response.get("error")
            if not isinstance(error, dict):
                raise SdkError("protocol_error", "worker returned an invalid error")
            raise SdkError(str(error.get("code", "protocol_error")), str(error.get("message", "worker rejected request")))
        result = response.get("result")
        if not isinstance(result, dict):
            raise SdkError("protocol_error", "worker returned an invalid result")
        return result

    def _ensure_ready(self) -> None:
        self._ensure_open()
        if not self._handshaked:
            raise SdkError("invalid_state", "handshake is required before this operation")

    def _ensure_open(self) -> None:
        if self._closed:
            raise SdkError("closed", "worker client is closed")


def _validate_id(name: str, value: str) -> None:
    if not isinstance(value, str) or not value or len(value) > MAX_ID_LENGTH or any(ord(char) < 32 for char in value):
        raise SdkError("invalid_identity", f"{name} is outside bounds")


def _validate_request_id(value: str) -> None:
    _validate_id("request_id", value)
    if not value.startswith("req-"):
        raise SdkError("invalid_request_id", "request id is outside the worker protocol")


def _validate_payload(payload: dict) -> None:
    if not isinstance(payload, dict) or not payload:
        raise SdkError("invalid_payload", "payload must be a non-empty object")
    try:
        size = len(json.dumps(payload, separators=(",", ":")).encode("utf-8"))
    except (TypeError, ValueError):
        raise SdkError("invalid_payload", "payload is not serializable") from None
    if size > MAX_PAYLOAD_BYTES:
        raise SdkError("oversized_payload", "payload exceeds the bounded size")
