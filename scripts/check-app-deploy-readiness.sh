#!/usr/bin/env bash
set -euo pipefail

app_dir="${1:?usage: check-app-deploy-readiness.sh <app-directory>}"
manifest=""
for candidate in "$app_dir/manifest.yaml" "$app_dir/manifest.json"; do
  if [ -f "$candidate" ]; then
    manifest="$candidate"
    break
  fi
done

if [ -z "$manifest" ]; then
  printf 'app deployment readiness: no manifest found in %s\n' "$app_dir" >&2
  exit 1
fi

placeholder_file="${BASH_SOURCE[0]%/*}/provider-registration-placeholders.txt"
while IFS= read -r placeholder; do
  [ -n "$placeholder" ] || continue
  if grep -qF -- "$placeholder" "$manifest"; then
    printf '::error::app %s still uses a provider registration placeholder (%s); register the provider app and replace the public manifest value before deployment\n' \
      "$app_dir" "$placeholder" >&2
    exit 1
  fi
done < "$placeholder_file"

if grep -qE '^id:[[:space:]]*github[[:space:]]*$' "$manifest"; then
  for secret_name in client_secret private_key; do
    if ! grep -qE "^[[:space:]]*-[[:space:]]+name:[[:space:]]*${secret_name}[[:space:]]*$" "$manifest"; then
      printf '::error::GitHub App manifest must require the %s deployment secret\n' \
        "$secret_name" >&2
      exit 1
    fi
  done
fi
