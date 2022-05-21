#!/usr/bin/env bash

set -e

cargo build
cargo clippy
