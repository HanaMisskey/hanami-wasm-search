#!/bin/bash
set -e

# WASMにコンパイル
wasm-pack build --target nodejs

# ベンチマークを実行
node benchmark.js --expose-gc