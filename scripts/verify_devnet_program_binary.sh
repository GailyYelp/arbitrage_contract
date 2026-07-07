#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  verify_devnet_program_binary.sh [options]

Options:
  --program-id <pubkey>       Program id to verify. Defaults to Anchor.toml [programs.devnet].
  --local-so <path>           Local program shared object. Defaults to target/deploy/arbitrage_contract.so.
  --url <rpc-url>             Solana RPC URL. Defaults to https://api.devnet.solana.com.
  --dump-path <path>          Path for the downloaded on-chain binary. Defaults to /private/tmp.
  -h, --help                  Show this help.

This script is read-only with respect to Solana. It dumps the deployed program
binary through RPC, compares it with the local build, and exits non-zero when
they differ.
USAGE
}

die() {
  printf 'verify_devnet_program_binary: error: %s\n' "$*" >&2
  exit 1
}

info() {
  printf 'verify_devnet_program_binary: %s\n' "$*"
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
contract_dir="$repo_root/arbitrage_contract"
anchor_toml="$contract_dir/Anchor.toml"

program_id="${PROGRAM_ID:-}"
local_so="${LOCAL_PROGRAM_SO:-$contract_dir/target/deploy/arbitrage_contract.so}"
rpc_url="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
dump_path="${PROGRAM_DUMP_PATH:-}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --program-id)
      [ "$#" -ge 2 ] || die "--program-id requires a value"
      program_id="$2"
      shift 2
      ;;
    --local-so)
      [ "$#" -ge 2 ] || die "--local-so requires a value"
      local_so="$2"
      shift 2
      ;;
    --url)
      [ "$#" -ge 2 ] || die "--url requires a value"
      rpc_url="$2"
      shift 2
      ;;
    --dump-path)
      [ "$#" -ge 2 ] || die "--dump-path requires a value"
      dump_path="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "$1 is required"
}

extract_toml_key() {
  local section="$1"
  local key="$2"
  local file="$3"
  awk -v section="[$section]" -v key="$key" '
    $0 == section { active = 1; next }
    /^\[/ { active = 0 }
    active {
      line = $0
      sub(/[[:space:]]*#.*/, "", line)
      if (line ~ "^[[:space:]]*" key "[[:space:]]*=") {
        sub(/^.*=[[:space:]]*"/, "", line)
        sub(/".*$/, "", line)
        print line
        exit
      }
    }
  ' "$file"
}

file_size_bytes() {
  wc -c < "$1" | tr -d '[:space:]'
}

file_sha256() {
  shasum -a 256 "$1" | awk '{print $1}'
}

require_command solana
require_command shasum
require_command awk

[ -f "$anchor_toml" ] || die "Anchor.toml not found: $anchor_toml"
[ -f "$local_so" ] || die "local program binary not found: $local_so"

if [ -z "$program_id" ]; then
  program_id="$(extract_toml_key "programs.devnet" "arbitrage_contract" "$anchor_toml")"
fi
[ -n "$program_id" ] || die "missing devnet program id"

if [ -z "$dump_path" ]; then
  dump_path="/private/tmp/raindarker-${program_id}-devnet.so"
fi

info "rpc url: $rpc_url"
info "program id: $program_id"
info "local binary: $local_so"
info "dump path: $dump_path"

solana program show "$program_id" --url "$rpc_url"
solana program dump "$program_id" "$dump_path" --url "$rpc_url" >/dev/null

local_size="$(file_size_bytes "$local_so")"
dump_size="$(file_size_bytes "$dump_path")"
local_sha="$(file_sha256 "$local_so")"
dump_sha="$(file_sha256 "$dump_path")"

info "local size: $local_size"
info "chain size: $dump_size"
info "local sha256: $local_sha"
info "chain sha256: $dump_sha"

if [ "$local_sha" != "$dump_sha" ]; then
  die "deployed program binary does not match local build"
fi

info "deployed program binary matches local build"
