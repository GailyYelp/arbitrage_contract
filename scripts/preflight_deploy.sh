#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  preflight_deploy.sh --cluster <localnet|devnet|mainnet> [options]

Options:
  --provider-cluster <cluster>      Expected Anchor provider cluster. If omitted, uses
                                    ANCHOR_PROVIDER_URL inference or Anchor.toml [provider].
  --program-keypair <path>          Program keypair file whose pubkey must equal the selected
                                    Anchor program id.
  --wallet <path>                   Wallet keypair file; the script prints only its pubkey.
  --skip-keypair-check              Skip program-keypair pubkey validation.
  --skip-wallet-check               Skip wallet pubkey validation.
  -h, --help                        Show this help.

Environment:
  ALLOW_MAINNET_DEPLOY=1            Required when --cluster mainnet is selected.
  ANCHOR_PROVIDER_URL               Used to infer provider cluster when --provider-cluster is absent.
  ANCHOR_WALLET                     Used as wallet path when --wallet is absent.
  PROGRAM_KEYPAIR                   Used as program keypair path when --program-keypair is absent.
USAGE
}

die() {
  printf 'preflight_deploy: error: %s\n' "$*" >&2
  exit 1
}

info() {
  printf 'preflight_deploy: %s\n' "$*"
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
contract_dir="$repo_root/arbitrage_contract"
anchor_toml="$contract_dir/Anchor.toml"
contract_lib="$contract_dir/src/lib.rs"
client_program_ids="$repo_root/money_donkey/src/config/program_id.rs"

cluster="${DEPLOY_CLUSTER:-}"
provider_cluster="${ANCHOR_PROVIDER_CLUSTER:-}"
program_keypair="${PROGRAM_KEYPAIR:-}"
wallet="${ANCHOR_WALLET:-}"
skip_keypair_check=0
skip_wallet_check=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --cluster)
      [ "$#" -ge 2 ] || die "--cluster requires a value"
      cluster="$2"
      shift 2
      ;;
    --provider-cluster)
      [ "$#" -ge 2 ] || die "--provider-cluster requires a value"
      provider_cluster="$2"
      shift 2
      ;;
    --program-keypair)
      [ "$#" -ge 2 ] || die "--program-keypair requires a value"
      program_keypair="$2"
      shift 2
      ;;
    --wallet)
      [ "$#" -ge 2 ] || die "--wallet requires a value"
      wallet="$2"
      shift 2
      ;;
    --skip-keypair-check)
      skip_keypair_check=1
      shift
      ;;
    --skip-wallet-check)
      skip_wallet_check=1
      shift
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

require_file() {
  [ -f "$1" ] || die "required file not found: $1"
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

extract_declare_id() {
  sed -n 's/^[[:space:]]*declare_id!("\([^"]*\)").*/\1/p' "$contract_lib" | head -n 1
}

extract_rust_pubkey_const() {
  local const_name="$1"
  awk -v const_name="$const_name" '
    $0 ~ "pub const " const_name "[[:space:]]*:" { found = 1 }
    found && /pubkey!\("/ {
      line = $0
      sub(/^.*pubkey!\("/, "", line)
      sub(/"\).*$/, "", line)
      print line
      exit
    }
  ' "$client_program_ids"
}

cluster_from_provider_url() {
  case "${ANCHOR_PROVIDER_URL:-}" in
    *mainnet-beta*|*mainnet*) printf 'mainnet' ;;
    *devnet*) printf 'devnet' ;;
    *localhost*|*127.0.0.1*|*0.0.0.0*) printf 'localnet' ;;
    *) printf '' ;;
  esac
}

expand_path() {
  case "$1" in
    "~/"*) printf '%s/%s' "$HOME" "${1#~/}" ;;
    *) printf '%s' "$1" ;;
  esac
}

pubkey_from_keypair() {
  command -v solana-keygen >/dev/null 2>&1 || die "solana-keygen is required for keypair checks"
  local keypair_path
  keypair_path="$(expand_path "$1")"
  [ -f "$keypair_path" ] || die "keypair file not found"
  solana-keygen pubkey "$keypair_path" 2>/dev/null
}

require_file "$anchor_toml"
require_file "$contract_lib"
require_file "$client_program_ids"

if [ -z "$cluster" ]; then
  cluster="$(extract_toml_key "provider" "cluster" "$anchor_toml")"
fi

case "$cluster" in
  localnet|devnet|mainnet) ;;
  *) die "unsupported cluster: ${cluster:-<empty>}" ;;
esac

if [ "$cluster" = "mainnet" ] && [ "${ALLOW_MAINNET_DEPLOY:-}" != "1" ]; then
  die "mainnet preflight requires ALLOW_MAINNET_DEPLOY=1"
fi

anchor_program_id="$(extract_toml_key "programs.$cluster" "arbitrage_contract" "$anchor_toml")"
[ -n "$anchor_program_id" ] || die "missing [programs.$cluster].arbitrage_contract in Anchor.toml"

declared_program_id="$(extract_declare_id)"
[ -n "$declared_program_id" ] || die "missing declare_id! in src/lib.rs"

if [ "$declared_program_id" != "$anchor_program_id" ]; then
  die "declare_id does not match Anchor.toml for $cluster"
fi

case "$cluster" in
  mainnet)
    client_const="ARBITRAGE_CONTRACT_PROGRAM_ID"
    ;;
  devnet)
    client_const="ARBITRAGE_CONTRACT_PROGRAM_ID_DEVNET"
    ;;
  localnet)
    client_const=""
    ;;
esac

if [ -n "$client_const" ]; then
  client_program_id="$(extract_rust_pubkey_const "$client_const")"
  [ -n "$client_program_id" ] || die "missing client program id const: $client_const"
  if [ "$client_program_id" != "$anchor_program_id" ]; then
    die "client $client_const does not match Anchor.toml for $cluster"
  fi
else
  info "client program id check skipped for localnet"
fi

if [ -z "$provider_cluster" ]; then
  provider_cluster="$(cluster_from_provider_url)"
fi
if [ -z "$provider_cluster" ]; then
  provider_cluster="$(extract_toml_key "provider" "cluster" "$anchor_toml")"
fi
[ -n "$provider_cluster" ] || die "provider cluster is not configured"

if [ "$provider_cluster" != "$cluster" ]; then
  die "provider cluster '$provider_cluster' does not match deployment cluster '$cluster'"
fi

if [ -z "$wallet" ]; then
  wallet="$(extract_toml_key "provider" "wallet" "$anchor_toml")"
fi

if [ "$skip_keypair_check" -eq 0 ]; then
  [ -n "$program_keypair" ] || die "program keypair is required; pass --program-keypair or --skip-keypair-check"
  program_keypair_pubkey="$(pubkey_from_keypair "$program_keypair")"
  if [ "$program_keypair_pubkey" != "$anchor_program_id" ]; then
    die "program keypair pubkey does not match Anchor.toml for $cluster"
  fi
  info "program keypair pubkey matches selected program id"
else
  info "program keypair pubkey check skipped"
fi

if [ "$skip_wallet_check" -eq 0 ]; then
  [ -n "$wallet" ] || die "wallet is required; pass --wallet, ANCHOR_WALLET, or --skip-wallet-check"
  wallet_pubkey="$(pubkey_from_keypair "$wallet")"
  info "wallet pubkey: $wallet_pubkey"
else
  info "wallet pubkey check skipped"
fi

info "cluster: $cluster"
info "program id: $anchor_program_id"
info "provider cluster: $provider_cluster"
info "preflight passed"
