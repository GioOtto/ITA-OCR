"""Disegna gli schemi del README e del sito.

Ogni schema esiste in quattro copie: italiano e inglese, tema chiaro e scuro.
Sono file distinti e non uno solo adattivo perche' GitHub sceglie l'immagine
con <picture> e non applica i media query scritti dentro un SVG.

Tenerli generati da qui evita di mantenere a mano otto file quasi uguali: si
cambia una parola o un colore in un posto solo.

    python scripts/genera-diagrammi.py
"""

from __future__ import annotations

from pathlib import Path
from xml.sax.saxutils import escape

USCITA = Path(__file__).resolve().parents[1] / "docs/assets"

# Gli stessi colori dell'applicazione, presi da app/ui/styles.css. Li' l'accento
# e' inchiostro e non un colore, e gli schemi seguono la stessa regola: cosi'
# non stonano ne' con l'interfaccia ne' con il sito.
TEMI = {
    "light": dict(fondo="#f7f7f8", carta="#ffffff", bordo="#e6e6e8", testo="#1f2023",
                  tenue="#5c6065", accento="#1f2023", confine="#c9cacd"),
    "dark": dict(fondo="#0f0f0f", carta="#1a1a1a", bordo="#2f2f2f", testo="#ececec",
                 tenue="#b4b4b4", accento="#ececec", confine="#4a4a4a"),
}

FONT = "system-ui,-apple-system,Segoe UI,Roboto,Arial,sans-serif"


def testo(x, y, contenuto, dimensione, colore, peso="400", spaziatura=None, ancora="start"):
    extra = f' letter-spacing="{spaziatura}"' if spaziatura else ""
    return (f'<text x="{x}" y="{y}" font-size="{dimensione}" fill="{colore}" '
            f'font-weight="{peso}" text-anchor="{ancora}"{extra}>{escape(contenuto)}</text>')


def carta(x, y, w, h, c, raggio=12):
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{raggio}" '
            f'fill="{c["carta"]}" stroke="{c["bordo"]}" stroke-width="1.5"/>')


def freccia_destra(x1, x2, y, c, spessore=2):
    return (f'<path d="M{x1} {y}H{x2 - 9}" stroke="{c["accento"]}" stroke-width="{spessore}" fill="none"/>'
            f'<path d="M{x2 - 11} {y - 5}l6 5-6 5" stroke="{c["accento"]}" stroke-width="{spessore}" '
            f'fill="none" stroke-linecap="round" stroke-linejoin="round"/>')


def freccia_sinistra(x1, x2, y, c, spessore=2):
    return (f'<path d="M{x1} {y}H{x2 + 9}" stroke="{c["accento"]}" stroke-width="{spessore}" fill="none"/>'
            f'<path d="M{x2 + 11} {y - 5}l-6 5 6 5" stroke="{c["accento"]}" stroke-width="{spessore}" '
            f'fill="none" stroke-linecap="round" stroke-linejoin="round"/>')


def involucro(w, h, corpo, c, titolo, descrizione):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
            f'viewBox="0 0 {w} {h}" role="img" aria-labelledby="titolo descrizione">'
            f'<title id="titolo">{escape(titolo)}</title>'
            f'<desc id="descrizione">{escape(descrizione)}</desc>'
            f'<rect width="{w}" height="{h}" rx="18" fill="{c["fondo"]}"/>'
            f'<g font-family="{FONT}">{corpo}</g></svg>')


# --------------------------------------------------------------------- pipeline

PIPELINE = {
    "it": dict(
        titolo="La pipeline locale di ITA-OCR",
        descrizione=("Il documento viene preparato, riconosciuto da GLM-OCR con llama.cpp, "
                     "corretto sul lessico e confrontato con l'originale, tutto sul computer "
                     "dell'utente. I PDF che contengono gia' il testo saltano il riconoscimento."),
        confine="IL TUO COMPUTER · NESSUNA PAGINA ESCE",
        tappe=[("Documento", "PDF o immagine", "aperto dall'utente"),
               ("Preparazione", "pagina 960 × 1248", "150 DPI, come in addestramento"),
               ("Riconoscimento", "GLM-OCR su llama.cpp", "CPU, Vulkan o CUDA"),
               ("Verifica", "confronto ed esportazione", "correzioni evidenziate")],
        cascata="cascata: se la pagina finisce incompleta, ritenta",
        salto="PDF con livello di testo: estrazione diretta, senza passare dal modello",
    ),
    "en": dict(
        titolo="The local ITA-OCR pipeline",
        descrizione=("The document is prepared, recognised by GLM-OCR through llama.cpp, "
                     "spell-checked and compared with the original, all on the user's machine. "
                     "PDFs that already carry a text layer skip recognition."),
        confine="YOUR MACHINE · NO PAGE EVER LEAVES",
        tappe=[("Document", "PDF or image", "opened by the user"),
               ("Preparation", "960 × 1248 page", "150 DPI, as in training"),
               ("Recognition", "GLM-OCR on llama.cpp", "CPU, Vulkan or CUDA"),
               ("Review", "compare and export", "corrections stay visible")],
        cascata="cascade: retries when a page comes back incomplete",
        salto="PDF with a text layer: extracted directly, the model is not involved",
    ),
}


def disegna_pipeline(lingua: str, tema: str) -> str:
    c = TEMI[tema]
    d = PIPELINE[lingua]
    larghezza_tela, altezza_tela = 1160, 356
    x0, y0, larghezza, altezza, passo = 56, 96, 244, 132, 268
    p = []

    # Il confine del dispositivo: tutto cio' che conta accade dentro.
    p.append(f'<rect x="32" y="58" width="{larghezza_tela - 64}" height="232" rx="16" fill="none" '
             f'stroke="{c["confine"]}" stroke-width="1.5" stroke-dasharray="7 6"/>')
    p.append(testo(52, 44, d["confine"], 12, c["tenue"], "600", "1.8"))

    for i, (titolo, riga, nota) in enumerate(d["tappe"]):
        x = x0 + i * passo
        p.append(carta(x, y0, larghezza, altezza, c))
        p.append(f'<rect x="{x}" y="{y0}" width="4" height="{altezza}" rx="2" fill="{c["accento"]}"/>')
        p.append(testo(x + 24, y0 + 42, titolo, 20, c["testo"], "600"))
        p.append(testo(x + 24, y0 + 72, riga, 14.5, c["tenue"]))
        p.append(testo(x + 24, y0 + 96, nota, 12.5, c["tenue"]))
        if i < len(d["tappe"]) - 1:
            p.append(freccia_destra(x + larghezza + 6, x + passo - 2, y0 + altezza / 2, c))

    # Il ritorno della cascata, sotto la tappa del riconoscimento.
    xr = x0 + 2 * passo
    p.append(f'<path d="M{xr + larghezza - 40} {y0 + altezza}v22h-{larghezza - 80}v-22" '
             f'stroke="{c["accento"]}" stroke-width="1.6" fill="none" stroke-dasharray="5 4"/>')
    p.append(f'<path d="M{xr + 35} {y0 + altezza + 8}l5-8 5 8" stroke="{c["accento"]}" '
             f'stroke-width="1.6" fill="none" stroke-linecap="round" stroke-linejoin="round"/>')
    p.append(testo(xr + larghezza / 2, y0 + altezza + 44, d["cascata"], 12.5, c["tenue"], ancora="middle"))

    p.append(testo(56, altezza_tela - 20, d["salto"], 13.5, c["tenue"]))
    return involucro(larghezza_tela, altezza_tela, "".join(p), c, d["titolo"], d["descrizione"])


# ---------------------------------------------------------------------- privacy

PRIVACY = {
    "it": dict(
        titolo="Dove restano i dati con ITA-OCR",
        descrizione=("Documenti, trascrizioni, archivio, modello e dizionari restano sul "
                     "computer. La rete serve solo a scaricare applicazione e pesi. Nessuna "
                     "telemetria, nessuna API OCR remota, nessun account."),
        dentro="IL TUO COMPUTER", fuori="RETE",
        elementi=[("Documenti e pagine", "aperti in locale, mai inviati"),
                  ("Trascrizioni e archivio", "salvati nel profilo utente"),
                  ("Modello GGUF", "letto da disco a ogni avvio"),
                  ("Dizionari", "correzione lessicale locale"),
                  ("llama-server", "in ascolto su 127.0.0.1")],
        ingressi=[("Installer", "GitHub · una volta"),
                  ("Pesi del modello", "Hugging Face · una volta")],
        senso="solo in entrata",
        piede="Nessuna telemetria · Nessuna API OCR remota · Nessun account · Funziona offline",
    ),
    "en": dict(
        titolo="Where the data stays with ITA-OCR",
        descrizione=("Documents, transcriptions, history, model and dictionaries stay on the "
                     "machine. The network is only used to download the app and the weights. "
                     "No telemetry, no remote OCR API, no account."),
        dentro="YOUR MACHINE", fuori="NETWORK",
        elementi=[("Documents and pages", "opened locally, never uploaded"),
                  ("Transcriptions and history", "stored in the user profile"),
                  ("GGUF model", "read from disk at start-up"),
                  ("Dictionaries", "local spelling correction"),
                  ("llama-server", "listening on 127.0.0.1")],
        ingressi=[("Installer", "GitHub · once"),
                  ("Model weights", "Hugging Face · once")],
        senso="inbound only",
        piede="No telemetry · No remote OCR API · No account · Works offline",
    ),
}


def disegna_privacy(lingua: str, tema: str) -> str:
    c = TEMI[tema]
    d = PRIVACY[lingua]
    larghezza_tela, altezza_tela = 1160, 400
    p = []

    p.append(f'<rect x="36" y="60" width="660" height="284" rx="16" fill="none" '
             f'stroke="{c["confine"]}" stroke-width="1.5" stroke-dasharray="7 6"/>')
    p.append(testo(56, 44, d["dentro"], 12, c["tenue"], "600", "1.8"))
    p.append(testo(826, 44, d["fuori"], 12, c["tenue"], "600", "1.8"))

    for i, (nome, nota) in enumerate(d["elementi"]):
        y = 84 + i * 52
        p.append(carta(58, y, 616, 44, c, 10))
        p.append(f'<circle cx="82" cy="{y + 22}" r="4.5" fill="{c["accento"]}"/>')
        p.append(testo(100, y + 27, nome, 15.5, c["testo"], "600"))
        p.append(testo(372, y + 27, nota, 13.5, c["tenue"]))

    # Le due sole cose che attraversano il confine, e lo fanno in un senso solo.
    for i, (nome, nota) in enumerate(d["ingressi"]):
        y = 120 + i * 112
        p.append(carta(826, y, 298, 78, c, 10))
        p.append(testo(850, y + 33, nome, 16.5, c["testo"], "600"))
        p.append(testo(850, y + 57, nota, 13, c["tenue"]))
        p.append(freccia_sinistra(818, 704, y + 39, c))

    p.append(testo(761, 336, d["senso"], 12.5, c["tenue"], ancora="middle"))
    p.append(testo(larghezza_tela / 2, altezza_tela - 22, d["piede"], 13.5, c["tenue"], ancora="middle"))
    return involucro(larghezza_tela, altezza_tela, "".join(p), c, d["titolo"], d["descrizione"])


def main() -> None:
    USCITA.mkdir(parents=True, exist_ok=True)
    for nome, disegna in (("pipeline", disegna_pipeline), ("privacy", disegna_privacy)):
        for lingua in ("it", "en"):
            for tema in ("light", "dark"):
                percorso = USCITA / f"{nome}-{lingua}-{tema}.svg"
                percorso.write_text(disegna(lingua, tema), encoding="utf-8", newline="\n")
                print("  " + percorso.name)


if __name__ == "__main__":
    main()
