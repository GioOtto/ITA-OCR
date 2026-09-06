<#
.SYNOPSIS
  Compila ITA-OCR per Windows: llama.cpp con CUDA e Vulkan, poi l'applicazione.

.DESCRIPTION
  Da Linux un .exe non si puo' produrre: Tauri non cross-compila verso Windows
  (servono WebView2 e la toolchain MSVC) e nvcc genera codice solo per il
  sistema su cui gira. Questo script va quindi eseguito **sulla macchina
  Windows**, che e' la stessa strada che segue la CI.

  Tutto quello che passa a schermo finisce anche in un file di log unico, con i
  comandi e i codici di uscita: se qualcosa si rompe, basta mandare quel file.

.PARAMETER Llama
  Sorgenti di llama.cpp. Se la cartella non esiste viene clonata al commit
  verificato (4df29be).

.PARAMETER SaltaCuda
  Compila solo Vulkan e CPU, senza CUDA.

.EXAMPLE
  .\costruisci-windows.ps1
  .\costruisci-windows.ps1 -Llama .\llama.cpp -SaltaCuda
#>
[CmdletBinding()]
param(
  [string]$Llama = "",
  [switch]$SaltaCuda
)

# Ogni errore ferma lo script: meglio fallire subito e forte che produrre un
# pacchetto a meta' che poi non parte per un motivo oscuro.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$COMMIT_LLAMA = "4df29be"
$Radice   = Split-Path -Parent $PSScriptRoot
$App      = Join-Path $Radice "app\src-tauri"
$Dist     = Join-Path $Radice "dist"
$Uscita   = Join-Path $Dist   "ITA-OCR-windows"
$Risorse  = Join-Path $Uscita "llama"
$BuildDir = Join-Path $Radice "build\llama-windows"
if (-not $Llama) { $Llama = Join-Path (Split-Path -Parent $Radice) "ocr-ita\vendor\llama.cpp" }

New-Item -ItemType Directory -Force -Path $Dist | Out-Null
$Log = Join-Path $Dist ("costruzione-windows-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".log")
Start-Transcript -Path $Log -Append | Out-Null

function Titolo($testo) {
  Write-Host ""
  Write-Host "==> $testo" -ForegroundColor Cyan
}

function Nota($testo) { Write-Host "    $testo" -ForegroundColor DarkGray }

# Esegue un comando esterno mostrandolo, e si ferma col suo codice se fallisce:
# senza questo PowerShell tira dritto dopo il fallimento di un .exe.
function Esegui($descrizione, $comando, [string[]]$argomenti) {
  Nota "$comando $($argomenti -join ' ')"
  & $comando @argomenti
  if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "FALLITO: $descrizione (codice $LASTEXITCODE)" -ForegroundColor Red
    Write-Host "Log completo: $Log" -ForegroundColor Yellow
    Stop-Transcript | Out-Null
    exit $LASTEXITCODE
  }
}

function Richiedi($nome, $comando, $dove) {
  $trovato = Get-Command $comando -ErrorAction SilentlyContinue
  if (-not $trovato) {
    Write-Host "MANCA: $nome (comando '$comando' non nel PATH)" -ForegroundColor Red
    Write-Host "       $dove" -ForegroundColor Yellow
    Stop-Transcript | Out-Null
    exit 2
  }
  Nota "$nome : $($trovato.Source)"
}

try {
  Titolo "ambiente"
  Nota "log di questa esecuzione: $Log"
  Nota "sistema: $([Environment]::OSVersion.VersionString)"
  Nota "radice progetto: $Radice"

  Richiedi "CMake"  "cmake" "https://cmake.org/download/ (spuntare 'Add to PATH')"
  Richiedi "Rust"   "cargo" "https://rustup.rs/ e poi: rustup default stable-msvc"
  Richiedi "Git"    "git"   "https://git-scm.com/download/win"

  # Il compilatore MSVC non sta nel PATH finche' non si apre un prompt di
  # Visual Studio: qui si controlla e si spiega, invece di fallire dentro cmake.
  if (-not (Get-Command "cl.exe" -ErrorAction SilentlyContinue)) {
    Write-Host "MANCA: il compilatore MSVC (cl.exe) non e' nel PATH." -ForegroundColor Red
    Write-Host "       Installa 'Visual Studio Build Tools' con il carico di lavoro" -ForegroundColor Yellow
    Write-Host "       'Sviluppo di applicazioni desktop con C++', poi rilancia questo" -ForegroundColor Yellow
    Write-Host "       script da 'x64 Native Tools Command Prompt for VS'." -ForegroundColor Yellow
    Stop-Transcript | Out-Null
    exit 2
  }
  Nota "MSVC : $((Get-Command cl.exe).Source)"

  $ConCuda = $false
  $ArchCuda = @()
  if (-not $SaltaCuda) {
    if (Get-Command "nvcc" -ErrorAction SilentlyContinue) {
      $ConCuda = $true
      Nota "CUDA : $((nvcc --version | Select-Object -Last 1))"

      # Architetture da compilare. "-real" produce codice macchina gia' pronto
      # per quella GPU; "-virtual" produce solo PTX, che il driver deve
      # compilare al primo avvio. Il PTX e' comodo ma fragile: quello emesso
      # dal Toolkit 12.9 pretende un driver r575 o superiore, mentre il runtime
      # CUDA 12.x si accontenta di driver molto piu' vecchi. Su una macchina
      # aggiornata a meta' il caricamento muore con "the provided PTX was
      # compiled with an unsupported toolchain" e il motore ripiega su Vulkan.
      # Il default di llama.cpp lascia virtuali proprio Pascal, Turing e Ampere
      # datacenter -- cioe' GTX 10xx, GTX 16xx e RTX 20xx, hardware diffuso.
      # Compilarle reali costa una DLL piu' grossa, ma toglie di mezzo il JIT
      # e con esso la dipendenza dalla versione del driver.
      $versione = [version]"12.0"
      $riga = (nvcc --version | Select-String -Pattern "release (\d+\.\d+)" | Select-Object -First 1)
      if ($riga) { $versione = [version]$riga.Matches[0].Groups[1].Value }
      $ArchCuda = @("61-real", "70-virtual", "75-real", "80-real", "86-real", "89-real", "90-virtual")
      if ($versione -lt [version]"13.0") {
        # Maxwell e' il minimo di CUDA 12; in CUDA 13 spariscono anche Pascal e Volta.
        $ArchCuda = @("50-virtual") + $ArchCuda
      } else {
        $ArchCuda = $ArchCuda | Where-Object { $_ -notlike "61-*" -and $_ -notlike "70-*" }
      }
      # Blackwell: 12.8 conosce sm_120, 12.9 anche sm_121.
      if ($versione -ge [version]"12.8") { $ArchCuda += "120a-real" }
      if ($versione -ge [version]"12.9") { $ArchCuda += "121a-real" }
      Nota "archit.: $($ArchCuda -join ' ')"
    } else {
      Write-Host "    nvcc non trovato: si compila senza CUDA." -ForegroundColor Yellow
      Write-Host "    Per il backend NVIDIA installa il CUDA Toolkit:" -ForegroundColor Yellow
      Write-Host "    https://developer.nvidia.com/cuda-downloads" -ForegroundColor Yellow
    }
  }

  # ---------------------------------------------------------------- sorgenti
  Titolo "sorgenti di llama.cpp"
  if (-not (Test-Path $Llama)) {
    Nota "non trovata in $Llama : la clono"
    Esegui "clone di llama.cpp" "git" @("clone", "https://github.com/ggml-org/llama.cpp.git", $Llama)
    Push-Location $Llama
    Esegui "checkout del commit verificato" "git" @("checkout", $COMMIT_LLAMA)
    Pop-Location
  } else {
    Nota "uso $Llama"
  }

  # ------------------------------------------------------------------ motore
  Titolo "llama.cpp: Vulkan + varianti CPU, CUDA se disponibile"
  $argomentiCmake = @(
    "-S", $Llama, "-B", $BuildDir,
    "-DCMAKE_BUILD_TYPE=Release",
    "-DGGML_NATIVE=OFF",
    "-DBUILD_SHARED_LIBS=ON",
    "-DGGML_BACKEND_DL=ON",
    "-DGGML_CPU_ALL_VARIANTS=ON",
    "-DGGML_VULKAN=ON",
    "-DGGML_HIP=OFF",
    "-DLLAMA_BUILD_TESTS=OFF",
    "-DLLAMA_BUILD_EXAMPLES=OFF",
    "-DLLAMA_BUILD_TOOLS=ON",
    "-DLLAMA_BUILD_SERVER=ON",
    "-DLLAMA_CURL=OFF",
    # LLAMA_OPENSSL e' ON di default e serve solo per scaricare modelli via
    # HTTPS: noi parliamo in chiaro con 127.0.0.1 e i modelli sono gia' su
    # disco. Lasciarlo acceso e' peggio che inutile, perche' CMake lo trova
    # nel primo OpenSSL che incontra nel PATH -- ad esempio quello dell'env
    # conda usato per cmake e ninja -- e llama-common finisce per importare
    # libssl-3-x64.dll, che sulla macchina di destinazione non esiste.
    "-DLLAMA_OPENSSL=OFF"
  )
  $argomentiCmake += if ($ConCuda) { "-DGGML_CUDA=ON" } else { "-DGGML_CUDA=OFF" }
  if ($ConCuda) { $argomentiCmake += "-DCMAKE_CUDA_ARCHITECTURES=$($ArchCuda -join ';')" }
  Esegui "configurazione di llama.cpp" "cmake" $argomentiCmake
  Esegui "compilazione di llama.cpp" "cmake" @("--build", $BuildDir, "--config", "Release", "-j", "$env:NUMBER_OF_PROCESSORS")

  # --------------------------------------------------------------- pacchetto
  Titolo "cartella portabile"
  Remove-Item -Recurse -Force $Uscita -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force -Path $Risorse | Out-Null

  $binLlama = Join-Path $BuildDir "bin\Release"
  if (-not (Test-Path $binLlama)) { $binLlama = Join-Path $BuildDir "bin" }
  Nota "binari del motore da $binLlama"
  Copy-Item (Join-Path $binLlama "llama-server.exe") $Risorse
  # ggml carica i backend a runtime dalla directory dell'eseguibile: servono
  # tutte le DLL, comprese le varianti CPU e i backend GPU.
  Get-ChildItem -Path $binLlama -Filter "*.dll" | ForEach-Object {
    Copy-Item $_.FullName $Risorse
  }
  $dllCuda = Get-ChildItem -Path $Risorse -Filter "ggml-cuda*.dll" -ErrorAction SilentlyContinue
  if ($dllCuda) {
    Nota "incluso il backend CUDA"
    # ggml-cuda.dll importa cudart e cuBLAS, che stanno nel Toolkit e non nel
    # driver: senza, sulla macchina di destinazione la DLL non si carica e il
    # motore ripiega su Vulkan **in silenzio**, facendo sembrare CUDA rotto
    # quando manca solo un file. cublasLt e' preteso da cublas, non da noi.
    # L'Attachment A dell'EULA CUDA li elenca fra i redistribuibili.
    $binCuda = if ($env:CUDA_PATH) { Join-Path $env:CUDA_PATH "bin" } else { "" }
    if ($binCuda -and (Test-Path $binCuda)) {
      $totale = 0
      foreach ($schema in @("cudart64_*.dll", "cublas64_*.dll", "cublasLt64_*.dll")) {
        $trovate = Get-ChildItem -Path $binCuda -Filter $schema -ErrorAction SilentlyContinue
        if (-not $trovate) {
          Write-Host "MANCA: $schema in $binCuda" -ForegroundColor Red
          Write-Host "       il pacchetto avrebbe CUDA solo in apparenza." -ForegroundColor Yellow
          Stop-Transcript | Out-Null
          exit 2
        }
        foreach ($f in $trovate) {
          Copy-Item $f.FullName $Risorse
          $totale += $f.Length
        }
      }
      Nota ("runtime CUDA incluso: {0:N0} MB" -f ($totale / 1MB))
    } else {
      Write-Host "MANCA: CUDA_PATH non impostata, runtime CUDA non copiabile." -ForegroundColor Red
      Write-Host "       Rilancia da un prompt aperto dopo l'installazione del Toolkit." -ForegroundColor Yellow
      Stop-Transcript | Out-Null
      exit 2
    }
  } else {
    Nota "nessun backend CUDA nel pacchetto"
  }

  # --------------------------------------------------- runtime C++ di MSVC
  # Tutto quello che compiliamo importa msvcp140 e vcruntime140, che sul PC di
  # destinazione ci sono solo se qualcuno ha installato il "Visual C++
  # Redistributable". Chiedere quell'installazione contraddirebbe una cartella
  # che si copia e si avvia, quindi le DLL viaggiano accanto agli eseguibili:
  # e' il deployment "app-local" previsto da Microsoft, senza UAC.
  # Servono in due posti perche' il caricatore cerca nella directory
  # dell'eseguibile: accanto a ocr-ita-desktop.exe e accanto a llama-server.exe.
  Titolo "runtime C++ di MSVC"
  $crt = Get-ChildItem (Join-Path $env:VCToolsRedistDir "x64\Microsoft.VC*.CRT") -Directory -ErrorAction SilentlyContinue |
    Select-Object -Last 1
  if (-not $crt) {
    Write-Host "MANCA: il redistributable MSVC non e' nella cartella VC\Redist." -ForegroundColor Red
    Write-Host "       Senza, l'app parte solo dove il VC++ Redistributable e' gia' installato." -ForegroundColor Yellow
    Stop-Transcript | Out-Null
    exit 2
  }
  # Solo la chiusura effettiva delle dipendenze: msvcp140 tira vcruntime140 e
  # vcruntime140_1, e li' finisce. Gli altri file della cartella redist
  # (concrt140, vccorlib140, msvcp140_*) non sono importati da niente di
  # nostro e spedirli sarebbe rumore.
  foreach ($dll in @("msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll")) {
    $da = Join-Path $crt.FullName $dll
    if (-not (Test-Path $da)) {
      Write-Host "MANCA: $dll in $($crt.FullName)" -ForegroundColor Red
      Stop-Transcript | Out-Null
      exit 2
    }
    Copy-Item $da $Uscita
    Copy-Item $da $Risorse
  }
  Nota "runtime C++ accanto all'app e al motore (da $($crt.Name))"

  # PDFium: si scarica il binario ufficiale, non si compila.
  Titolo "PDFium"
  $zipPdfium = Join-Path $Dist "pdfium-win.tgz"
  if (-not (Test-Path (Join-Path $Uscita "pdfium.dll"))) {
    $urlPdfium = "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz"
    Nota "scarico $urlPdfium"
    Invoke-WebRequest -Uri $urlPdfium -OutFile $zipPdfium
    $tempPdfium = Join-Path $Dist "pdfium-tmp"
    New-Item -ItemType Directory -Force -Path $tempPdfium | Out-Null
    Esegui "estrazione di PDFium" "tar" @("-xzf", $zipPdfium, "-C", $tempPdfium)
    Copy-Item (Join-Path $tempPdfium "bin\pdfium.dll") $Uscita
    Remove-Item -Recurse -Force $tempPdfium, $zipPdfium
  }
  Nota "pdfium.dll pronta"

  # ------------------------------------------------------- post-correzione
  # Senza questi la correzione lessicale non parte: l'app li cerca accanto a
  # se stessa, in dictionaries\it_IT.
  Titolo "dizionari della post-correzione"
  $dizionari = Join-Path $Uscita "dictionaries\it_IT"
  New-Item -ItemType Directory -Force -Path $dizionari | Out-Null
  $daDizionari = Join-Path $Radice "resources\dictionaries\it_IT"
  if (-not (Test-Path $daDizionari)) {
    Write-Host "MANCA: $daDizionari" -ForegroundColor Red
    Stop-Transcript | Out-Null
    exit 2
  }
  foreach ($nome in @("it_IT.aff", "it_IT.dic", "COPYING", "README_it_IT.txt", "LICENSE-GPL-3.txt")) {
    Copy-Item -LiteralPath (Join-Path $daDizionari $nome) -Destination $dizionari
  }
  Nota "italiano: copiati $((Get-ChildItem $dizionari).Count) file"

  # L'inglese serve solo a riconoscere le parole da non correggere. Se manca
  # non si ferma la build: l'app funziona lo stesso, perde solo quella
  # protezione, e vale la pena dirlo invece di far fallire tutto.
  $daInglese = Join-Path $Radice "resources\dictionaries\en_US"
  if (Test-Path $daInglese) {
    $inglese = Join-Path $Uscita "dictionaries\en_US"
    New-Item -ItemType Directory -Force -Path $inglese | Out-Null
    Copy-Item (Join-Path $daInglese "*") $inglese
    Nota "inglese: copiati $((Get-ChildItem $inglese).Count) file"
  } else {
    Write-Host "    dizionario inglese assente in $daInglese :" -ForegroundColor Yellow
    Write-Host "    le parole inglesi non saranno protette dalla post-correzione." -ForegroundColor Yellow
  }

  # ------------------------------------------------------------ applicazione
  Titolo "applicazione (Tauri + Rust)"
  Push-Location $App
  Esegui "compilazione dell'applicazione" "cargo" @("build", "--release")
  Pop-Location
  Copy-Item (Join-Path $App "target\release\ocr-ita-desktop.exe") $Uscita
  & (Join-Path $PSScriptRoot "aggiorna-icone-windows.ps1") -Percorsi @(
    (Join-Path $Uscita "ocr-ita-desktop.exe")
  )

  # Licenze incluse anche nel pacchetto portabile.
  $repo = Split-Path -Parent $Radice
  Copy-Item -LiteralPath (Join-Path $repo "LICENSE"), (Join-Path $repo "TERZE-PARTI.md") -Destination $Uscita
  Copy-Item -LiteralPath (Join-Path $repo "licenses") -Destination $Uscita -Recurse

  # ---------------------------------------------------------------- modelli
  Titolo "modelli"
  $modelli = Join-Path $Uscita "models"
  New-Item -ItemType Directory -Force -Path $modelli | Out-Null
  $sorgenteModelli = Join-Path $Dist "models"
  $gguf = @(
    "glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf",
    "glm-ocr-base-mmproj-q8_0.gguf"
  )
  foreach ($m in $gguf) {
    $da = Join-Path $sorgenteModelli $m
    if (Test-Path $da) {
      Nota "copio $m (e' grosso, ci mette un po')"
      Copy-Item $da $modelli
    } else {
      Write-Host "    manca $m : copialo a mano in $modelli" -ForegroundColor Yellow
    }
  }

  # ------------------------------------------------------------- avviatore
  # L'app trova le risorse accanto all'eseguibile, ma un .cmd esplicito rende
  # ovvio dove sono e permette di spostare i modelli altrove.
  $avvio = @"
@echo off
rem Avvia ITA-OCR dalla cartella portabile.
set "QUI=%~dp0"
set "OCR_ITA_RESOURCES=%QUI%"
if not defined OCR_ITA_MODELS set "OCR_ITA_MODELS=%QUI%models"
start "" "%QUI%ocr-ita-desktop.exe" %*
"@
  Set-Content -Path (Join-Path $Uscita "ITA-OCR.cmd") -Value $avvio -Encoding ASCII

  # ------------------------------------------- verifica delle dipendenze
  # Il controllo che mancava: la macchina che costruisce ha nel PATH cose che
  # quella di destinazione non ha (l'env conda di cmake e ninja, il CUDA
  # Toolkit, il Vulkan SDK). Un binario puo' quindi importare una DLL che qui
  # si risolve e altrove no, e il guasto si vede solo sull'altro computer,
  # come una finestra "impossibile proseguire l'esecuzione del codice".
  # Si ispeziona ogni binario e si considera risolta una importazione solo se
  # la DLL sta accanto al binario, nella radice del pacchetto, o in System32.
  Titolo "verifica delle dipendenze"
  $binari = Get-ChildItem -LiteralPath $Uscita -Recurse -Include *.exe, *.dll
  $tmpDip = Join-Path $env:TEMP "ocr-ita-dipendenze.txt"
  $mancanti = @{}
  foreach ($b in $binari) {
    cmd /c "dumpbin /dependents `"$($b.FullName)`" > `"$tmpDip`" 2>&1"
    $imp = Get-Content $tmpDip | Select-String -Pattern "^\s+\S+\.dll$" |
      ForEach-Object { $_.ToString().Trim() }
    foreach ($i in $imp) {
      # Le api-ms-win-* sono l'Universal CRT, componente di Windows 10/11.
      # nvcuda.dll arriva col driver NVIDIA: assente qui per costruzione, e
      # presente su ogni macchina che abbia davvero una GPU NVIDIA.
      if ($i -like "api-ms-*" -or $i -eq "nvcuda.dll") { continue }
      $ok = (Test-Path -LiteralPath (Join-Path $b.DirectoryName $i)) -or
            (Test-Path -LiteralPath (Join-Path $Uscita $i)) -or
            (Test-Path -LiteralPath (Join-Path "$env:SystemRoot\System32" $i))
      if (-not $ok) {
        if (-not $mancanti.ContainsKey($i)) { $mancanti[$i] = @() }
        $mancanti[$i] += $b.Name
      }
    }
  }
  if ($mancanti.Count -gt 0) {
    Write-Host "DIPENDENZE MANCANTI: il pacchetto non partirebbe altrove." -ForegroundColor Red
    foreach ($k in ($mancanti.Keys | Sort-Object)) {
      Write-Host ("  {0}  <- {1}" -f $k, ($mancanti[$k] -join ", ")) -ForegroundColor Yellow
      $dove = (Get-Command $k -ErrorAction SilentlyContinue).Source
      if ($dove) { Write-Host "      qui si risolve in: $dove" -ForegroundColor Yellow }
    }
    Stop-Transcript | Out-Null
    exit 2
  }
  Nota "$($binari.Count) binari, nessuna dipendenza esterna al pacchetto"

  Titolo "fatto"
  Write-Host "Cartella portabile: $Uscita" -ForegroundColor Green
  Write-Host "Avvia con ITA-OCR.cmd" -ForegroundColor Green
  Write-Host "Backend CUDA: $(if ($dllCuda) { 'incluso' } else { 'assente' })" -ForegroundColor Green
  Write-Host "Log: $Log" -ForegroundColor Green
}
catch {
  Write-Host ""
  Write-Host "ERRORE NON PREVISTO" -ForegroundColor Red
  Write-Host $_.Exception.Message -ForegroundColor Red
  Write-Host $_.ScriptStackTrace -ForegroundColor DarkGray
  Write-Host "Log completo da mandare: $Log" -ForegroundColor Yellow
  Stop-Transcript | Out-Null
  exit 1
}
Stop-Transcript | Out-Null
