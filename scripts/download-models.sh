#!/usr/bin/env bash
set -euo pipefail

# Downloads ONNX models and ONNX Runtime for Monolith MCP.
# Usage: ./scripts/download-models.sh [--models-only] [--ort-only]
#
# Models downloaded:
#   - nomic-embed-text-v1.5 (522MB) — 768-d embeddings
#   - cross-encoder/ms-marco-MiniLM-L-6-v2 (87MB) — reranker
#
# ONNX Runtime:
#   - v1.23.0 for your platform (Linux x64, macOS arm64/x64)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="${DATA_DIR:-$REPO_ROOT/data}"
MODELS_DIR="$DATA_DIR/models"
LIB_DIR="$REPO_ROOT/lib"

ORT_VERSION="1.23.0"

SKIP_MODELS=false
SKIP_ORT=false

for arg in "$@"; do
  case "$arg" in
    --models-only) SKIP_ORT=true ;;
    --ort-only) SKIP_MODELS=true ;;
    --help|-h)
      echo "Usage: $0 [--models-only] [--ort-only]"
      echo "  --models-only   Skip ONNX Runtime download"
      echo "  --ort-only      Skip model downloads"
      exit 0
      ;;
  esac
done

command -v curl >/dev/null 2>&1 || { echo "Error: curl is required but not installed."; exit 1; }

# ─── Models ───────────────────────────────────────────────────────────────────

download_models() {
  mkdir -p "$MODELS_DIR"

  echo ""
  echo "=== Downloading Nomic Embed v1.5 (embedder, ~522MB) ==="
  if [ -f "$MODELS_DIR/nomic-embed.onnx" ]; then
    echo "  Already exists, skipping. Delete to re-download."
  else
    curl -L --progress-bar \
      -o "$MODELS_DIR/nomic-embed.onnx" \
      "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/onnx/model.onnx"
  fi

  if [ -f "$MODELS_DIR/tokenizer.json" ]; then
    echo "  tokenizer.json already exists, skipping."
  else
    curl -L --progress-bar \
      -o "$MODELS_DIR/tokenizer.json" \
      "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/tokenizer.json"
  fi

  echo ""
  echo "=== Downloading Cross-Encoder Reranker (~87MB) ==="
  if [ -f "$MODELS_DIR/reranker.onnx" ]; then
    echo "  Already exists, skipping."
  else
    curl -L --progress-bar \
      -o "$MODELS_DIR/reranker.onnx" \
      "https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/onnx/model.onnx"
  fi

  if [ -f "$MODELS_DIR/reranker-tokenizer.json" ]; then
    echo "  reranker-tokenizer.json already exists, skipping."
  else
    curl -L --progress-bar \
      -o "$MODELS_DIR/reranker-tokenizer.json" \
      "https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/tokenizer.json"
  fi

  echo ""
  echo "Models downloaded to: $MODELS_DIR"
}

# ─── ONNX Runtime ─────────────────────────────────────────────────────────────

download_ort() {
  mkdir -p "$LIB_DIR"

  OS="$(uname -s)"
  ARCH="$(uname -m)"

  echo ""
  echo "=== Downloading ONNX Runtime v${ORT_VERSION} ==="

  if [ "$OS" = "Linux" ] && [ "$ARCH" = "x86_64" ]; then
    ORT_FILE="onnxruntime-linux-x64-${ORT_VERSION}.tgz"
    ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_FILE}"
    ORT_LIB="libonnxruntime.so.${ORT_VERSION}"

    if [ -f "$LIB_DIR/$ORT_LIB" ]; then
      echo "  Already exists, skipping."
    else
      echo "  Downloading $ORT_FILE ..."
      curl -L --progress-bar -o "/tmp/$ORT_FILE" "$ORT_URL"
      tar xzf "/tmp/$ORT_FILE" -C /tmp
      cp "/tmp/onnxruntime-linux-x64-${ORT_VERSION}/lib/$ORT_LIB" "$LIB_DIR/"
      ln -sf "$ORT_LIB" "$LIB_DIR/libonnxruntime.so"
      rm -rf "/tmp/$ORT_FILE" "/tmp/onnxruntime-linux-x64-${ORT_VERSION}"
    fi

    echo ""
    echo "Set in your .env:"
    echo "  ORT_DYLIB_PATH=$LIB_DIR/$ORT_LIB"

  elif [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
      ORT_FILE="onnxruntime-osx-arm64-${ORT_VERSION}.tgz"
    else
      ORT_FILE="onnxruntime-osx-x86_64-${ORT_VERSION}.tgz"
    fi
    ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_FILE}"
    ORT_LIB="libonnxruntime.${ORT_VERSION}.dylib"

    if [ -f "$LIB_DIR/$ORT_LIB" ]; then
      echo "  Already exists, skipping."
    else
      echo "  Downloading $ORT_FILE ..."
      curl -L --progress-bar -o "/tmp/$ORT_FILE" "$ORT_URL"
      EXTRACT_DIR=$(basename "$ORT_FILE" .tgz)
      tar xzf "/tmp/$ORT_FILE" -C /tmp
      cp "/tmp/$EXTRACT_DIR/lib/$ORT_LIB" "$LIB_DIR/"
      ln -sf "$ORT_LIB" "$LIB_DIR/libonnxruntime.dylib"
      rm -rf "/tmp/$ORT_FILE" "/tmp/$EXTRACT_DIR"
    fi

    echo ""
    echo "Set in your .env:"
    echo "  ORT_DYLIB_PATH=$LIB_DIR/$ORT_LIB"

  else
    echo ""
    echo "  Unsupported platform: $OS/$ARCH"
    echo "  Download ONNX Runtime v${ORT_VERSION} manually from:"
    echo "  https://github.com/microsoft/onnxruntime/releases/tag/v${ORT_VERSION}"
    echo "  Place the shared library in $LIB_DIR/ and set ORT_DYLIB_PATH in .env"
  fi
}

# ─── Main ─────────────────────────────────────────────────────────────────────

echo "Monolith MCP — Model & Runtime Setup"
echo "====================================="

if [ "$SKIP_MODELS" = false ]; then
  download_models
fi

if [ "$SKIP_ORT" = false ]; then
  download_ort
fi

echo ""
echo "Done! Next steps:"
echo "  1. cp .env.example .env"
echo "  2. Edit .env with your ORT_DYLIB_PATH and API keys"
echo "  3. cargo build --release"
echo "  4. ./target/release/rag-mcp serve"
