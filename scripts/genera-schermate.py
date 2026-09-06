"""Cattura l'interfaccia con dati sintetici per README e sito."""
import base64
import importlib.util
import json
from pathlib import Path
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import functools
import sys
import threading
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "ocr-desktop/smoke"))
from demo import pagina_demo, pagina_manoscritta, TRASCRIZIONE_MANOSCRITTA
spec = importlib.util.spec_from_file_location("smoke_ui", ROOT / "ocr-desktop/smoke/ui.py")
ui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ui)


def main():
    out = ROOT / "docs/assets"
    out.mkdir(parents=True, exist_ok=True)
    handler = functools.partial(ui.QuietHandler, directory=str(ROOT / "ocr-desktop/app/ui"))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch()
            page = browser.new_page(viewport={"width": 1380, "height": 920}, reduced_motion="reduce")
            errors = []
            page.on("pageerror", lambda err: errors.append(str(err)))
            page.add_init_script("window.__preview=" + json.dumps(base64.b64encode(pagina_demo()).decode()) + ";" + ui.BRIDGE)
            page.goto(f"http://127.0.0.1:{server.server_port}")
            expect(page.locator(".apri-voce")).to_have_count(3)
            page.evaluate("window.__fixture('completata')")
            page.locator(".testo-blocco").first.wait_for(state="visible")
            page.locator(".katex").first.wait_for(state="visible")
            # Le anteprime arrivano da un osservatore piu' un rinvio di 150 ms:
            # senza questa attesa la cattura coglie ancora il segnaposto
            # "Caricamento anteprima" al posto della pagina.
            page.locator("#scorri-documento .cornice img").first.wait_for(state="visible")
            page.wait_for_function(
                "() => [...document.querySelectorAll('#scorri-documento .cornice img')]"
                ".every(i => i.complete && i.naturalWidth > 0)"
            )
            for theme in ("chiaro", "scuro"):
                page.locator(f'#scelta-tema [data-tema="{theme}"]').click()
                page.screenshot(path=str(out / ("app-light.png" if theme == "chiaro" else "app-dark.png")))

            # Terza schermata: una pagina manoscritta, che e' il caso d'uso
            # vero dell'applicazione. Si scambiano immagine e trascrizione nel
            # ponte simulato e si ricostruisce l'elenco, cosi' le carte
            # ricaricano l'anteprima nuova. In tema scuro, perche' il sito ha
            # il fondo nero e una schermata chiara ci starebbe come un faro.
            page.locator('#scelta-tema [data-tema="scuro"]').click()
            page.evaluate(
                "([immagine, testo]) => {"
                "  window.__preview = immagine;"
                "  window.__nomeDocumento = 'Fisica - appunti a mano.jpg';"
                "  window.__contenuti.testo = testo;"
                "}",
                [base64.b64encode(pagina_manoscritta()).decode(), TRASCRIZIONE_MANOSCRITTA],
            )
            page.evaluate("window.__fixture('completata')")
            page.locator(".testo-blocco").first.wait_for(state="visible")
            page.wait_for_function(
                "() => [...document.querySelectorAll('#scorri-documento .cornice img')]"
                ".length > 0 && [...document.querySelectorAll('#scorri-documento .cornice img')]"
                ".every(i => i.complete && i.naturalWidth > 0)"
            )
            page.screenshot(path=str(out / "app-manoscritto.png"))

            assert not errors, errors
            # L'anteprima per i social usa lo stesso vestito del sito: fondo
            # nero, un carattere solo, nessun punto divisore. Il font viaggia
            # dentro la pagina, che qui non ha una cartella da cui pescarlo.
            logo = base64.b64encode((out / "logo.png").read_bytes()).decode()
            carattere = base64.b64encode(
                (out / "fonts" / "inter-latin-variable.woff2").read_bytes()
            ).decode()
            page.set_viewport_size({"width": 1200, "height": 630})
            page.set_content(f'''<!doctype html><html lang="it"><meta charset="utf-8"><style>
                @font-face{{font-family:"Inter var";font-weight:100 900;
                  src:url(data:font/woff2;base64,{carattere}) format("woff2")}}
                *{{box-sizing:border-box}}
                body{{margin:0;background:#000;color:#fff;padding:72px;
                  font-family:"Inter var",Arial,sans-serif;-webkit-font-smoothing:antialiased}}
                header{{display:flex;gap:18px;align-items:center;font-size:26px;font-weight:500;
                  letter-spacing:-.01em}}img{{width:56px;border-radius:14px}}
                h1{{font-size:68px;line-height:1.06;font-weight:500;letter-spacing:-.03em;
                  margin:78px 0 28px}}
                p{{font-size:24px;line-height:1.5;color:#c4c4c4;margin:0}}
                footer{{margin-top:56px;display:flex;gap:12px}}
                footer span{{border:1px solid #242424;border-radius:999px;padding:8px 18px;
                  font-size:16px;color:#8c8c8c}}</style>
                <header><img src="data:image/png;base64,{logo}" alt="">ITA-OCR</header>
                <h1>Dalla pagina al testo.<br>Sul tuo computer.</h1>
                <p>OCR locale per la scrittura italiana, con GLM-OCR.</p>
                <footer><span>Windows</span><span>Open source</span>
                  <span>Elaborazione locale</span></footer></html>''')
            page.wait_for_timeout(400)
            page.screenshot(path=str(out / "social.png"))
            browser.close()
    finally:
        server.shutdown()
    print("Schermate sintetiche generate in docs/assets")


if __name__ == "__main__":
    main()
