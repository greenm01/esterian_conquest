#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/publish_release_packages.sh [--tag TAG] [--variant classic|unlocked]

Builds the DOS release bundles under releases/ and uploads the selected assets
to an existing GitHub Release with `gh release upload --clobber`.

Options:
  --tag TAG                 GitHub release tag to update.
                            Default: v1.5-dos-bundles
  --variant classic         Build and upload only the classic package.
  --variant unlocked        Build and upload only the unlocked package.
  -h, --help                Show this help text.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

release_tag="v1.5-dos-bundles"
build_args=()
assets=()
want_classic=0
want_unlocked=0

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
      build_args+=("--variant" "$2")
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

if [[ $want_classic -eq 0 && $want_unlocked -eq 0 ]]; then
  want_classic=1
  want_unlocked=1
fi

if [[ $want_classic -eq 1 ]]; then
  assets+=("releases/ec-v1.5-classic.zip")
fi

if [[ $want_unlocked -eq 1 ]]; then
  assets+=("releases/ec-v1.5-unlocked.zip")
fi

python3 scripts/build_release_packages.py "${build_args[@]}" --verify
gh release upload "$release_tag" "${assets[@]}" --clobber

echo "Updated release assets on tag: $release_tag"
