#!/usr/bin/env bash

set -e

cd "`dirname $0`"

cargo publish -p fastnear-primitives
cargo publish -p fastnear-neardata-fetcher
