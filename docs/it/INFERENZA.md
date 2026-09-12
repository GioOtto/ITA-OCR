# Prestazioni dell'inferenza

Il motore usa profili di thread distinti: al massimo 8 su CPU e 4 con
offload GPU (CUDA o Vulkan). Il numero si riduce sulle macchine con meno
thread disponibili, riservandone due quando possibile. Sono impostazioni
prudenti per questo modello Q8, non un autotuning dell'hardware.

Il primo tentativo di ogni pagina svuota il contesto precedente. Nei retry
della stessa pagina `cache_prompt=true` riusa la KV della slot attiva:
immagine e prompt non vengono valutati di nuovo. Restano una sola slot e
`--cache-ram 0`: non viene abilitata la cache aggiuntiva da 8 GiB in RAM.
La selezione della GPU vale anche per la torre visiva; in CPU anche la torre
visiva resta sulla CPU.

Canvas 960×1248, pesi Q8, contesto 8192, limite 4096 token, stop ufficiali,
penalità e soglie della cascata restano quelli del protocollo precedente.
Il conteggio del prompt include anche i token riusati, così la diagnostica
non mostra erroneamente un prompt di un solo token nei retry.

## Regolazione locale

`OCR_ITA_THREADS` imposta i thread di generazione; `OCR_ITA_THREADS_BATCH`
imposta quelli per prompt e batch; `OCR_ITA_THREADS_VISION` imposta quelli
della torre visiva. Senza override, batch e visione usano lo stesso numero
della generazione. Sono ammessi interi da 1 al numero
di thread disponibili al processo; valori non validi vengono segnalati nel
log e sostituiti dal profilo predefinito. Le variabili si leggono all'avvio
del motore. I tre valori compaiono nella diagnostica e nella modalità `--prova`.

I thread visivi indipendenti richiedono la ricompilazione di llama-server:
gli script Windows, Linux e la CI Windows applicano automaticamente
`ocr-desktop/patches/llama-inferenza.patch` al commit fissato del motore.
La patch è idempotente e la build si ferma se i sorgenti sono incompatibili.
I vecchi binari non interpretano `MTMD_N_THREADS`, la variabile inoltrata
dall'app, e continuano a usare i thread della generazione per la visione.

Esempio PowerShell, dalla radice del repository:

```powershell
$env:OCR_ITA_THREADS = '8'
$env:OCR_ITA_THREADS_BATCH = '8'
$env:OCR_ITA_THREADS_VISION = '8'
$env:OCR_ITA_RESOURCES = (Resolve-Path 'ocr-desktop/dist/ITA-OCR-windows').Path
$env:OCR_ITA_DATA = Join-Path (Get-Location) 'ocr-desktop/dist/prova-prestazioni'
& 'ocr-desktop/app/src-tauri/target/release/ocr-ita-desktop.exe' --prova 'pagina.png' --backend cpu
```

`OCR_ITA_DATA` isola log e pidfile dalle sessioni aperte dell'applicazione.
Per tornare al profilo automatico, rimuovere gli override prima di riavviare
il motore:

```powershell
Remove-Item Env:OCR_ITA_THREADS, Env:OCR_ITA_THREADS_BATCH, Env:OCR_ITA_THREADS_VISION -ErrorAction SilentlyContinue
```

## Benchmark ripetibile

Lo script `ocr-desktop/scripts/benchmark-inferenza.py` richiede Pillow e
confronta più configurazioni, avviando un server alla volta su una porta
locale temporanea. Esegue un riscaldamento escluso dalle misure e salva
tempi, conteggi della cache, hash del testo e comando effettivo in JSON.
Non salva il testo della pagina nel risultato.

```powershell
python ocr-desktop/scripts/benchmark-inferenza.py pagina.png --server ocr-desktop/dist/ITA-OCR-windows/llama/llama-server.exe --modello ocr-desktop/dist/models/glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf --mmproj ocr-desktop/dist/models/glm-ocr-base-mmproj-q8_0.gguf --device none --thread 22 8 --ripetizioni 3 --retry --output ocr-desktop/dist/benchmark-inferenza/cpu.json
```

Per Vulkan usare `--device Vulkan0 --thread 22 4`; per CUDA scegliere il
nome restituito da `llama-server --list-devices`, ad esempio `CUDA0`.
Adattare i numeri di thread alla macchina. `--thread-batch N` permette un
confronto separato del batch. `--token` vale 128 per un confronto breve;
usare `--token 4096` per estenderlo fino al limite dell'applicazione.

`--stream` misura il tempo al primo token e gli intervalli fra gli eventi
SSE (mediana, p95 e massimo). `--thread-vision N`, `--flash-attn auto|on|off`,
`--batch N`, `--ubatch N` e `--senza-cache` servono per le ablazioni.
`--affinita MASK` limita soltanto il server figlio: la maschera deve essere
scelta conoscendo la topologia locale, non copiata da un'altra macchina.
Con il motore ricompilato, il JSON raccoglie anche `vision_encode_ms` e
`image_prefill_ms` dal log; `null` indica che quella fase non è stata
osservata (per esempio in un retry con cache o con un vecchio motore).

La patch misura il tempo host delle chiamate alla torre visiva e al prefill
dell'immagine. Su GPU le operazioni possono essere asincrone: questi numeri
non sono misure dei singoli kernel e non vanno sommati per ricostruire il
tempo totale. `timings.prompt_ms` resta il tempo complessivo del prompt,
visione inclusa. La latenza al primo token comprende anche preparazione
server, HTTP e tokenizzazione.

Con `--retry`, lo script confronta lo stadio `discord_f35_p20` senza cache
con lo stesso stadio dopo un greedy interrotto chiudendo lo streaming a
32 token. Aspetta il rilascio della slot prima del retry. Il tempo del
greedy interrotto è escluso dal tempo del retry; questo test non misura
l'intera cascata né la frequenza dei retry sui documenti reali.

L'override EOT predefinito è 59246, complementare all'EOS 59253 dei pesi
distribuiti. Per GGUF con EOS 59246 usare `--eot 59253`.

Pillow prepara il canvas con Lanczos; il test completo della pipeline Rust
resta la modalità `--prova`. Le piccole differenze di arrotondamento e filtro
fra le due librerie rendono i due percorsi distinti.

Il riuso della KV e le diverse configurazioni di thread possono cambiare
gli arrotondamenti numerici e quindi una decodifica greedy. L'identità del
testo su un campione non sostituisce una valutazione CER/WER sull'intero
insieme di validazione. CUDA richiede un collaudo su hardware NVIDIA.

## Misure locali del 12 settembre 2026

Windows, Intel i7-13700KF (24 thread logici), AMD Radeon RX 7900 XT,
motore e GGUF distribuiti, una pagina manoscritta locale. Due ripetizioni
per profilo dopo il riscaldamento, 128 token, opzione `--retry`.
Le cifre sono medie del solo tentativo con penalità:

| Backend | Thread generazione/batch | Senza cache | Retry con cache | Token/s senza cache |
| --- | ---: | ---: | ---: | ---: |
| CPU | 22/22, precedente | 14,923 s | 1,563 s | 92,1 |
| CPU | 8/8, nuovo | 14,555 s | 1,271 s | 120,0 |
| Vulkan | 22/22, precedente | 1,082 s | 0,483 s | 446,0 |
| Vulkan | 4/4, nuovo | 1,083 s | 0,482 s | 448,9 |

In tutti questi confronti il retry ha riusato 1541 token su 1542.
A parità di backend e thread, il testo con e senza cache è identico.
Fra CPU a 22 e 8 thread il testo dello stadio con penalità differisce:
non si dichiara quindi equivalenza numerica fra profili.

I risultati mostrano soprattutto l'eliminazione del prefill ripetuto:
il guadagno della pagina intera dipende da quanti tentativi richiede e da
quanto testo genera. La velocità del primo tentativo Vulkan resta
sostanzialmente invariata; ridurre i worker host non è una misura del
consumo energetico. I risultati non vanno estesi ad altre GPU o dataset.

Confronto aggiuntivo con la pipeline Rust completa (`--prova --testo`),
eseguibile distribuito contro la nuova build Release, stessa pagina,
un'esecuzione per configurazione:

| Backend | Tempo OCR precedente | Tempo OCR nuovo | Riduzione |
| --- | ---: | ---: | ---: |
| CPU | 33,06 s | 17,43 s | 47,3% |
| Vulkan | 2,34 s | 1,74 s | 25,6% |

Sono i secondi nel VLM, inclusi entrambi i tentativi, esclusi caricamento
del motore e preparazione della pagina. Tutte le esecuzioni completano la
pagina con due tentativi e chiudono il proprio server. Il testo Vulkan
resta identico. Su CPU il testo cambia fra i due profili (953 contro 951
caratteri); senza trascrizione di riferimento non si attribuisce a questa
differenza un miglioramento o peggioramento dell'accuratezza.

## Streaming, prefill e torre visiva

La diagnostica dell'app e `--prova` riportano per tentativo tempo al primo
token, tempo del prompt (visione inclusa) e tempo di generazione. Se un
tentativo è interrotto prima dell'evento finale, i tempi che il server non
ha comunicato restano assenti, non vengono inventati o impostati a zero.
Le sessioni archiviate con la versione precedente restano leggibili.

Il lettore HTTP riusa i buffer fra i chunk e costruisce direttamente le
righe SSE senza un vettore intermedio. La cascata legge gli ID token dal
JSON senza allocare un nuovo vettore per ogni evento. Il raggruppamento
degli aggiornamenti UI a 50 ms rimane: evita di inviare centinaia di eventi
al secondo alla webview. Questi interventi riducono allocazioni lato host;
non si attribuisce loro un'accelerazione misurata dei kernel del modello.

Ulteriori prove sullo stesso PC e sulla stessa pagina, 128 token greedy,
un riscaldamento e una misura per configurazione, generazione e batch a
8 thread:

| Thread visivi | Flash Attention | Codifica visiva CPU | Prefill immagine CPU | Primo token |
| ---: | --- | ---: | ---: | ---: |
| 4 | auto | 21,923 s | 1,770 s | 23,932 s |
| 8 | auto | 11,290 s | 1,760 s | 13,271 s |
| 12 | auto | 13,961 s | 1,745 s | 15,915 s |
| 16 | auto | 13,508 s | 1,823 s | 15,560 s |
| 24 | auto | 11,216 s | 1,740 s | 13,174 s |
| 8 | off | 21,479 s | 2,143 s | 23,843 s |

24 thread non danno un vantaggio sostanziale su 8, mentre le altre varianti
peggiorano. Anche limitare il processo alla maschera locale `0xffff` non
migliora: codifica 12,985 e 11,999 s nelle due ripetizioni. Queste opzioni
non diventano impostazioni predefinite.

Su CPU i microbatch 256, 512 e 1024 danno prefill di circa 1,73–1,76 s:
il tempo al primo token resta circa 13,27–13,36 s. Su Vulkan il microbatch
2048 dà un primo token a 0,776 s contro 0,808 s con 512 in una singola prova;
il vantaggio complessivo è piccolo e non giustifica un aumento predefinito
della memoria richiesta. Rimane il microbatch 512 del motore.

Lo streaming CPU a 8 thread registra intervalli fra eventi di circa
8,1 ms (mediana) e 8,5 ms (p95); Vulkan circa 2,1 e 2,2 ms. In queste prove
arrivano tutti i 128 eventi token e non emergono pause rilevanti del
trasporto. Il test Rust copre inoltre 4096 eventi, Content-Length, chunk
HTTP con estensioni/trailer e caratteri UTF-8 spezzati fra chunk.

L'accuratezza CER/WER resta da verificare: il dataset con trascrizioni di
riferimento non è attualmente disponibile. L'identità dei primi 128 token
nelle ablazioni non è una verifica di accuratezza su pagine complete.
