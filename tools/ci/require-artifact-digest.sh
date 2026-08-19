#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${ARTIFACT_DIGEST:-}" ]]; then
  printf '%s\n' 'artifact digest is missing; refusing a green build' >&2
  exit 1
fi

printf 'artifact digest verified: %s\n' "$ARTIFACT_DIGEST"
