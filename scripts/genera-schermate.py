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
from demo import pagina_demo
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
            for theme in ("chiaro", "scuro"):
                page.locator(f'#scelta-tema [data-tema="{theme}"]').click()
                page.screenshot(path=str(out / ("app-light.png" if theme == "chiaro" else "app-dark.png")))
            assert not errors, errors
            logo = base64.b64encode((out / "logo.png").read_bytes()).decode()
            page.set_viewport_size({"width": 1200, "height": 630})
            page.set_content(f'''<!doctype html><html lang="it"><meta charset="utf-8"><style>
                *{{box-sizing:border-box}}body{{margin:0;background:#f6f5ef;color:#233d30;padding:65px;font-family:Arial}}
                header{{display:flex;gap:20px;align-items:center;font-size:28px;font-weight:bold}}img{{width:72px}}
                h1{{font:64px/1.12 Georgia;letter-spacing:-2px;margin:60px 0 25px}}p{{font-size:23px;color:#667069}}
                footer{{margin-top:42px;font-size:16px;letter-spacing:2px}}</style>
                <header><img src="data:image/png;base64,{logo}" alt="">ITA-OCR</header>
                <h1>Dalla pagina al testo.<br><em>Sul tuo computer.</em></h1>
                <p>OCR locale per la scrittura italiana, con GLM-OCR.</p>
                <footer>WINDOWS · OPEN SOURCE · ELABORAZIONE LOCALE</footer></html>''')
            page.screenshot(path=str(out / "social.png"))
            browser.close()
    finally:
        server.shutdown()
    print("Schermate sintetiche generate in docs/assets")


if __name__ == "__main__":
    main()
