#!/usr/bin/env bash
set -euo pipefail

APP_NAME="calculatrice_qpur"
TARGET="wasm32-unknown-unknown"
OUT_DIR="docs"

# Dossier temporaire pour wasm-bindgen (NE PAS utiliser "web" sinon tu détruis web/index.html)
BINDGEN_DIR="dist_web"

# Détection automatique de index.html
if [ -f "index.html" ]; then
  HTML_SRC="index.html"
elif [ -f "web/index.html" ]; then
  HTML_SRC="web/index.html"
else
  echo "[ERR] Fichier HTML introuvable: index.html ni web/index.html"
  exit 1
fi

echo "[OK] HTML source détecté: $HTML_SRC"

echo "==> Build Rust WASM…"
cargo build --release --target "$TARGET"

echo "==> wasm-bindgen…"
rm -rf "$BINDGEN_DIR"
mkdir -p "$BINDGEN_DIR"

wasm-bindgen \
  "target/$TARGET/release/$APP_NAME.wasm" \
  --out-dir "$BINDGEN_DIR" \
  --target web \
  --no-typescript

echo "==> Préparation dossier $OUT_DIR/ (GitHub Pages)…"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

cp "$HTML_SRC" "$OUT_DIR/index.html"
cp "$BINDGEN_DIR"/*.js "$OUT_DIR/"
cp "$BINDGEN_DIR"/*.wasm "$OUT_DIR/"

echo
echo "======================================"
echo " Build WEB terminé ✅"
echo " Dossier prêt : $OUT_DIR/"
echo
echo " Test local :"
echo "   cd $OUT_DIR"
echo "   python3 -m http.server 8080"
echo "   ouvrir http://localhost:8080"
echo "======================================"
