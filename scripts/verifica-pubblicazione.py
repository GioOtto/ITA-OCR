"""Controlla che nel materiale pubblicabile non finisca roba privata.

Guarda solo i file che Git pubblicherebbe davvero: quelli tracciati piu' quelli
non tracciati e non ignorati. Le cartelle di lavoro locali restano fuori perche'
le esclude .gitignore, che e' esattamente la garanzia che si vuole verificare.

    python scripts/verifica-pubblicazione.py

Esce con 0 se e' tutto pulito, 1 se trova qualcosa. Nessuna dipendenza esterna.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

RADICE = Path(__file__).resolve().parent.parent

# Percorsi della macchina di sviluppo, materiale del corpus e nomi di file
# derivati dall'addestramento: niente di tutto questo deve uscire.
VIETATI = [
    (re.compile(r"[A-Za-z]:\\?Users\\?", re.I), "percorso assoluto di un profilo Windows"),
    (re.compile(r"[A-Za-z]:[\/]repo[ _-]?ocr", re.I), "percorso assoluto del repository di sviluppo"),
    (re.compile(r"/home/[a-z0-9_.-]+/", re.I), "percorso assoluto di una home Linux"),
    (re.compile(r"studocu", re.I), "riferimento alla fonte dei documenti"),
    (re.compile(r"lessico_train|contesto_train", re.I), "risorsa lessicale derivata dal corpus"),
    (re.compile(r"manoscritt[oi]-\d", re.I), "nome di un documento del corpus"),
    (re.compile(r"\bhf_[A-Za-z0-9]{30,}\b"), "token Hugging Face"),
    (re.compile(r"\bgh[pousr]_[A-Za-z0-9]{30,}\b"), "token GitHub"),
]

# File che non devono proprio esistere fra quelli pubblicati.
NOMI_VIETATI = [
    re.compile(r"(^|/)private/"),
    re.compile(r".*train.*\.txt$", re.I),
    re.compile(r"\.gguf$", re.I),
    re.compile(r"deep-research-report\.md$"),
    re.compile(r"\.env($|\.)"),
]

# Estensioni da non leggere come testo.
BINARI = {".png", ".jpg", ".jpeg", ".ico", ".gif", ".webp", ".pdf", ".woff",
          ".woff2", ".ttf", ".otf", ".gguf", ".exe", ".dll", ".zip", ".7z"}

LIMITE_BYTE = 50 * 1024 * 1024

# La versione ha una sola fonte: il manifesto di Tauri. Altrove compare dentro
# il nome dell'installer e nell'etichetta del sito, e sono copie: se una resta
# indietro il bottone "scarica" del sito punta a un file che nella release
# nuova non esiste piu', e nessuno se ne accorge finche' non da' 404.
MANIFESTO = Path("ocr-desktop/app/src-tauri/tauri.conf.json")
VERSIONI = [
    (re.compile(r"ITA-OCR-setup_v(\d+\.\d+\.\d+)"), "nome dell'installer"),
    (re.compile(r"ITA-OCR-v(\d+\.\d+\.\d+)-x86_64\.AppImage"), "nome dell'AppImage"),
    (re.compile(r'class="version">v(\d+\.\d+\.\d+)<'), "etichetta di versione del sito"),
]

# Un punto interrogativo isolato nel testo HTML e il carattere sostitutivo
# Unicode sono quasi sempre il risultato di una conversione di codifica. Il
# controllo avrebbe intercettato, per esempio, "Il codice ? stato" nell'app.
TESTI_INTERFACCIA = {Path("ocr-desktop/app/ui/index.html")}
CARATTERI_CORROTTI = re.compile(r"\ufffd|(?<=\s)\?(?=\s)")


def versione_dichiarata() -> str | None:
    """La versione del manifesto, unica fonte di verita'."""
    try:
        testo = (RADICE / MANIFESTO).read_text(encoding="utf-8")
    except OSError:
        return None
    trovata = re.search(r'"version"\s*:\s*"(\d+\.\d+\.\d+)"', testo)
    return trovata.group(1) if trovata else None


def elenco_file() -> list[str]:
    """I file che una pubblicazione porterebbe con se'."""
    tracciati = subprocess.run(
        ["git", "ls-files"], cwd=RADICE, capture_output=True, text=True, check=True
    ).stdout.split("\n")
    nuovi = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        cwd=RADICE, capture_output=True, text=True, check=True
    ).stdout.split("\n")
    # Il sottomodulo llama.cpp ha una storia sua: non lo si ispeziona qui.
    return sorted({f for f in tracciati + nuovi if f and not f.startswith("ocr-ita/vendor/")})


def link_locali(testo: str) -> list[str]:
    """Destinazioni di link Markdown che puntano a file del repository."""
    fuori = ("http://", "https://", "mailto:", "#")
    return [d for _, d in re.findall(r"\[([^\]]*)\]\(([^)\s]+)\)", testo)
            if not d.startswith(fuori)]


def main() -> int:
    problemi: list[str] = []
    file = elenco_file()
    versione = versione_dichiarata()
    if versione is None:
        problemi.append(f"{MANIFESTO.as_posix()}: versione non leggibile")

    for nome in file:
        percorso = RADICE / nome

        for regola in NOMI_VIETATI:
            if regola.search(nome):
                problemi.append(f"{nome}: file che non deve essere pubblicato")

        if not percorso.is_file():
            continue

        if percorso.stat().st_size > LIMITE_BYTE:
            mb = percorso.stat().st_size / 1e6
            problemi.append(f"{nome}: {mb:.0f} MB, oltre il limite di GitHub per file")

        if percorso.suffix.lower() in BINARI:
            continue

        # Questo script e' fatto di quelle parole, e .gitignore serve appunto a
        # nominare cio' che va escluso: citarle li' e' il comportamento giusto.
        if nome in ("scripts/verifica-pubblicazione.py", ".gitignore"):
            continue

        try:
            testo = percorso.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue

        if Path(nome) in TESTI_INTERFACCIA and CARATTERI_CORROTTI.search(testo):
            problemi.append(f"{nome}: possibile carattere corrotto nell'interfaccia")

        for numero, riga in enumerate(testo.splitlines(), 1):
            for regola, motivo in VIETATI:
                if regola.search(riga):
                    problemi.append(f"{nome}:{numero}: {motivo} -> {riga.strip()[:90]}")
            if versione:
                for regola, dove in VERSIONI:
                    for citata in regola.findall(riga):
                        if citata != versione:
                            problemi.append(
                                f"{nome}:{numero}: {dove} ferma a {citata}, "
                                f"il manifesto dice {versione}"
                            )

        if percorso.suffix == ".md":
            for destinazione in link_locali(testo):
                bersaglio = (percorso.parent / destinazione.split("#")[0]).resolve()
                if not bersaglio.exists():
                    problemi.append(f"{nome}: link rotto -> {destinazione}")

    print(f"esaminati {len(file)} file")
    if problemi:
        print(f"\n{len(problemi)} problemi:\n")
        for p in problemi:
            print("  " + p)
        return 1
    print("nessun riferimento privato, nessun link rotto, nessun file fuori misura")
    return 0


if __name__ == "__main__":
    sys.exit(main())
