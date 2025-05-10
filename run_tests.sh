#!/bin/bash
set -e

# WASMにコンパイル
wasm-pack build --target nodejs
# テストを実行
node test.js
