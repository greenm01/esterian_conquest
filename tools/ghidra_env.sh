#!/usr/bin/env bash

# Shared Ghidra path helpers for the repo's headless wrappers.

resolve_ghidra_home() {
  local repo_root="${1:-}"
  local candidate

  if [[ -n "${GHIDRA_HOME:-}" && -x "${GHIDRA_HOME}/support/analyzeHeadless" ]]; then
    printf '%s\n' "$GHIDRA_HOME"
    return 0
  fi

  if command -v ghidra-analyzeHeadless >/dev/null 2>&1; then
    candidate=$(cd "$(dirname "$(command -v ghidra-analyzeHeadless)")/.." && pwd)
    if [[ -x "${candidate}/support/analyzeHeadless" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  fi

  for candidate in /opt/ghidra /usr/share/ghidra; do
    if [[ -x "${candidate}/support/analyzeHeadless" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  if [[ -n "$repo_root" && -x "${repo_root}/ghidra/support/analyzeHeadless" ]]; then
    printf '%s\n' "${repo_root}/ghidra"
    return 0
  fi

  for candidate in "$HOME"/tools/ghidra_*_PUBLIC; do
    if [[ -x "${candidate}/support/analyzeHeadless" ]]; then
      printf '%s\n' "$candidate"
    fi
  done | sort -V | tail -n 1 | grep -q . && {
    for candidate in "$HOME"/tools/ghidra_*_PUBLIC; do
      if [[ -x "${candidate}/support/analyzeHeadless" ]]; then
        printf '%s\n' "$candidate"
      fi
    done | sort -V | tail -n 1
    return 0
  }

  return 1
}

resolve_analyze_headless() {
  local repo_root="${1:-}"
  local ghidra_home
  ghidra_home=$(resolve_ghidra_home "$repo_root") || return 1
  printf '%s\n' "${ghidra_home}/support/analyzeHeadless"
}
