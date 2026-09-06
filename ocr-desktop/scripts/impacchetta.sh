#!/usr/bin/env bash
# Assembla la cartella portabile (AppDir) e, se possibile, l'AppImage.
source "$(dirname "${BASH_SOURCE[0]}")/comune.sh"

BINARIO="$APP/target/release/$NOME_APP"
[ -x "$BINARIO" ] || { echo "manca $BINARIO: esegui prima scripts/costruisci.sh" >&2; exit 1; }
[ -x "$BUILD_LLAMA/bin/llama-server" ] || { echo "manca llama-server" >&2; exit 1; }

RISORSE="$APPDIR/usr/lib/$NOME_APP"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$RISORSE/llama" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"

echo "==> eseguibile"
install -m 0755 "$BINARIO" "$APPDIR/usr/bin/$NOME_APP"

echo "==> motore llama.cpp"
install -m 0755 "$BUILD_LLAMA/bin/llama-server" "$RISORSE/llama/"
# Le librerie del motore: quelle di llama/ggml piu' tutte le varianti CPU e il
# backend Vulkan, che ggml carica da sola dalla directory dell'eseguibile.
cp -a "$BUILD_LLAMA"/bin/libllama.so* "$BUILD_LLAMA"/bin/libllama-common.so* \
      "$BUILD_LLAMA"/bin/libllama-server-impl.so "$BUILD_LLAMA"/bin/libmtmd.so* \
      "$BUILD_LLAMA"/bin/libggml.so* "$BUILD_LLAMA"/bin/libggml-base.so* \
      "$BUILD_LLAMA"/bin/libggml-cpu-*.so "$BUILD_LLAMA"/bin/libggml-vulkan.so \
      "$RISORSE/llama/"
# Il backend CUDA c'e' solo se la macchina di build aveva nvcc: ggml lo carica
# da sola a runtime se lo trova, quindi il pacchetto resta valido comunque.
if [ -f "$BUILD_LLAMA/bin/libggml-cuda.so" ]; then
  echo "==> incluso il backend CUDA"
  cp -a "$BUILD_LLAMA"/bin/libggml-cuda.so "$RISORSE/llama/"
else
  echo "==> nessun backend CUDA da includere"
fi
# Il RUNPATH assoluto della build va sostituito, altrimenti il pacchetto resta
# legato alla directory in cui e' stato compilato.
if command -v patchelf >/dev/null; then
  patchelf --set-rpath '$ORIGIN' "$RISORSE/llama/llama-server"
  for lib in "$RISORSE"/llama/*.so*; do
    [ -L "$lib" ] || patchelf --set-rpath '$ORIGIN' "$lib" 2>/dev/null || true
  done
fi

echo "==> PDFium"
install -m 0644 "$RADICE/thirdparty/pdfium/lib/libpdfium.so" "$RISORSE/libpdfium.so"

echo "==> dizionario italiano pubblico"
mkdir -p "$RISORSE/dictionaries/it_IT"
install -m 0644 "$RADICE/resources/dictionaries/it_IT/it_IT.aff" \
  "$RADICE/resources/dictionaries/it_IT/it_IT.dic" \
  "$RADICE/resources/dictionaries/it_IT/COPYING" \
  "$RADICE/resources/dictionaries/it_IT/LICENSE-GPL-3.txt" \
  "$RISORSE/dictionaries/it_IT/"

# L'inglese serve solo a riconoscere le parole da lasciare stare, mai a
# correggerle. Assente, l'app funziona senza quella protezione.
if [ -d "$RADICE/resources/dictionaries/en_US" ]; then
  echo "==> dizionario inglese (solo riconoscimento)"
  mkdir -p "$RISORSE/dictionaries/en_US"
  install -m 0644 "$RADICE/resources/dictionaries/en_US/en_US.aff" \
    "$RADICE/resources/dictionaries/en_US/en_US.dic" \
    "$RADICE/resources/dictionaries/en_US/README_en_US.txt" \
    "$RISORSE/dictionaries/en_US/"
fi

echo "==> modelli"
# I due GGUF pesano 1,15 GB: restano **fuori** dall'AppImage, in dist/models/.
# L'app li cerca accanto all'AppImage, nelle risorse o in OCR_ITA_MODELS.
mkdir -p "$DIST/models"
for gguf in "$GGUF_LM" "$GGUF_MMPROJ"; do
  if [ ! -e "$DIST/models/$gguf" ]; then
    ln "$MODELLI_SORGENTE/$gguf" "$DIST/models/$gguf" 2>/dev/null \
      || cp "$MODELLI_SORGENTE/$gguf" "$DIST/models/$gguf"
  fi
done
if [ "${MODELLI_NEL_PACCHETTO:-0}" = "1" ]; then
  mkdir -p "$RISORSE/models"
  cp "$DIST/models/$GGUF_LM" "$DIST/models/$GGUF_MMPROJ" "$RISORSE/models/"
fi

echo "==> metadati desktop"
cat > "$APPDIR/$NOME_APP.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=ITA-OCR
Comment=Trascrizione di manoscritti italiani, in locale
Exec=$NOME_APP
Icon=$NOME_APP
Categories=Office;Utility;
Terminal=false
StartupWMClass=ocr-ita-desktop
DESKTOP
cp "$APPDIR/$NOME_APP.desktop" "$APPDIR/usr/share/applications/"
cp "$APP/icons/icon.png" "$APPDIR/$NOME_APP.png"
cp "$APP/icons/icon.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/$NOME_APP.png"

cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/sh
# Trova le risorse e i modelli senza dipendere dalla directory di lavoro.
QUI="$(dirname "$(readlink -f "$0")")"
export APPDIR="${APPDIR:-$QUI}"
export OCR_ITA_RESOURCES="${OCR_ITA_RESOURCES:-$QUI/usr/lib/ocr-ita-desktop}"
if [ -z "${OCR_ITA_MODELS:-}" ]; then
  if [ -n "${APPIMAGE:-}" ]; then
    BASE="$(dirname "$(readlink -f "$APPIMAGE")")"
  else
    BASE="$(dirname "$QUI")"
  fi
  for CANDIDATO in "$BASE/models" "$QUI/usr/lib/ocr-ita-desktop/models" \
                   "${XDG_DATA_HOME:-$HOME/.local/share}/ocr-ita-desktop/models"; do
    if [ -f "$CANDIDATO/glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf" ]; then
      OCR_ITA_MODELS="$CANDIDATO"
      export OCR_ITA_MODELS
      break
    fi
  done
fi
exec "$QUI/usr/bin/ocr-ita-desktop" "$@"
APPRUN
chmod 0755 "$APPDIR/AppRun"

echo "==> cartella portabile pronta: $APPDIR"
du -sh "$APPDIR"

APPIMAGETOOL="$RADICE/thirdparty/appimagetool"
if [ -x "$APPIMAGETOOL" ]; then
  echo "==> AppImage"
  # Il runtime predefinito di appimagetool vuole libfuse2, che su Ubuntu 26.04
  # non c'e' piu': si usa il runtime type2 moderno, che linka fuse3 staticamente.
  RUNTIME="$RADICE/thirdparty/runtime-x86_64"
  ARCH=x86_64 "$APPIMAGETOOL" --appimage-extract-and-run --no-appstream \
    ${RUNTIME:+--runtime-file "$RUNTIME"} "$APPDIR" "$DIST/$APPIMAGE_NOME"
  chmod +x "$DIST/$APPIMAGE_NOME"
  ls -lh "$DIST/$APPIMAGE_NOME"
  ( cd "$DIST" && sha256sum "$APPIMAGE_NOME" > "$APPIMAGE_NOME.sha256" )
  cat "$DIST/$APPIMAGE_NOME.sha256"
else
  echo "appimagetool assente: resta la cartella portabile" >&2
fi
