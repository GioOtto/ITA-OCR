# Sicurezza · Security

## Italiano

### Segnalare una vulnerabilità

Non pubblicare vulnerabilità contenenti dati riservati in una issue pubblica.
Usa la funzione di segnalazione privata della scheda **Security** di GitHub,
oppure scrivi a **giorgio.ottoboni@proton.me**, allegando una riproduzione
sintetica e minimale.

Indica versione dell’app, sistema operativo, backend in uso e i passi
necessari a riprodurre il problema. Non allegare documenti personali, token,
password, archivi di sessione o log non controllati.

Il progetto è mantenuto da una persona sola nel tempo libero: non c’è un
servizio di supporto con tempi di risposta garantiti. Le segnalazioni serie
vengono comunque lette e affrontate.

### Versioni supportate

Riceve correzioni soltanto l’ultima release pubblicata.

### Perimetro

Rientrano nel perimetro l’applicazione desktop, gli script di costruzione e
dell’installer e il sito statico in `docs/`. Restano fuori le vulnerabilità dei
progetti a monte (llama.cpp, Tauri, PDFium, i dizionari), che vanno segnalate
ai rispettivi progetti, e i modelli GLM-OCR a monte.

### Note sul modello di minaccia

Il motore deve restare in ascolto solo su loopback: non esporre la sua porta
alla rete. Il pacchetto Windows non è firmato con Authenticode, quindi la
verifica dell’origine passa dal checksum pubblicato nella release.

Il software non è un sistema di anonimizzazione e non garantisce che le
trascrizioni siano prive di dati personali. Un modello linguistico può
produrre testo errato o inventato: nessun risultato va considerato affidabile
senza confronto con l’originale.

## English

### Reporting a vulnerability

Do not open a public issue for a vulnerability that contains confidential data.
Use GitHub's private reporting under the **Security** tab, or write to
**giorgio.ottoboni@proton.me**, attaching a minimal synthetic reproduction.

State the application version, operating system, active backend and the steps
needed to reproduce the problem. Do not attach personal documents, tokens,
passwords, session histories or unreviewed logs.

The project is maintained by one person in their spare time: there is no
support service with guaranteed response times. Serious reports are still read
and addressed.

### Supported versions

Only the latest published release receives fixes.

### Scope

In scope: the desktop application, the build and installer scripts, and the
static website in `docs/`. Out of scope: vulnerabilities in upstream projects
(llama.cpp, Tauri, PDFium, the dictionaries), which should be reported to those
projects, and the upstream GLM-OCR models.

### Threat-model notes

The engine must keep listening on loopback only: do not expose its port to the
network. The Windows package is not signed with Authenticode, so origin
verification relies on the checksum published with the release.

The software is not an anonymisation system and does not guarantee that
transcriptions are free of personal data. A language model can produce wrong or
invented text: no result should be trusted without comparing it to the original.
