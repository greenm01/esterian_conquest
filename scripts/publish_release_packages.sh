#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/publish_release_packages.sh [--tag TAG] [--variant classic|unlocked] [--ec-connect-target TARGET] [--gpg-key KEYID]

Builds the selected release assets under releases/ and uploads them to an
existing GitHub Release with `gh release upload --clobber`.

If you do not pass any asset-selection flags, the default stays the historical
DOS release flow: both `classic` and `unlocked`.

Options:
  --tag TAG                 GitHub release tag to update.
                            Default: release-artifacts
  --variant classic         Build and upload only the classic package.
  --variant unlocked        Build and upload only the unlocked package.
  --ec-connect-target TARGET
                            Build and upload a public ec-connect archive.
                            Supported targets:
                              x86_64-unknown-linux-gnu
                              aarch64-apple-darwin
  --gpg-key KEYID           GPG key fingerprint or key ID used to sign the
                            public ec-connect checksum manifest. Required when
                            any --ec-connect-target is passed.
  -h, --help                Show this help text.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

release_tag="release-artifacts"
build_args=()
assets=()
want_classic=0
want_unlocked=0
ec_connect_targets=()
ec_connect_assets=()
selection_made=0
gpg_key=""

readonly ec_connect_release_note_url="https://github.com/greenm01/esterian_conquest/blob/main/docs/release-signing.md"
readonly ec_connect_checksum_path="releases/SHA256SUMS.txt"
readonly ec_connect_signature_path="releases/SHA256SUMS.txt.asc"
readonly public_ec_connect_targets=(
  "x86_64-unknown-linux-gnu"
  "aarch64-apple-darwin"
)

print_release_note() {
  local fingerprint="$1"
  cat <<EOF
## Verify Rust downloads

The Rust-built \`ec-connect\` downloads in this release can be verified with the signed \`SHA256SUMS.txt\` manifest.

\`gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt\`
\`shasum -a 256 -c SHA256SUMS.txt\`

Full instructions and public key: $ec_connect_release_note_url
Signing key fingerprint: \`$fingerprint\`
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      release_tag="$2"
      shift 2
      ;;
    --variant)
      case "$2" in
        classic)
          want_classic=1
          ;;
        unlocked)
          want_unlocked=1
          ;;
        *)
          echo "Unknown variant: $2" >&2
          usage >&2
          exit 2
          ;;
      esac
      selection_made=1
      build_args+=("--variant" "$2")
      shift 2
      ;;
    --ec-connect-target)
      case "$2" in
        x86_64-unknown-linux-gnu|aarch64-apple-darwin)
          ;;
        *)
          echo "Unknown ec-connect target: $2" >&2
          usage >&2
          exit 2
          ;;
      esac
      selection_made=1
      ec_connect_targets+=("$2")
      shift 2
      ;;
    --gpg-key)
      gpg_key="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ $selection_made -eq 0 ]]; then
  want_classic=1
  want_unlocked=1
fi

if [[ $want_classic -eq 1 || $want_unlocked -eq 1 ]]; then
  if [[ $want_classic -eq 1 ]]; then
    assets+=("releases/ec-v1.5-classic.zip")
  fi

  if [[ $want_unlocked -eq 1 ]]; then
    assets+=("releases/ec-v1.5-unlocked.zip")
  fi

  python3 scripts/build_release_packages.py "${build_args[@]}" --verify
fi

if [[ ${#ec_connect_targets[@]} -gt 0 ]]; then
  if [[ -z "$gpg_key" ]]; then
    echo "--gpg-key is required when publishing ec-connect release assets." >&2
    exit 2
  fi

  IFS=$'\n' read -r -d '' -a sorted_targets < <(printf '%s\n' "${ec_connect_targets[@]}" | sort -u && printf '\0')
  IFS=$'\n' read -r -d '' -a sorted_public_targets < <(printf '%s\n' "${public_ec_connect_targets[@]}" | sort -u && printf '\0')

  if [[ "${sorted_targets[*]}" != "${sorted_public_targets[*]}" ]]; then
    echo "Signed ec-connect publishing requires the full public player target set in one run:" >&2
    for target in "${public_ec_connect_targets[@]}"; do
      echo "  --ec-connect-target $target" >&2
    done
    exit 2
  fi
fi

for target in "${ec_connect_targets[@]}"; do
  archive_path="$(
    python3 scripts/build_playtest_bundle.py --artifact ec-connect --target "$target" --verify | tail -n 1
  )"
  assets+=("$archive_path")
  ec_connect_assets+=("$archive_path")
done

if [[ ${#assets[@]} -eq 0 ]]; then
  echo "No release assets selected." >&2
  usage >&2
  exit 2
fi

if [[ ${#ec_connect_assets[@]} -gt 0 ]]; then
  python3 scripts/write_release_checksums.py \
    --output "$ec_connect_checksum_path" \
    "${ec_connect_assets[@]}"
  gpg --batch --yes --armor --local-user "$gpg_key" \
    --output "$ec_connect_signature_path" \
    --detach-sign "$ec_connect_checksum_path"
  assets+=("$ec_connect_checksum_path" "$ec_connect_signature_path")

  resolved_fingerprint="$(
    gpg --batch --with-colons --fingerprint "$gpg_key" \
      | awk -F: '$1 == "fpr" { print $10; exit }'
  )"
  if [[ -z "$resolved_fingerprint" ]]; then
    echo "Unable to resolve a full fingerprint for GPG key: $gpg_key" >&2
    exit 2
  fi
fi

gh release upload "$release_tag" "${assets[@]}" --clobber

echo "Updated release assets on tag: $release_tag"
if [[ ${#ec_connect_assets[@]} -gt 0 ]]; then
  echo
  echo "Paste this at the top of the GitHub release body:"
  echo
  print_release_note "$resolved_fingerprint"
fi
