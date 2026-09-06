# -*- coding: utf-8 -*-
r"""Genera il set di icone di ITA-OCR da marchio/logo.png.

    python genera-icone.py            scrive in app/src-tauri/icons
    python genera-icone.py --verifica non scrive, dice solo se e' aggiornato

Dipendenze: Python 3, Pillow e numpy. Installarle con pip install Pillow numpy.

build.rs segue la directory icons: dopo la generazione basta cargo build
--release, senza pulire la cache di Cargo.

Tre cose non ovvie che questo script fa apposta, e che vanno tenute se
qualcuno lo riscrive.

1. Il sorgente e' opaco e non pulito: il fondo sta a 254 invece che a 255 e
   l'inchiostro a 9 invece che a 0. La copertura si ricava dalla luminanza
   (1 - g/255) e poi si stira con una correzione di livelli, altrimenti
   restano alpha 1-5 sul 43% della tela -- una velatura invisibile a occhio
   ma che tinge il quadrato sui fondi colorati -- e il marchio non arriva mai
   a essere opaco davvero.

2. Tutte le risoluzioni conservano il marchio completo, compresi firma e
   testo interno. Eliminare quelle parti nelle misure piccole faceva
   apparire un foglio vuoto nell'elenco di Explorer e nella barra del titolo,
   mentre durante il trascinamento compariva un logo diverso e completo.
   Alle misure minori l'antialias conserva i dettagli come trama visiva:
   le scritte interne non possono essere leggibili a 16px.

3. Il .ico usa DIB sotto i 256px e PNG a 256px, come raccomandato dalle
   linee guida Microsoft. E' una scelta di compatibilita', non un limite
   generale del formato PNG o di Pillow.

Le icone dell'app hanno un fondo bianco arrotondato: la shell non cambia
il colore del marchio quando Esplora file usa il tema scuro. I marchi
chiari/scuri per l'interfaccia restano invece trasparenti.
"""
import hashlib
import io
import os
import struct
import sys

import numpy as np
from PIL import Image, ImageDraw

QUI = os.path.dirname(os.path.abspath(__file__))
SORG = os.path.join(QUI, "marchio", "logo.png")
DEST = os.path.join(QUI, "..", "app", "src-tauri", "icons")

MARG = 0.055              # margine attorno al disegno, frazione del lato
FONDO = 0.03              # sotto questa copertura e' fondo: alpha a zero
INCHIOSTRO = 0.95         # sopra questa copertura e' inchiostro pieno

# misura -> gamma: <1 ispessisce leggermente i tratti sottili
PIANO = {
    16: .90, 20: .90, 24: .90, 32: .90, 48: .92, 64: .92,
    128: .92, 256: 1., 512: 1.,
}
ICO = [16, 20, 24, 32, 48, 64, 128, 256]


def marchio():
    """Copertura del logo completo, centrato sulla stessa tela a ogni misura."""
    g = np.asarray(Image.open(SORG).convert("L")).astype(np.float32)
    cop = np.clip(1.0 - g / 255.0, 0, 1)
    cop = np.clip((cop - FONDO) / (INCHIOSTRO - FONDO), 0, 1)

    def quadra(c):
        ys, xs = np.where(c > 0.02)
        t, b, l, r = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
        lato = max(b - t, r - l)
        tela = int(round(lato / (1 - 2 * MARG)))
        o = np.zeros((tela, tela), np.float32)
        oy, ox = (tela - (b - t)) // 2, (tela - (r - l)) // 2
        o[oy:oy + (b - t), ox:ox + (r - l)] = c[t:b, l:r]
        return o

    return quadra(cop)


def alfa(Q, m):
    gain = PIANO[m]
    a = np.clip(np.asarray(Image.fromarray(Q, "F").resize((m, m), Image.LANCZOS)), 0, 1)
    return (a ** gain * 255 + .5).astype(np.uint8)


def rgba(a, chiaro=False):
    h, w = a.shape
    img = np.zeros((h, w, 4), np.uint8)
    img[..., 0:3] = 255 if chiaro else 0
    img[..., 3] = a
    return Image.fromarray(img, "RGBA")


def icona_app(a):
    """Marchio nero su supporto chiaro, leggibile con entrambi i temi."""
    m = a.shape[0]
    scala = 4
    maschera = Image.new("L", (m * scala, m * scala))
    ImageDraw.Draw(maschera).rounded_rectangle(
        (0, 0, m * scala - 1, m * scala - 1), radius=m * scala * .12, fill=255)
    fondo = Image.new("RGBA", (m, m), "white")
    fondo.putalpha(maschera.resize((m, m), Image.Resampling.LANCZOS))
    return Image.alpha_composite(fondo, rgba(a))


def _png(img):
    buf = io.BytesIO()
    img.save(buf, format="PNG", optimize=True)
    return buf.getvalue()


def _dib(img):
    """Voce d'icona non compressa: BITMAPINFOHEADER, pixel BGRA dal basso
    verso l'alto, e in coda la maschera AND a 1 bit per i pixel trasparenti."""
    px = np.asarray(img)[..., [2, 1, 0, 3]]  # RGBA -> BGRA
    h, w = px.shape[:2]
    xor = px[::-1].tobytes()
    mask = np.zeros((h, ((w + 31) // 32) * 32), np.uint8)
    mask[:, :w] = px[::-1, :, 3] == 0
    and_ = np.packbits(mask, axis=1, bitorder="big").tobytes()
    testa = struct.pack("<IiiHHIIiiII", 40, w, h * 2, 1, 32, 0,
                        len(xor) + len(and_), 0, 0, 0, 0)
    return testa + xor + and_


def contenuto_ico(A):
    dati = [(_png if m >= 256 else _dib)(icona_app(A[m])) for m in ICO]
    off = 6 + 16 * len(ICO)
    testa = struct.pack("<HHH", 0, 1, len(ICO))
    for m, d in zip(ICO, dati):
        testa += struct.pack("<BBBBHHII", m % 256, m % 256, 0, 0, 1, 32, len(d), off)
        off += len(d)
    return testa + b"".join(dati)


def main():
    verifica = "--verifica" in sys.argv
    Q = marchio()
    A = {m: alfa(Q, m) for m in PIANO}

    attesi = {"icon.ico": contenuto_ico(A)}
    # Anche il marchio nell'interfaccia deriva da questo sorgente. Il CSS
    # usa l'alpha come maschera per adattare il colore al tema chiaro/scuro.
    attesi[os.path.join("..", "..", "ui", "marchio.png")] = _png(rgba(A[256]))
    for nome, img in [("icon.png", icona_app(A[512]))] + \
                     [("%dx%d.png" % (m, m), icona_app(A[m])) for m in (32, 128, 256)]:
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        attesi[nome] = buf.getvalue()
    mar = os.path.join(DEST, "marchio")
    for m in (64, 128, 256, 512):
        for tinta, chiaro in (("scuro", False), ("chiaro", True)):
            buf = io.BytesIO()
            rgba(A[m], chiaro).save(buf, format="PNG")
            attesi[os.path.join("marchio", "marchio-%s-%d.png" % (tinta, m))] = buf.getvalue()

    if verifica:
        diversi = []
        for nome, atteso in sorted(attesi.items()):
            p = os.path.join(DEST, nome)
            uguale = os.path.exists(p) and open(p, "rb").read() == atteso
            print("  %-32s %s" % (nome.replace("\\", "/"), "ok" if uguale else "DA RIGENERARE"))
            if not uguale:
                diversi.append(nome)
        return 1 if diversi else 0

    os.makedirs(mar, exist_ok=True)
    for nome, dati in sorted(attesi.items()):
        p = os.path.join(DEST, nome)
        with open(p, "wb") as f:
            f.write(dati)
        print("  %-32s %7d B  %s" % (nome.replace("\\", "/"), len(dati),
                                     hashlib.sha256(dati).hexdigest()[:12]))
    print("\nIcone aggiornate. Ricompila l'app e l'installer.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
