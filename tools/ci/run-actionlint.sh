#!/usr/bin/env bash
set -euo pipefail

readonly ACTIONLINT_VERSION='1.7.12'
readonly ACTIONLINT_SHA256='8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8'
readonly ACTIONLINT_ARCHIVE="actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz"
readonly ACTIONLINT_URL="https://github.com/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/${ACTIONLINT_ARCHIVE}"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

curl --fail --location --retry 3 --retry-all-errors --proto '=https' --tlsv1.2 \
  --silent --show-error --output "$tmp_dir/$ACTIONLINT_ARCHIVE" "$ACTIONLINT_URL"
printf '%s  %s\n' "$ACTIONLINT_SHA256" "$tmp_dir/$ACTIONLINT_ARCHIVE" | sha256sum -c -
tar --extract --gzip --file "$tmp_dir/$ACTIONLINT_ARCHIVE" --directory "$tmp_dir"
chmod 0755 "$tmp_dir/actionlint"

"$tmp_dir/actionlint" "$@"
