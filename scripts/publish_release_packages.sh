#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/publish_release_packages.sh [--tag TAG] [--variant classic|unlocked] [--ec-connect-target TARGET] [--gpg-key KEYID]

Builds the selected release assets under releases/ and uploads them to an
existing GitHub Release with `gh release upload --clobber`. When public
`ec-connect` assets are included, the script also refreshes the shared signed
checksum manifest and updates the release-body verification notice in place.

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
                            any --ec-connect-target is passed. The manifest
                            includes the selected build(s) plus any other
                            already-published public ec-connect archives on
                            the release.
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
ec_connect_manifest_assets=()
selection_made=0
gpg_key=""

readonly ec_connect_release_note_url="https://github.com/greenm01/esterian_conquest/blob/main/docs/release-signing.md"
readonly ec_connect_checksum_path="releases/SHA256SUMS.txt"
readonly ec_connect_signature_path="releases/SHA256SUMS.txt.asc"
readonly ec_connect_release_note_path="releases/ec-connect-release-note.md"

is_selected_ec_connect_asset() {
  local candidate="$1"
  local selected
  for selected in "${ec_connect_assets[@]}"; do
    if [[ "$(basename "$selected")" == "$candidate" ]]; then
      return 0
    fi
  done
  return 1
}

download_existing_ec_connect_assets() {
  local download_dir="$1"
  local asset_name=""
  while IFS= read -r asset_name; do
    [[ -n "$asset_name" ]] || continue
    if is_selected_ec_connect_asset "$asset_name"; then
      continue
    fi
    gh release download "$release_tag" --pattern "$asset_name" --dir "$download_dir"
    ec_connect_manifest_assets+=("$download_dir/$asset_name")
  done < <(
    gh release view "$release_tag" --json assets --jq \
      '.assets[].name | select(test("^ec-connect-v.*-(linux-x64|macos-arm64)\\.tar\\.gz$"))'
  )
}

write_release_note() {
  local fingerprint="$1"
  cat >"$ec_connect_release_note_path" <<EOF
<!-- EC-RUST-VERIFY:START -->
## Verify Rust downloads

The Rust-built \`ec-connect\` downloads in this release can be verified with the signed \`SHA256SUMS.txt\` manifest.

\`gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt\`
\`shasum -a 256 -c SHA256SUMS.txt\`

Full instructions and public key: $ec_connect_release_note_url
Signing key fingerprint: \`$fingerprint\`

The signed manifest covers the public \`ec-connect\` archives, not the DOS compatibility bundles on this page.
<!-- EC-RUST-VERIFY:END -->
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

if [[ ${#ec_connect_targets[@]} -gt 0 && -z "$gpg_key" ]]; then
  echo "--gpg-key is required when publishing ec-connect release assets." >&2
  exit 2
fi

for target in "${ec_connect_targets[@]}"; do
  archive_path="$(
    python3 scripts/build_playtest_bundle.py --artifact ec-connect --target "$target" --verify | tail -n 1
  )"
  assets+=("$archive_path")
  ec_connect_assets+=("$archive_path")
  ec_connect_manifest_assets+=("$archive_path")
done

if [[ ${#assets[@]} -eq 0 ]]; then
  echo "No release assets selected." >&2
  usage >&2
  exit 2
fi

if [[ ${#ec_connect_assets[@]} -gt 0 ]]; then
  manifest_download_dir="$(mktemp -d)"
  trap 'rm -rf "$manifest_download_dir"' EXIT
  download_existing_ec_connect_assets "$manifest_download_dir"

  python3 scripts/write_release_checksums.py \
    --output "$ec_connect_checksum_path" \
    "${ec_connect_manifest_assets[@]}"
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

  write_release_note "$resolved_fingerprint"
fi

gh release upload "$release_tag" "${assets[@]}" --clobber

if [[ ${#ec_connect_assets[@]} -gt 0 ]]; then
  existing_release_body="$(mktemp)"
  merged_release_body="$(mktemp)"
  gh release view "$release_tag" --json body --jq '.body' >"$existing_release_body"
  python3 scripts/upsert_release_note.py \
    --body-file "$existing_release_body" \
    --note-file "$ec_connect_release_note_path" \
    --output "$merged_release_body"
  gh release edit "$release_tag" --notes-file "$merged_release_body"
  rm -f "$existing_release_body" "$merged_release_body"
fi

echo "Updated release assets on tag: $release_tag"
if [[ ${#ec_connect_assets[@]} -gt 0 ]]; then
  echo "Updated the release-body verification notice automatically."
fi
