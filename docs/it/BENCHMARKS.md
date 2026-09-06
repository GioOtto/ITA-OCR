# Valutazione e limiti

*[English version](../en/BENCHMARKS.md)*

Questa distribuzione non include il dataset di addestramento o di valutazione,
i documenti originali, le trascrizioni di riferimento o identificatori dei
partecipanti. Le schermate usano contenuti sintetici e non costituiscono un
benchmark dell'OCR.

## Metodo

Le valutazioni interne confrontano il modello base con il fine-tuning su
pagine italiane e osservano errori sui caratteri e sulle parole (CER e WER),
pagine incomplete, ripetizioni e latenza dell'intera pipeline. Preparazione
della pagina, backend, quantizzazione e strategia di decodifica influenzano
il risultato e devono essere indicati insieme a qualsiasi misura.

Il set di verifica finale è stato consultato più volte durante lo sviluppo:
non è un test indipendente incontaminato. Le misure interne vanno quindi
interpretate come descrittive. Non viene pubblicata una classifica o una
percentuale universale di accuratezza.

## Risultati misurati

Le cifre qui sotto vengono dal [report tecnico](../assets/ITA-OCR-report-tecnico.pdf)
e confrontano il modello base con il sistema distribuito. Ogni riga porta con sé
l'insieme su cui è stata misurata, perché è quello a dare senso alla percentuale.
Il rumore di fondo delle misure è di 0,3 punti: i margini riportati sono molto
più larghi.

| Insieme | Pagine | Errore caratteri | Errore parole |
| --- | --- | --- | --- |
| Holdout, scriventi disgiunti dall'addestramento | 164 | 30,88 → 25,71 (−16,7%) | 54,68 → 45,54 (−16,7%) |
| Benchmark sigillato, parte leggibile | 67 | 29,28 → 18,78 (−35,9%) | 56,74 → 38,99 (−31,3%) |
| Coorte | 136 | −8,8% | −20,4% |
| Sottoinsieme dichiarato facile a priori | 61 | 12,06 → 11,28 (−6,5%) | 36,97 → 30,67 (−17,0%) |

L'holdout usa l'errore sui caratteri con tetto, l'unica statistica equa quando
si perdono pagine; sugli altri insiemi nessuno dei due modelli perde pagine,
quindi grezzo e con tetto coincidono. Il benchmark esclude un singolo scrivente
al limite della leggibilità, che da solo sposta l'aggregato più di qualsiasi
differenza fra modelli.

Sul sottoinsieme facile, pagina per pagina, il sistema distribuito vince su 43,
perde su 17 e pareggia su 1: il vantaggio aggregato è reale e non è uniforme.

### Pagine perse e cascata

| Configurazione | Pagine perse su 48 |
| --- | --- |
| Decodifica greedy, nessuna mitigazione | 24 |
| Penalità di frequenza 0,35 + presenza 0,20 | 5 |
| Sola penalità di presenza 0,20 | 22 |
| Campionamento DRY | 22 |

La cascata costa meno del non averla: sull'holdout una passata completa con
cascata impiega meno di una singola passata greedy sulle stesse pagine, perché
le pagine in fuga vengono interrotte intorno ai 550 token invece di correre
fino al tetto di 4.096, e il tentativo successivo è breve.

### Quantizzazione e CPU

I pesi distribuiti sono Q8_0 per entrambe le torri. Rispetto a bf16 usano circa
il 30% di memoria video in meno, decodificano circa il 35% più in fretta e
risultano appaiati in qualità pagina per pagina. Q5_K_M e Q4_K_M mostrano un
peggioramento piccolo ma sistematico.

Su CPU la qualità non cambia, cambia il tempo. Sulla macchina di collaudo una
pagina passa da poco più di un secondo e mezzo su GPU a una quindicina di
secondi su CPU, e la maggior parte di quel tempo se ne va nella codifica
dell'immagine, non nella generazione del testo. **I valori assoluti dipendono
dalla macchina** (processore, scheda video, driver, complessità della pagina)
e vanno letti come rapporto, non come prestazione garantita. Su CPU la
quantizzazione fa risparmiare memoria più che tempo: circa 2,4 GB residenti con
Q8_0. La leva sulla latenza sarebbe la risoluzione della pagina, non la
precisione dei pesi.

## Verifiche del pacchetto

La build Windows è stata collaudata con GPU AMD Radeon RX 7900 XT e Vulkan.
Questo verifica che la pipeline possa completare un documento su quella
configurazione, non certifica tutte le GPU né la qualità di ogni trascrizione.
La suite UI usa un ponte simulato e verifica le interazioni dell'app.

## Limiti noti

- Scrittura complessa, impaginazione irregolare, formule e tabelle possono
  causare omissioni, sostituzioni, ripetizioni o contenuti inventati.
- La correzione da dizionario può introdurre errori: le sostituzioni restano
  evidenziate e il risultato va confrontato con l'originale.
- Piccole variazioni nella decodifica GPU possono produrre risultati diversi.
- Il riconoscimento su CPU può essere sensibilmente più lento.
- I risultati ottenuti con risorse lessicali private non sono trasferibili
  automaticamente al pacchetto pubblico, che usa solo dizionari pubblici.

Per una riproduzione pubblica usa esclusivamente documenti sintetici o dati
che puoi distribuire e documenta ambiente, parametri e criteri di misura.
