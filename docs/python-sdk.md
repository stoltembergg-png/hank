# Python SDK protocol-only

`python/sdk/` is a small client for the versioned Hank worker protocol. It
wraps the existing Content-Length framed JSON-RPC 2.0 transport and does not
create a process, execute tools, persist secrets or grant capabilities. Process
lifecycle remains owned by Rust `PythonLifecycle`.

## Client lifecycle

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
client.health()
client.cancel("req-1", "user")
client.shutdown()
```

The client must be handshaked before `request`, `health`, `cancel` or
`shutdown`. Handshake and shutdown are correlated requests and require the
expected `handshake_accepted`/`shutdown_ack` result. `cancel` is a JSON-RPC
notification: it is written without an expected response. After a successful
shutdown the client is closed and does not reopen or retry the channel.

## Versioning and bounded input

The SDK currently sends schema and protocol version `1`. The worker must
accept both versions during handshake; a different result fails as
`invalid_handshake` before the client is marked ready.

Before writing a request, the client validates:

- non-empty project, session, optional task, trace, worker and request
  identities, each at most 128 characters and without control characters;
- a non-empty capability at most 128 characters; this is descriptive protocol
  data and is not an authorization grant;
- request IDs beginning with `req-`;
- a serializable, non-empty object payload of at most 65,536 UTF-8 bytes;
- at most 32 worker capability labels.

Invalid context, IDs, capability or payload fail locally, before a frame is
written. The SDK does not retry a failed or partially read request, so callers
must choose a new operation/request identity only when their host-side policy
allows a new attempt.

## Trust and error boundary

Model, skill and provider text is data and is never interpreted by this
package. The SDK cannot register or execute tools, access the filesystem or
network, spawn subprocesses, persist state or authorize a capability.
Authorization, budgets, approval, worker lifecycle, isolation and recovery
remain host-side responsibilities.

`SdkError` exposes only a bounded code (64 characters) and detail (256
characters). Protocol EOF, malformed results and worker errors become typed
errors; the SDK does not copy the request payload into an error. The worker
side must still redact its own error messages before returning them.

## Rollback and recovery

The SDK has no persistent mutation to roll back. A failed handshake, request,
cancel or shutdown leaves recovery to the Rust supervisor, which owns process
cleanup, timeout/cancel policy and operation deduplication. The SDK does not
silently replay a request after EOF, timeout or reconnect.

## Verification

```bash
python3 -m unittest discover -s python/tests -p 'test_*.py'
node test/python-sdk-contract.spec.test.js
```

The Python suite uses in-memory framed streams and no network or external
provider dependency. The same acceptance path is included in
`node tools/run-all-tests.mjs`, whose result is recorded in
`.spec/verification/python-worker.json` for AC-699–AC-703.
