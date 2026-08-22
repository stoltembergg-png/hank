# Python SDK

`python/sdk/` is a protocol-only wrapper for the versioned Hank worker contract.
It does not create a process, execute tools, persist secrets, or grant
capabilities. Process lifecycle remains owned by Rust `PythonLifecycle`.

## Safe usage

```python
from python.sdk import PythonWorkerClient, WorkerContext

client = PythonWorkerClient(stdout, stdin, "worker-1", ["chat"])
client.handshake()
result = client.request(
    "req-1",
    WorkerContext("project-1", "session-1", "task-1", "trace-1"),
    "chat",
    {"message": "bounded input"},
)
client.cancel("req-1", "user")
client.shutdown()
```

The client validates identity, capability, context, request IDs and payload
size before writing a frame. `cancel` is a JSON-RPC notification and does not
wait for a response. Handshake and shutdown are correlated requests.

## Trust boundary

Model/skill/provider text is data and is never interpreted by this package.
The SDK cannot register or execute tools; a capability string is descriptive
protocol data, not authorization. Authorization, budget policy and worker
lifecycle remain host-side responsibilities.

Protocol errors are bounded and redacted. No raw payload is inserted into
`SdkError` messages.

## Tests

```bash
python3 -m unittest discover -s python/tests -p 'test_*.py'
```

The same command is part of `node tools/run-all-tests.mjs` and therefore the
ONP/CI aggregate. The SDK tests use in-memory framed streams and contain no
network or external provider dependency.
