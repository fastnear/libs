#!/usr/bin/env bash

set -e

cd "`dirname $0`"

cargo publish -p fastnear-primitives fastnear-neardata-fetcher
