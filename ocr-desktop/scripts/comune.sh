# Percorsi e variabili condivise dagli script di build.
set -euo pipefail

RADICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROGETTO="$(cd "$RADICE/.." && pwd)"
LLAMA_SORGENTE="$PROGETTO/ocr-ita/vendor/llama.cpp"
VULKAN_HEADERS="$PROGETTO/ocr-ita/vendor/Vulkan-Headers/include"
BUILD_LLAMA="$RADICE/build/llama-unified"
APP="$RADICE/app/src-tauri"
DIST="$RADICE/dist"
APPDIR="$DIST/ITA-OCR.AppDir"
NOME_APP="ocr-ita-desktop"
MODELLI_SORGENTE="$PROGETTO/ocr-ita/models/gguf"
GGUF_LM="glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf"
GGUF_MMPROJ="glm-ocr-base-mmproj-q8_0.gguf"
SYSROOT="$RADICE/thirdparty/sysroot"

# Le intestazioni di WebKitGTK/GTK3 stanno in un sysroot locale estratto dai
# .deb, perche' su questa macchina i pacchetti -dev non sono installati.
if [ -d "$SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig" ]; then
  export PKG_CONFIG_PATH="$SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig:$SYSROOT/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
fi
