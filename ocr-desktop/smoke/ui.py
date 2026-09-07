"""Verifica la UI con un ponte Tauri simulato, senza leggere l'archivio personale.

Prerequisiti: pip install playwright; python -m playwright install chromium
Esecuzione dalla radice: python ocr-desktop/smoke/ui.py
Le schermate finiscono in ocr-desktop/dist/ui-v1.0.0/. Non verifica il modello OCR.
"""

import base64
from demo import pagina_demo
import functools
import json
import re
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import threading

from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "ocr-desktop/dist/ui-v1.0.0"
OUT.mkdir(parents=True, exist_ok=True)

BRIDGE = r"""
(() => {
  const events = new Map();
  const emit = async (name, payload) => {
    for (const handler of events.get(name) || []) await handler({ payload });
  };
  let pages = [];
  let archive = [
    { id: 'a', nome: 'Appunti di matematica', pagine: 3, istante: 1788604800, completa: true },
    { id: 'b', nome: 'Università · lezioni di storia', pagine: 8, istante: 1788518400, completa: true },
    { id: 'c', nome: 'Esercizio dimostrativo', pagine: 2, istante: 1788432000, completa: false },
  ];
  const engine = { fase: 'pronto', etichetta: 'Motore pronto · CPU', dispositivo: 'CPU di prova' };
  const sample = (state = 'attesa') => Array.from({ length: 3 }, (_, n) => ({
    id: `d0p${n+1}`, documento: 0, nome_documento: window.__nomeDocumento || 'Appunti di matematica.pdf',
    numero: n+1, pagine_documento: 3, stato: state, caratteri: state === 'completata' ? 800 : 0,
    correzioni_dizionario: state === 'completata' ? 2 : 0, correzioni_contesto: 0,
    secondi: state === 'completata' ? 3.2 : 0, tentativi: 1, fast_path: false,
  }));
  const contents = {
    testo: 'Appunti di matematica\n\nIl concetto di funzione\n\nUna funzione associa a ogni elemento del dominio uno e un solo elemento del codominio. La relazione può essere descritta con una formula, una tabella o un grafico.\n\nConsideriamo la funzione $f(x) = x^2 + 2x + 1$.\n\nPer trovare i suoi zeri, raccogliamo il quadrato di un binomio:\n\n$$f(x) = (x + 1)^2$$\n\nIl vertice si trova nel punto $V = (-1, 0)$. La parabola ha concavità rivolta verso l’alto ed è simmetrica rispetto alla retta $x = -1$.\n\n| x | f(x) |\n| --- | --- |\n| -2 | 1 |\n| -1 | 0 |\n| 0 | 1 |\n\nOsservazione\n\nLo stesso procedimento si applica alle altre funzioni quadratiche. Prima si individua il dominio, poi si studiano il segno e le intersezioni con gli assi.',
    correzioni: [],
  };
  // Le schermate del sito cambiano pagina e trascrizione senza toccare il
  // ponte: qui si espone il contenuto, li' lo si sostituisce.
  window.__contenuti = contents;
  window.__calls = [];
  window.__failStart = false;
  window.__saved = [];
  window.__emit = emit;
  window.__fixture = async (state) => { pages = sample(state); await emit('ocr://elenco', pages); };
  window.__ripristinaArchivio = async () => emit('ocr://archivio', archive);
  const invoke = async (name, args = {}) => {
    window.__calls.push({ name, args });
    switch (name) {
      case 'info_motore': return engine;
      case 'elenco': return pages;
      case 'archivio_elenco': return archive;
      case 'plugin:dialog|open': return ['demo/Appunti di matematica.pdf'];
      case 'apri_file': pages = sample(); await emit('ocr://elenco', pages); return [];
      case 'anteprima': return 'data:image/png;base64,' + window.__preview;
      case 'testo_pagina': return contents;
      case 'svuota': pages = []; await emit('ocr://elenco', pages); return;
      case 'avvia':
        if (window.__failStart) throw new Error('Motore di prova non disponibile');
        await emit('ocr://progresso', { in_corso: true, id_corrente: 'd0p1' });
        return;
      case 'annulla':
        pages = pages.map(p => ({ ...p, stato: p.stato === 'attesa' ? 'annullata' : p.stato }));
        await emit('ocr://elenco', pages);
        await emit('ocr://progresso', { in_corso: false }); return;
      case 'archivio_carica': pages = sample('completata'); await emit('ocr://elenco', pages); return 0;
      case 'archivio_elimina': archive = archive.filter(a => a.id !== args.id); await emit('ocr://archivio', archive); return;
      case 'esporta': return contents.testo;
      case 'plugin:dialog|save': return `demo/appunti.${args.options.defaultPath.split('.').pop()}`;
      case 'salva_file': window.__saved.push(args); return args.percorso;
      case 'esporta_docx': window.__saved.push(args); return args.percorso;
      case 'plugin:clipboard-manager|write_text': return;
      case 'imposta_correzione_automatica': return;
      case 'imposta_correzione_contestuale': return 0;
      case 'imposta_forza_ocr': return;
      case 'imposta_backend': return;
      case 'riavvia_motore': return;
      case 'apri_cartella_log': return 'demo/log';
      case 'apri_repository': return 'https://github.com/GioOtto/ITA-OCR';
      case 'diagnostica': return {
        motore: engine, canvas: [960, 1248], dpi: 200, n_predict: 4096,
        cascata: [{ stadio: 'greedy' }], rilevatore: { min_ripetizioni: 3, periodo_massimo: 40, span_minimo: 100 },
        impostazioni: { backend: 'auto', forza_ocr: false, correzione_automatica: true, correzione_contestuale: false },
        righe_log: [],
      };
      default: throw new Error(`Comando senza fixture: ${name}`);
    }
  };
  window.__TAURI__ = {
    core: { invoke },
    event: { listen: async (name, handler) => { events.set(name, [...(events.get(name) || []), handler]); return () => {}; } },
    webview: { getCurrentWebview: () => ({ onDragDropEvent: async handler => { window.__drop = handler; return () => {}; } }) },
  };
})();
"""


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_args):
        pass


def run():
    handler = functools.partial(QuietHandler, directory=str(ROOT / "ocr-desktop/app/ui"))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    preview = base64.b64encode(pagina_demo()).decode()
    errors = []
    with sync_playwright() as pw:
        browser = pw.chromium.launch()
        context = browser.new_context(viewport={"width": 1380, "height": 920}, color_scheme="light", reduced_motion="reduce")
        context.add_init_script("window.__preview = " + json.dumps(preview) + ";" + BRIDGE)
        page = context.new_page()
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.goto(f"http://127.0.0.1:{server.server_port}")
        expect(page.locator(".apri-voce")).to_have_count(3)
        # Senza una scelta salvata si parte dal chiaro, gia' dall'attributo.
        expect(page.locator("html")).to_have_attribute("data-tema", "chiaro")
        expect(page.locator('#scelta-tema [data-tema="chiaro"]')).to_have_attribute("aria-pressed", "true")
        # Un solo numero di versione, e la firma d'autore in fondo al pannello.
        expect(page).to_have_title("ITA-OCR 1.0.0")
        expect(page.locator(".versione")).to_have_text("1.0.0")
        expect(page.locator("#apri-informazioni")).to_contain_text("ITA-OCR")
        # Ctrl+O resta una scorciatoia: non deve piu' essere scritta da nessuna parte.
        assert "Ctrl" not in page.locator("#riquadro-drop").inner_text()
        assert page.locator("#riquadro-drop kbd").count() == 0
        assert "Ctrl" not in (page.locator("#scegli-file").get_attribute("title") or "")
        # L'avvertenza sul modello si vede anche prima di importare qualcosa.
        avviso_home = page.locator(".avviso-minuto")
        expect(avviso_home).to_be_visible()
        expect(avviso_home).to_contain_text("modello di intelligenza artificiale")
        assert avviso_home.evaluate("e => parseFloat(getComputedStyle(e).fontSize)") <= 12
        expect(page.locator("#strumenti-lavoro")).to_be_hidden()
        expect(page.locator(".pie")).to_be_hidden()
        expect(page.locator("#testo-progresso")).to_have_text("")
        content_box = page.locator(".corpo").bounding_box()
        drop_box = page.locator("#riquadro-drop").bounding_box()
        assert abs(drop_box['x'] + drop_box['width']/2 - content_box['x'] - content_box['width']/2) < 1
        assert abs(drop_box['y'] + drop_box['height']/2 - content_box['y'] - content_box['height']/2) < 1
        assert page.locator('.guscio').evaluate("e => !/ritrovato|Dalla pagina|pagina alla volta|Pronto per/.test(e.textContent)")
        page.screenshot(path=str(OUT / "01-home.png"))

        page.locator("#cerca-archivio").fill("universita")
        expect(page.locator(".apri-voce")).to_have_count(1)
        page.locator("#cerca-archivio").fill("nessun documento")
        expect(page.locator("#archivio-nessun-risultato")).to_be_visible()
        page.locator("#cerca-archivio").fill("")
        expect(page.locator(".gruppo-archivio").first).to_be_visible()

        # Archivio piu' lungo del pannello: scorre dentro la sua cassetta e
        # lascia al loro posto la nuova trascrizione e la scelta del tema.
        page.evaluate('''() => window.__emit('ocr://archivio', Array.from({ length: 24 }, (_, n) => ({
            id: 'x' + n, nome: 'Trascrizione di prova numero ' + (n + 1), pagine: n + 1,
            istante: Math.floor(Date.now() / 1000) - n * 39600, completa: true })))''')
        expect(page.locator(".apri-voce")).to_have_count(24)
        assert page.locator(".scorri-archivio").evaluate("e => e.scrollHeight > e.clientHeight + 4")
        cassetta = page.locator(".cassetta-archivio").bounding_box()
        rail = page.locator("#fianco").bounding_box()
        tema = page.locator(".pie-fianco").bounding_box()
        assert cassetta["y"] + cassetta["height"] <= rail["y"] + rail["height"], (cassetta, rail)
        assert tema["y"] >= cassetta["y"] + cassetta["height"] - 1, (tema, cassetta)
        expect(page.locator("#nuovo-ocr")).to_be_visible()
        assert len(page.locator(".gruppo-archivio").all()) >= 3
        page.screenshot(path=str(OUT / "08-archivio-lungo.png"))
        page.evaluate("() => window.__ripristinaArchivio()")
        expect(page.locator(".apri-voce")).to_have_count(3)

        page.locator("#apri-avanzate").click()
        expect(page.locator("#chiudi-avanzate")).to_be_focused()
        assert page.locator("#pannello-avanzate #correzione-contestuale").count() == 1
        expect(page.locator("#correzione-contestuale")).to_be_visible()
        page.screenshot(path=str(OUT / "02-impostazioni.png"))
        for _ in range(20):
            page.keyboard.press("Tab")
            assert page.evaluate("document.querySelector('#pannello-avanzate').contains(document.activeElement)")
        calls = page.evaluate("window.__calls.length")
        page.keyboard.press("Control+o")
        assert not any(c["name"] == "plugin:dialog|open" for c in page.evaluate("window.__calls")[calls:])
        page.keyboard.press("Escape")
        expect(page.locator("#apri-avanzate")).to_be_focused()
        # Col cassetto chiuso -- davvero chiuso, la scivolata dura 240 ms --
        # la scorciatoia funziona.
        expect(page.locator("#pannello-avanzate")).to_be_hidden()
        calls = page.evaluate("window.__calls.length")
        page.keyboard.press("Control+o")
        expect(page.locator("#area-lavoro")).to_be_visible()
        assert any(c["name"] == "plugin:dialog|open" for c in page.evaluate("window.__calls")[calls:])
        assert "Ctrl" not in (page.locator("#aggiungi-altri").get_attribute("title") or "")
        page.locator("#svuota").click()
        expect(page.locator("#zona-vuota")).to_be_visible()

        page.locator("#scegli-file").click()
        expect(page.locator("#area-lavoro")).to_be_visible()
        expect(page.locator("#strumenti-lavoro")).to_be_visible()
        expect(page.locator("#nome-sessione")).to_have_text("Appunti di matematica.pdf")
        expect(page.locator("#posizione-pagina")).to_have_text("1 / 3")
        expect(page.locator("#copia")).to_be_disabled()
        expect(page.locator("#barra-progresso")).to_be_hidden()
        expect(page.locator("#avvia")).to_be_visible()
        assert page.locator(".testa-trascrizione #avvia").count() == 1
        assert page.locator("#avvia").bounding_box()["height"] >= 32
        # L'avvertenza sul modello sta sotto il testo e non se ne va piu'.
        expect(page.locator(".avviso-modello")).to_be_visible()
        expect(page.locator(".avviso-modello")).to_contain_text("errori")
        page.wait_for_function("document.querySelector('.cornice img')?.naturalWidth > 0")
        page.screenshot(path=str(OUT / "09-da-trascrivere.png"))
        page.locator("#pagina-successiva").click()
        expect(page.locator("#posizione-pagina")).to_have_text("2 / 3")
        page.locator("#pagina-precedente").click()
        page.locator("#divisore").focus()
        page.keyboard.press("ArrowRight")
        expect(page.locator("#divisore")).to_have_attribute("aria-valuenow", "47")
        page.keyboard.press("Home")
        page.evaluate("window.__failStart = true")
        page.locator("#avvia").click()
        expect(page.locator("#avvia")).to_be_enabled()
        expect(page.locator("#brindisi")).to_contain_text("Trascrizione non avviata")
        page.locator("#brindisi").click()
        page.evaluate("window.__failStart = false")
        page.locator("#avvia").click()
        expect(page.locator("#interrompi")).to_be_visible()
        expect(page.locator("#svuota")).to_be_disabled()
        page.keyboard.press("Escape")
        expect(page.locator("#avvia")).to_be_enabled()
        page.locator("#brindisi").click()

        page.evaluate("window.__fixture('completata')")
        expect(page.locator("#avvia")).to_be_hidden()
        expect(page.locator("#conteggio-dizionario")).to_have_text("6")
        assert page.locator("#conteggio-contesto").count() == 0
        # Le pagine prese dal testo del PDF dicevano "text layer" due volte.
        page.evaluate('''() => window.__emit('ocr://elenco', [{
            id: 'd0p1', documento: 0, nome_documento: 'Appunti di matematica.pdf',
            numero: 1, pagine_documento: 1, stato: 'completata', caratteri: 800,
            correzioni_dizionario: 0, correzioni_contesto: 0, secondi: 0.2,
            tentativi: 1, fast_path: true }])''')
        expect(page.locator(".testa-blocco").first).to_contain_text("text layer")
        assert page.locator(".testa-blocco").first.inner_text().count("text layer") == 1
        page.evaluate("window.__fixture('completata')")
        expect(page.locator(".blocco")).to_have_count(3)
        expect(page.locator(".katex").first).to_be_visible()
        expect(page.locator(".tabella-markdown").first).to_be_visible()
        page.locator("#apri-avanzate").click()
        expect(page.locator("#tempi details")).to_be_visible()
        assert page.locator("#tempi > .riga-tempo").is_visible()
        assert not page.locator("#tempi details .riga-tempo").first.is_visible()
        page.locator("#pannello-avanzate").evaluate("e => e.scrollTop = e.scrollHeight")
        page.screenshot(path=str(OUT / "12-tempi.png"))
        page.locator("#pannello-avanzate").evaluate("e => e.scrollTop = 0")
        page.locator('[data-carattere="lettura"]').click()
        page.keyboard.press("Escape")
        page.evaluate("document.activeElement.blur()")
        page.screenshot(path=str(OUT / "03-confronto.png"))
        page.locator('[data-vista="testo"]').click()
        expect(page.locator("#colonna-documento")).to_be_hidden()
        expect(page.locator("#aggiungi-altri")).to_be_visible()
        page.screenshot(path=str(OUT / "04-lettura.png"))
        page.locator('[data-vista="confronto"]').click()
        expect(page.locator("#colonna-documento")).to_be_visible()
        # Finita la lettura, "Copia" prende il colore d'accento e le uscite si vedono.
        assert page.evaluate("document.body.classList.contains('esportabile')")
        piede = page.locator(".pie").bounding_box()
        avanzamento = page.locator(".progresso").bounding_box()
        assert avanzamento["width"] > piede["width"] * 0.45, (avanzamento, piede)
        expect(page.locator("#barra-progresso")).to_have_class("barra-progresso")
        # La spiegazione sta sotto la pillola e non si sposta col puntatore.
        spiegazione = page.locator("#spiegazione-dizionario")
        chip = page.locator("#chip-dizionario").bounding_box()
        page.mouse.move(chip["x"] + 8, chip["y"] + 6)
        expect(spiegazione).to_be_visible()
        expect(spiegazione).to_contain_text("dizionario italiano")
        page.screenshot(path=str(OUT / "11-spiegazione.png"))
        prima = spiegazione.bounding_box()
        page.mouse.move(chip["x"] + chip["width"] - 6, chip["y"] + chip["height"] - 4)
        assert spiegazione.bounding_box() == prima, (spiegazione.bounding_box(), prima)
        page.mouse.move(0, 0)
        expect(spiegazione).to_be_hidden()
        sfondi = [page.locator(s).evaluate("e => getComputedStyle(e).backgroundColor")
                  for s in ("#copia", "#apri-esporta")]
        assert sfondi[0] != sfondi[1], sfondi
        expect(page.locator("#voci-esporta")).to_be_hidden()
        page.locator("#apri-esporta").click()
        expect(page.locator("#voci-esporta")).to_be_visible()
        expect(page.locator("#esporta-docx")).to_be_focused()
        page.screenshot(path=str(OUT / "10-menu-esporta.png"))
        page.keyboard.press("Escape")
        expect(page.locator("#voci-esporta")).to_be_hidden()
        expect(page.locator("#apri-esporta")).to_be_focused()
        page.locator("#apri-esporta").click()
        page.locator("#esporta-txt").click()
        expect(page.locator("#voci-esporta")).to_be_hidden()
        page.locator("#apri-esporta").click()
        page.locator("#esporta-md").click()
        page.locator("#apri-esporta").click()
        page.locator("#esporta-docx").click()
        salvati = page.evaluate("window.__saved")
        assert len(salvati) == 3, salvati
        assert salvati[2]["percorso"].endswith(".docx"), salvati[2]
        assert "contenuto" not in salvati[2], salvati[2]
        page.locator("#copia").click()
        expect(page.locator("#brindisi")).to_contain_text("copiat", ignore_case=True)
        page.locator("#brindisi").click()
        # Se un secondo avviso arriva nei 240 ms dell'animazione di chiusura,
        # il vecchio timer non deve nascondere anche quello nuovo.
        page.evaluate("avvisa('Avviso sostitutivo')")
        page.wait_for_timeout(300)
        expect(page.locator("#brindisi")).to_be_visible()
        expect(page.locator("#brindisi")).to_contain_text("Avviso sostitutivo")
        page.locator("#brindisi").click()

        expect(page.locator(".avviso-modello")).to_be_visible()

        # "Informazioni": autore, avvertenza sull'IA, trattamento dei dati, repo.
        page.locator("#apri-informazioni").click()
        info = page.locator("#informazioni")
        expect(info).to_be_visible()
        expect(info).to_contain_text("ITA-OCR")
        expect(info).to_contain_text("intelligenza artificiale")
        expect(info).to_contain_text("non manda niente su internet")
        expect(page.locator("#indirizzo-repo")).to_have_text("https://github.com/GioOtto/ITA-OCR")
        cassetto = page.locator(".contenuto-cassetto").bounding_box()
        riquadro = info.bounding_box()
        assert riquadro["y"] < cassetto["y"] + cassetto["height"], (riquadro, cassetto)
        page.screenshot(path=str(OUT / "13-informazioni.png"))
        # L'indirizzo e' un collegamento: cliccarlo apre la pagina nel browser.
        chiamate = page.evaluate("window.__calls.length")
        page.locator("#indirizzo-repo").click()
        assert any(c["name"] == "apri_repository"
                   for c in page.evaluate("window.__calls")[chiamate:])
        page.locator("#copia-repo").click()
        expect(page.locator("#brindisi")).to_contain_text("Indirizzo copiato")
        page.locator("#brindisi").click()
        page.keyboard.press("Escape")
        expect(page.locator("#pannello-avanzate")).to_be_hidden()

        page.locator('#scelta-tema [data-tema="scuro"]').click()
        page.screenshot(path=str(OUT / "05-scuro.png"))
        page.set_viewport_size({"width": 940, "height": 620})
        page.screenshot(path=str(OUT / "06-compatto.png"))
        for selector in ["#copia", "#apri-esporta", "#pagina-successiva", "#scelta-vista", "#correzione-automatica"]:
            box = page.locator(selector).bounding_box()
            assert box and box["x"] >= 0 and box["x"] + box["width"] <= 940, (selector, box)
            assert box["y"] + box["height"] <= 620, (selector, box)
        assert page.evaluate("document.documentElement.scrollWidth <= innerWidth")
        page.locator("#nuovo-ocr").click()
        expect(page.locator("#scegli-file")).to_be_focused()
        expect(page.locator("#zona-vuota")).to_be_visible()
        page.locator('#scelta-tema [data-tema="chiaro"]').click()
        page.screenshot(path=str(OUT / "07-home-compatta.png"))
        page.set_viewport_size({"width": 1380, "height": 920})
        page.locator(".apri-voce").first.click()
        expect(page.locator("#nome-sessione")).to_have_text("Appunti di matematica")
        page.locator("#pagina-successiva").click()
        expect(page.locator("#posizione-pagina")).to_have_text("2 / 3")
        # Delete has its own native button: Enter must not open the document.
        opens = len([c for c in page.evaluate("window.__calls") if c["name"] == "archivio_carica"])
        delete = page.locator('.comandi-voce button').last
        delete.focus()
        page.keyboard.press("Enter")
        expect(page.locator(".apri-voce")).to_have_count(2)
        assert len([c for c in page.evaluate("window.__calls") if c["name"] == "archivio_carica"]) == opens
        page.locator("#mostra-fianco").click()
        assert page.locator("#fianco").evaluate("e => e.inert")
        page.reload()
        expect(page.locator("#mostra-fianco")).to_have_attribute("aria-expanded", "false")
        expect(page.locator("html")).to_have_attribute("data-tema", "chiaro")
        # Anche con il sistema impostato scuro si parte dal tema chiaro.
        contesto_scuro = browser.new_context(
            viewport={"width": 1100, "height": 800}, color_scheme="dark", reduced_motion="reduce"
        )
        contesto_scuro.add_init_script("window.__preview = " + json.dumps(preview) + ";" + BRIDGE)
        pagina_scura = contesto_scuro.new_page()
        pagina_scura.on("pageerror", lambda error: errors.append(str(error)))
        pagina_scura.goto(f"http://127.0.0.1:{server.server_port}")
        expect(pagina_scura.locator("html")).to_have_attribute("data-tema", "chiaro")
        sfondo = pagina_scura.evaluate("getComputedStyle(document.body).backgroundColor")
        assert min(int(n) for n in re.findall(r"\d+", sfondo)[:3]) > 200, sfondo
        contesto_scuro.close()

        assert not errors, errors
        browser.close()
    server.shutdown()
    print("PASS: importazione, scorciatoia Ctrl+O, archivio/ricerca/scorrimento, focus modale, pagine, divisore, errore avvio, annullamento, formule/tabelle, viste, esportazione, copia, avviso sul modello, informazioni, temi e 940x620.")
    print(f"Schermate: {OUT}")


if __name__ == "__main__":
    run()
