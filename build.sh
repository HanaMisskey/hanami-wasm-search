#!/bin/bash
set -e

# Clean previous build
rm -rf pkg

# Build optimized WebAssembly
echo "Building optimized WebAssembly..."
wasm-pack build --target bundler --release

# Remove unnecessary files from package
echo "Optimizing package size..."
cd pkg
find . -name ".gitignore" -delete

# Update package.json with correct metadata
echo "Updating package.json metadata..."
if ! command -v jq &> /dev/null; then
  echo "Warning: 'jq' not found, skipping package.json optimization"
else
  # Ensure all metadata is correct
  # This can be enhanced if you need to update more fields
  jq '.repository.url = "https://github.com/HanaMisskey/hanami-wasm-search"' package.json > tmp.json && mv tmp.json package.json
  jq '.author = "HanaMisskey"' package.json > tmp.json && mv tmp.json package.json
fi

# Optimize wasm file size if wasm-opt is available
if command -v wasm-opt &> /dev/null; then
  echo "Optimizing WebAssembly binary size..."
  wasm-opt -Oz -o hanami_wasm_search_bg.wasm.opt hanami_wasm_search_bg.wasm
  mv hanami_wasm_search_bg.wasm.opt hanami_wasm_search_bg.wasm
else
  echo "Note: Install wasm-opt from binaryen for additional size optimization"
fi

# Show package size information
echo "Package size information:"
du -sh .
du -sh *.wasm

echo "Build complete! Package is ready in the pkg/ directory"
echo "To publish: cd pkg && npm publish"
