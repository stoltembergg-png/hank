import io
import json
import unittest

from python.runtime import transport
from python.sdk.client import PythonWorkerClient, WorkerContext
from python.sdk.errors import SdkError


def framed(*messages):
    stream = io.BytesIO()
    for message in messages:
        transport.write_frame(stream, message)
    return io.BytesIO(stream.getvalue())


class PythonWorkerSdkTests(unittest.TestCase):
    # @spec:AC-699 @spec:AC-703
    def test_handshake_and_request_require_bounded_context(self):
        responses = framed(
            transport.result_message(1, {"kind": "handshake_accepted", "schema_version": 1}),
            transport.result_message(
                2,
                {
                    "kind": "response",
                    "schema_version": 1,
                    "request_id": "req-1",
                    "result": "not_supported",
                    "value": None,
                    "error": {"code": "invalid_message", "detail": "not supported"},
                },
            ),
        )
        output = io.BytesIO()
        client = PythonWorkerClient(
            output,
            responses,
            worker_id="worker-1",
            capabilities=["chat"],
        )
        client.handshake()
        result = client.request(
            "req-1",
            WorkerContext("project-1", "session-1", "task-1", "trace-1"),
            "chat",
            {"message": "hello"},
        )
        self.assertEqual(result["result"], "not_supported")
        self.assertIn(b'"method":"handshake"', output.getvalue())
        self.assertIn(b'"method":"request"', output.getvalue())

    # @spec:AC-700
    def test_invalid_context_and_oversized_payload_fail_before_write(self):
        client = PythonWorkerClient(io.BytesIO(), io.BytesIO(), "worker-1", ["chat"])
        with self.assertRaises(SdkError):
            client.request(
                "req-1",
                WorkerContext("", "session-1", "task-1", "trace-1"),
                "chat",
                {"message": "hello"},
            )
        with self.assertRaises(SdkError):
            client.request(
                "req-1",
                WorkerContext("project-1", "session-1", "task-1", "trace-1"),
                "chat",
                {"message": "x" * 70_000},
            )

    # @spec:AC-701
    def test_cancel_is_notification_and_shutdown_is_correlated(self):
        responses = framed(
            transport.result_message(1, {"kind": "handshake_accepted", "schema_version": 1}),
            transport.result_message(3, {"kind": "shutdown_ack", "schema_version": 1}),
        )
        output = io.BytesIO()
        client = PythonWorkerClient(output, responses, "worker-1", ["chat"])
        client.handshake()
        client.cancel("req-1", "user")
        client.shutdown()
        messages = []
        reader = io.BytesIO(output.getvalue())
        while True:
            message = transport.read_frame(reader)
            if message is None:
                break
            messages.append(message)
        self.assertEqual(messages[1]["method"], "cancel")
        self.assertNotIn("id", messages[1])
        self.assertEqual(messages[2]["method"], "shutdown")

    # @spec:AC-702
    def test_protocol_error_is_redacted(self):
        output = io.BytesIO()
        client = PythonWorkerClient(output, framed(transport.error_message(1, -32600, "bad")), "worker-1", ["chat"])
        with self.assertRaises(SdkError) as raised:
            client.handshake()
        self.assertNotIn("secret", str(raised.exception).lower())


if __name__ == "__main__":
    unittest.main()
