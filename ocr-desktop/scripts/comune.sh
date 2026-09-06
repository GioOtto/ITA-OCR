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
VULKAN_TP="$RADICE/thirdparty/vulkan"

# La versione ha una sola fonte, il manifesto di Tauri: il nome dell'AppImage
# ne e' una copia, come lo e' quello dell'installer Windows. Il bottone del
# sito punta al file per nome, quindi le due cose devono restare allineate --
# ci pensa scripts/verifica-pubblicazione.py a controllarlo.
VERSIONE="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([0-9.]*\)".*/\1/p' \
  "$APP/tauri.conf.json" | head -1)"
[ -n "$VERSIONE" ] || { echo "versione non leggibile da $APP/tauri.conf.json" >&2; exit 1; }
APPIMAGE_NOME="ITA-OCR-v${VERSIONE}-x86_64.AppImage"

# Le intestazioni di WebKitGTK/GTK3 stanno in un sysroot locale estratto dai
# .deb, per le macchine dove i pacchetti -dev non sono installati.
if [ -d "$SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig" ]; then
  export PKG_CONFIG_PATH="$SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig:$SYSROOT/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
fi

# Compilatore di shader e intestazioni SPIR-V, se prepara.sh li ha messi in
# thirdparty invece di trovarli nel sistema. Quello che c'e' nel PATH ha la
# precedenza: chi ha il Vulkan SDK installato usa il suo.
if [ -z "${GLSLC:-}" ] && [ -x "$VULKAN_TP/usr/bin/glslc" ]; then
  GLSLC="$VULKAN_TP/usr/bin/glslc"
  export GLSLC
fi
if [ -z "${SPIRV_HEADERS_DIR:-}" ]; then
  for c in "$VULKAN_TP"/usr/share/cmake/SPIRV-Headers "$VULKAN_TP"/usr/lib/*/cmake/SPIRV-Headers; do
    if [ -f "$c/SPIRV-HeadersConfig.cmake" ]; then
      SPIRV_HEADERS_DIR="$c"
      export SPIRV_HEADERS_DIR
      break
    fi
  done
fi
# ggml-vulkan include <spirv/unified1/spirv.hpp> ma il suo CMakeLists non
# collega il target SPIRV-Headers::SPIRV-Headers: si limita a find_package.
# Con il Vulkan SDK non si nota, perche' tiene spirv/ nella stessa include di
# vulkan/ e quella arriva da Vulkan::Vulkan. Prendendo le intestazioni dai .deb
# stanno altrove, e la directory va passata al compilatore a mano.
if [ -z "${SPIRV_HEADERS_INCLUDE:-}" ]; then
  for c in "$VULKAN_TP/usr/include" "$VULKAN_HEADERS" /usr/include; do
    if [ -f "$c/spirv/unified1/spirv.hpp" ]; then
      SPIRV_HEADERS_INCLUDE="$c"
      export SPIRV_HEADERS_INCLUDE
      break
    fi
  done
fi

# I sorgenti del motore e le intestazioni Vulkan sono submodule: appena clonato
# il repo sono cartelle vuote, e cmake fallirebbe con un errore che non dice
# questo.
for s in "$LLAMA_SORGENTE:llama.cpp" "$VULKAN_HEADERS:Vulkan-Headers"; do
  if [ ! -d "${s%%:*}" ] || [ -z "$(ls -A "${s%%:*}" 2>/dev/null)" ]; then
    echo "il submodule ${s##*:} non e' inizializzato." >&2
    echo "dalla radice del repo: git submodule update --init --recursive" >&2
    exit 1
  fi
done
