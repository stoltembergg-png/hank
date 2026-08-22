"""Stable, redacted SDK errors."""


class SdkError(Exception):
    """A bounded protocol/client error without raw payload content."""

    def __init__(self, code: str, detail: str) -> None:
        self.code = code[:64]
        self.detail = detail[:256]
        super().__init__(f"{self.code}: {self.detail}")
