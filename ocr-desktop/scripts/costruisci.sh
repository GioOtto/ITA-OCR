#!/usr/bin/env bash
# Compila il motore llama.cpp (Vulkan + dispatch CPU) e l'applicazione.
source "$(dirname "${BASH_SOURCE[0]}")/comune.sh"

# CUDA si compila solo dove c'e' il toolkit: chiederlo su una macchina senza
# nvcc farebbe fallire tutta la build, e nvcc genera codice solo per il sistema
# su cui gira (niente cross-compilazione da Linux verso Windows).
CUDA=-DGGML_CUDA=OFF
ARCH_CUDA=()
if command -v nvcc >/dev/null 2>&1; then
  CUDA=-DGGML_CUDA=ON
  echo "==> nvcc trovato ($(nvcc --version | tail -1)): si compila anche il backend CUDA"

  # "-real" produce codice macchina gia' pronto per quella GPU, "-virtual" solo
  # PTX che il driver compila al primo avvio. Il PTX e' fragile: quello emesso
  # dal Toolkit 12.9 pretende un driver r575 o superiore, mentre il runtime
  # CUDA 12.x si accontenta di driver molto piu' vecchi. Su una macchina
  # aggiornata a meta' il caricamento muore con "the provided PTX was compiled
  # with an unsupported toolchain" e il motore ripiega sul backend successivo.
  # Il default di llama.cpp lascia virtuali proprio Pascal, Turing e Ampere
  # datacenter -- GTX 10xx, GTX 16xx, RTX 20xx: hardware tutt'altro che raro.
  versione=$(nvcc --version | sed -n 's/.*release \([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\1 \2/p')
  numero=$(( ${versione% *} * 100 + ${versione#* } ))
  archi="75-real;80-real;86-real;89-real;90-virtual"
  # Maxwell e' il minimo di CUDA 12; in CUDA 13 spariscono anche Pascal e Volta.
  [ "$numero" -lt 1300 ] && archi="50-virtual;61-real;70-virtual;$archi"
  # Blackwell: 12.8 conosce sm_120, 12.9 anche sm_121.
  [ "$numero" -ge 1208 ] && archi="$archi;120a-real"
  [ "$numero" -ge 1209 ] && archi="$archi;121a-real"
  echo "    architetture: $archi"
  ARCH_CUDA=(-DCMAKE_CUDA_ARCHITECTURES="$archi")
else
  echo "==> nvcc assente: si compilano solo Vulkan e CPU"
fi

echo "==> llama.cpp: Vulkan + varianti CPU caricate a runtime"
# GGML_BACKEND_DL + GGML_CPU_ALL_VARIANTS: un solo eseguibile che a runtime
# sceglie da solo fra x64 generico, SSE4.2, AVX, AVX2/FMA/F16C/BMI2, AVX-VNNI e
# le varianti AVX-512. Niente binari separati scelti a mano dall'utente.
cmake -S "$LLAMA_SORGENTE" -B "$BUILD_LLAMA" \
  -DCMAKE_BUILD_TYPE=Release \
  -DGGML_NATIVE=OFF \
  -DBUILD_SHARED_LIBS=ON \
  -DGGML_BACKEND_DL=ON \
  -DGGML_CPU_ALL_VARIANTS=ON \
  -DGGML_VULKAN=ON \
  -DGGML_HIP=OFF "$CUDA" ${ARCH_CUDA[@]+"${ARCH_CUDA[@]}"} \
  -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF \
  -DLLAMA_BUILD_TOOLS=ON -DLLAMA_BUILD_SERVER=ON -DLLAMA_CURL=OFF \
  -DLLAMA_OPENSSL=OFF \
  -DVulkan_INCLUDE_DIR="$VULKAN_HEADERS" \
  -DVulkan_LIBRARY="${VULKAN_LIBRARY:-/usr/lib/x86_64-linux-gnu/libvulkan.so.1}" \
  ${SPIRV_HEADERS_DIR:+-DSPIRV-Headers_DIR="$SPIRV_HEADERS_DIR"} \
  ${SPIRV_HEADERS_INCLUDE:+-DCMAKE_CXX_FLAGS="-isystem $SPIRV_HEADERS_INCLUDE"} \
  ${GLSLC:+-DVulkan_GLSLC_EXECUTABLE="$GLSLC"}
cmake --build "$BUILD_LLAMA" -j "$(nproc)"

echo "==> applicazione (Tauri + Rust)"
cd "$APP"
cargo build --release
echo "==> fatto: $APP/target/release/$NOME_APP"
