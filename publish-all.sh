#!/usr/bin/env bash
# Publish the workspace crates to crates.io in dependency order.
#
# Crates whose current version is already on crates.io are skipped, so the
# release workflow also succeeds after a local `cargo release` publish.
#
#   ./publish-all.sh            publish (needs CARGO_REGISTRY_TOKEN or `cargo login`)
#   DRY_RUN=1 ./publish-all.sh  package and verify only
set -euo pipefail
cd "$(dirname "$0")"

CRATES=(blutils-core blutils-proc-adapter blutils-cli)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
USER_AGENT="blutils-publish (https://github.com/LepistaBioinformatics/blutils)"

published() { # crate
  local status
  status=$(curl -s -o /dev/null -w '%{http_code}' -A "$USER_AGENT" \
    "https://crates.io/api/v1/crates/$1/$VERSION")
  case "$status" in
    200) return 0 ;;
    404) return 1 ;;
    *) echo "Could not query crates.io for $1 $VERSION (HTTP $status)" >&2; exit 1 ;;
  esac
}

for crate in "${CRATES[@]}"; do
  if published "$crate"; then
    echo "$crate $VERSION is already on crates.io; skipping."
    continue
  fi
  if [ "${DRY_RUN:-0}" = 1 ]; then
    cargo publish -p "$crate" --dry-run
  else
    # cargo waits until each crate is available before the next one starts.
    cargo publish -p "$crate"
  fi
done
