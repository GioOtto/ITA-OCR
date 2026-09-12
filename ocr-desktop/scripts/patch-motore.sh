#!/usr/bin/env bash
set -euo pipefail
llama_sorgente="${1:?serve il percorso dei sorgenti llama.cpp}"
patch_inferenza="$(cd "$(dirname "${BASH_SOURCE[0]}")/../patches" && pwd)/llama-inferenza.patch"
if git -C "$llama_sorgente" apply --reverse --check "$patch_inferenza" 2>/dev/null; then
  exit 0
fi
git -C "$llama_sorgente" apply --check "$patch_inferenza"
git -C "$llama_sorgente" apply "$patch_inferenza"
