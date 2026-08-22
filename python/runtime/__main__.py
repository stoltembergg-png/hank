"""Package entrypoint for the minimal Hank Python worker.

Run from the ``python`` directory with ``python -m runtime``. The worker is
optional: the Rust core starts, tests and runs without any Python runtime.
"""

from __future__ import annotations

from .worker import main

raise SystemExit(main())
