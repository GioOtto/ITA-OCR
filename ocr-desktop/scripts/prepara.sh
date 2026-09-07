#!/usr/bin/env bash
# Procura le dipendenze binarie che la build Linux si aspetta in thirdparty/.
#
# Su Windows costruisci-windows.ps1 si scarica da solo pdfium e quello che gli
# serve; su Linux quelle stesse cose esistevano solo sul disco di chi le aveva
# prese a mano. Chi clonava il repo compilava il motore e l'applicazione, poi
# impacchetta.sh si fermava su libpdfium.so. Questo script chiude l'asimmetria.
#
# Si puo' rilanciare quante volte si vuole: quello che c'e' gia' non si
# riscarica. Non chiede root e non installa niente nel sistema.
source "$(dirname "${BASH_SOURCE[0]}")/comune.sh"

TP="$RADICE/thirdparty"
mkdir -p "$TP"
TEMP="$(mktemp -d)"
trap 'rm -rf "$TEMP"' EXIT

PDFIUM_TAG="chromium/8021"
PDFIUM_SHA256="685f7930cd184ea22cd77afe707c1cf53b173d18118b6e16cb213c9277d7cdc3"
APPIMAGETOOL_TAG="1.9.1"
APPIMAGETOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
# type2-runtime pubblica solo una release continuous: il digest impedisce che
# un aggiornamento del tag entri silenziosamente in una build.
RUNTIME_COMMIT="75849dce7cc37e4319b633df1f116ca895c71a12"
RUNTIME_SHA256="1cc49bcf1e2ccd593c379adb17c9f85a36d619088296504de95b1d06215aebbf"

verifica_sha256() {
  local file="$1" atteso="$2"
  printf '%s  %s\n' "$atteso" "$file" | sha256sum --check --status
}

scarica_verificato() {
  local nome="$1" url="$2" destinazione="$3" sha="$4" modo="$5"
  if [ -f "$destinazione" ] && verifica_sha256 "$destinazione" "$sha"; then
    echo "==> $nome: gia' presente e verificato"
    return
  fi
  echo "==> $nome: scarico e verifico SHA-256"
  local temporaneo="$TEMP/${nome// /-}.download"
  curl -fsSL -o "$temporaneo" "$url"
  verifica_sha256 "$temporaneo" "$sha" || {
    echo "$nome: SHA-256 diverso da quello atteso" >&2
    exit 1
  }
  install -m "$modo" "$temporaneo" "$destinazione"
}

# Scarica i .deb di una lista di pacchetti e li estrae in una directory, senza
# passare da apt-get install: --print-uris chiede solo la chiusura delle
# dipendenze, il download e dpkg -x non vogliono privilegi.
estrai_deb() {
  local destinazione="$1"; shift
  apt-get install --print-uris -y "$@" 2>/dev/null \
    | sed -n "s/^'\([^']*\)'.*/\1/p" > "$TEMP/uri.txt"
  local n; n=$(wc -l < "$TEMP/uri.txt")
  [ "$n" -gt 0 ] || { echo "apt non ha prodotto nessun URI per $*: sorgenti non configurate?" >&2; return 1; }
  echo "    $n pacchetti"
  rm -rf "$TEMP/debs"; mkdir -p "$TEMP/debs" "$destinazione"
  (cd "$TEMP/debs" && xargs -n1 -P4 curl -fsSLO < "$TEMP/uri.txt")
  for d in "$TEMP/debs"/*.deb; do dpkg -x "$d" "$destinazione"; done
}

# ------------------------------------------------------------------- pdfium
# Obbligatorio: impacchetta.sh installa libpdfium.so dentro l'AppDir e senza
# quella l'applicazione non apre nessun PDF.
if [ -f "$TP/pdfium/lib/libpdfium.so" ] \
    && [ "$(cat "$TP/pdfium/.ita-ocr-archive.sha256" 2>/dev/null || true)" = "$PDFIUM_SHA256" ]; then
  echo "==> pdfium $PDFIUM_TAG: gia' presente e verificata"
else
  echo "==> pdfium $PDFIUM_TAG: scarico e verifico SHA-256"
  curl -fsSL -o "$TEMP/pdfium.tgz" \
    "https://github.com/bblanchon/pdfium-binaries/releases/download/$PDFIUM_TAG/pdfium-linux-x64.tgz"
  verifica_sha256 "$TEMP/pdfium.tgz" "$PDFIUM_SHA256" || {
    echo "pdfium: SHA-256 diverso da quello atteso" >&2
    exit 1
  }
  mkdir -p "$TEMP/pdfium"
  tar xzf "$TEMP/pdfium.tgz" -C "$TEMP/pdfium"
  [ -f "$TEMP/pdfium/lib/libpdfium.so" ] || { echo "l'archivio non conteneva lib/libpdfium.so" >&2; exit 1; }
  rm -rf "$TP/pdfium"
  mv "$TEMP/pdfium" "$TP/pdfium"
  printf '%s\n' "$PDFIUM_SHA256" > "$TP/pdfium/.ita-ocr-archive.sha256"
  echo "    versione $(sed -n 's/^BUILD=//p' "$TP/pdfium/VERSION" 2>/dev/null || echo ignota)"
fi

# ------------------------------------------------------- appimagetool e runtime
# Facoltativi: senza, impacchetta.sh produce la cartella portabile e salta
# l'AppImage. Il runtime predefinito di appimagetool vuole libfuse2, che su
# Ubuntu 26.04 non c'e' piu': serve quello type2, che linka fuse3 staticamente.
scarica_verificato "appimagetool $APPIMAGETOOL_TAG" \
  "https://github.com/AppImage/appimagetool/releases/download/$APPIMAGETOOL_TAG/appimagetool-x86_64.AppImage" \
  "$TP/appimagetool" "$APPIMAGETOOL_SHA256" 0755
scarica_verificato "runtime type2 $RUNTIME_COMMIT" \
  "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64" \
  "$TP/runtime-x86_64" "$RUNTIME_SHA256" 0755

# ------------------------------------------------------------ toolchain Vulkan
# ggml-vulkan compila gli shader durante la build: gli servono glslc e le
# intestazioni SPIR-V, e le cerca con find_package(SPIRV-Headers), cioe' vuole
# il file di configurazione CMake, non solo gli header. Il Vulkan SDK completo
# di LunarG li porta entrambi ma pesa quasi un giga: qui bastano due pacchetti.
if [ -x "${GLSLC:-}" ] || command -v glslc >/dev/null 2>&1; then
  echo "==> glslc: gia' disponibile"
elif [ -x "$TP/vulkan/usr/bin/glslc" ]; then
  echo "==> glslc: gia' presente in thirdparty/vulkan"
elif ! command -v apt-get >/dev/null 2>&1; then
  echo "manca glslc e questa non e' una macchina Debian/Ubuntu." >&2
  echo "installa l'equivalente di: glslc spirv-headers" >&2
  exit 1
else
  echo "==> glslc e intestazioni SPIR-V: li estraggo dai .deb senza installarli"
  estrai_deb "$TP/vulkan" glslc spirv-headers
  [ -x "$TP/vulkan/usr/bin/glslc" ] || { echo "i .deb non contenevano usr/bin/glslc" >&2; exit 1; }
  # glslc linka libshaderc_shared.so, che sta nello stesso albero e non nel
  # sistema: senza questo l'eseguibile non parte e cmake lo dichiara rotto.
  patchelf --set-rpath '$ORIGIN/../lib/x86_64-linux-gnu' "$TP/vulkan/usr/bin/glslc" 2>/dev/null || true
  echo "    $("$TP/vulkan/usr/bin/glslc" --version 2>&1 | head -1)"
fi

# ------------------------------------------------------------------ sysroot
# Tauri 2 compila contro WebKitGTK, GTK3 e libsoup3. La via normale e'
# installarne i pacchetti -dev di sistema; se non ci sono e non si vuole (o non
# si puo') toccare la macchina, si estraggono i .deb in un sysroot locale, che
# comune.sh mette poi in PKG_CONFIG_PATH.
SERVONO=(webkit2gtk-4.1 javascriptcoregtk-4.1 libsoup-3.0 gtk+-3.0)
PACCHETTI=(libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev)

mancanti=()
for p in "${SERVONO[@]}"; do
  pkg-config --exists "$p" 2>/dev/null || mancanti+=("$p")
done

if [ ${#mancanti[@]} -eq 0 ]; then
  echo "==> librerie di sistema: gia' a posto (${SERVONO[*]})"
elif [ -d "$SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig" ]; then
  echo "==> sysroot: gia' presente in thirdparty/sysroot"
elif ! command -v apt-get >/dev/null 2>&1; then
  echo "mancano ${mancanti[*]} e questa non e' una macchina Debian/Ubuntu." >&2
  echo "installa gli equivalenti di: ${PACCHETTI[*]}" >&2
  exit 1
else
  echo "==> sysroot: mancano ${mancanti[*]}, li estraggo dai .deb senza installarli"
  estrai_deb "$SYSROOT" "${PACCHETTI[@]}"

  # I .pc dichiarano prefix=/usr, che punterebbe al sistema invece che qui.
  # Vanno riscritti sul sysroot, altrimenti pkg-config trova le descrizioni e
  # il compilatore non trova le intestazioni.
  find "$SYSROOT/usr" -name '*.pc' -print0 \
    | xargs -0 sed -i -E "s|^([a-z_]+)=/usr|\1=$SYSROOT/usr|"
  echo "    sysroot pronto: $SYSROOT"
fi

echo "==> dipendenze a posto: ora scripts/costruisci.sh e scripts/impacchetta.sh"
