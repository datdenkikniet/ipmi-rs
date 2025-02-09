#!/bin/bash

# set -x

host=$1
shift

full_file_path="$1"
shift

base=$(basename "${full_file_path}")
relative_path="./.temp/${base}"

echo "Syncing files..."

ssh "${host}" mkdir -p .temp/
rsync -azP "$full_file_path" "${host}:${relative_path}"

echo "Executing command..."

ssh -t -q "$host" "sudo RUST_BACKTRACE="${RUST_BACKTRACE}" RUST_LOG="${RUST_LOG}" ${relative_path} $@"
