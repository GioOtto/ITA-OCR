"""Pagine sintetiche per test e schermate: nessun documento reale o dataset."""
import io
import random
from PIL import Image, ImageDraw, ImageFont


def pagina_demo():
    image = Image.new("RGB", (960, 1248), "#fffef9")
    draw = ImageDraw.Draw(image)
    title = ImageFont.load_default(size=38)
    heading = ImageFont.load_default(size=28)
    body = ImageFont.load_default(size=23)
    draw.line((65, 125, 895, 125), fill="#52745f", width=3)
    draw.text((65, 62), "APPUNTI DI MATEMATICA", font=title, fill="#233d30")
    draw.text((65, 147), "Esempio sintetico per la dimostrazione", font=body, fill="#778076")
    for y in range(237, 1150, 43):
        draw.line((60, y, 900, y), fill="#e4e8dc", width=1)
    draw.text((65, 209), "Il concetto di funzione", font=heading, fill="#233d30")
    lines = [
        "Una funzione associa a ogni elemento del dominio",
        "uno e un solo elemento del codominio.",
        "La relazione puo essere descritta con una formula,",
        "una tabella o un grafico.",
        "", "Consideriamo la funzione:",
        "f(x) = x^2 + 2x + 1", "",
        "Raccogliendo il quadrato di un binomio:",
        "f(x) = (x + 1)^2", "",
        "Il vertice si trova nel punto V = (-1, 0).",
        "La parabola ha concavita rivolta verso l'alto.",
    ]
    for n, line in enumerate(lines):
        draw.text((65, 254 + n * 43), line, font=body, fill="#39463e")
    draw.text((65, 925), "Osservazione", font=heading, fill="#233d30")
    draw.text((65, 973), "Si individuano dominio, segno e intersezioni con gli assi.", font=body, fill="#39463e")
    draw.text((65, 1180), "CONTENUTO SINTETICO - NON E UN RISULTATO DI BENCHMARK", font=ImageFont.load_default(size=16), fill="#778076")
    out = io.BytesIO()
    image.save(out, format="PNG")
    return out.getvalue()


# Font corsivi di sistema, in ordine di preferenza. Se non ce n'e' nessuno la
# pagina si disegna lo stesso, solo in stampatello: serve a non far fallire le
# schermate su una macchina che non ha questi file.
CORSIVI = (
    "C:/Windows/Fonts/Inkfree.ttf",
    "C:/Windows/Fonts/segoepr.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
)


def _corsivo(dimensione):
    for percorso in CORSIVI:
        try:
            return ImageFont.truetype(percorso, dimensione)
        except OSError:
            continue
    return ImageFont.load_default(size=dimensione)


def pagina_manoscritta():
    """Un foglio a righe scritto a mano: contenuto inventato, nessun corpus.

    Le righe sono sfalsate di pochi pixel con un seme fisso, cosi' la pagina
    sembra scritta a mano e resta identica a ogni esecuzione: una schermata che
    cambia a ogni build non si potrebbe rivedere in una pull request.
    """
    rng = random.Random(7)
    image = Image.new("RGB", (960, 1248), "#fdfcf6")
    draw = ImageDraw.Draw(image)
    titolo = _corsivo(46)
    mano = _corsivo(34)

    # Foglio a righe con il margine, come un quaderno.
    for y in range(210, 1180, 46):
        draw.line((70, y, 900, y), fill="#dbe3ea", width=1)
    draw.line((132, 90, 132, 1190), fill="#e8c9c9", width=2)

    draw.text((150, 96), "Fisica - moto uniformemente accelerato", font=titolo, fill="#26324a")
    draw.line((150, 158, 806, 161), fill="#26324a", width=2)

    righe = [
        "Un corpo che parte da fermo e accelera in modo",
        "costante percorre uno spazio proporzionale al",
        "quadrato del tempo.",
        "",
        "        s(t) = s0 + v0 t + \u00bd a t\u00b2",
        "",
        "Se v0 = 0 la legge si riduce a s = \u00bd a t\u00b2.",
        "",
        "Esempio: con a = 2 m/s\u00b2 e t = 3 s si ottiene",
        "s = 9 m.",
        "",
        "Nota: la velocita cresce in modo lineare,",
        "lo spazio cresce come t\u00b2.",
    ]
    for n, riga in enumerate(righe):
        if not riga:
            continue
        x = 150 + rng.randint(-3, 3)
        y = 214 + n * 46 + rng.randint(-4, 2)
        draw.text((x, y), riga, font=mano, fill="#1f3350")

    draw.text((150, 1196), "CONTENUTO SINTETICO - NON E UN DOCUMENTO REALE",
              font=ImageFont.load_default(size=16), fill="#8b8f96")
    out = io.BytesIO()
    image.save(out, format="PNG")
    return out.getvalue()


# Il testo che l'OCR restituirebbe da quella pagina, usato dalle schermate.
TRASCRIZIONE_MANOSCRITTA = (
    "Fisica \u2014 moto uniformemente accelerato\n\n"
    "Un corpo che parte da fermo e accelera in modo costante percorre uno "
    "spazio proporzionale al quadrato del tempo.\n\n"
    "$$s(t) = s_0 + v_0 t + \\tfrac{1}{2} a t^2$$\n\n"
    "Se $v_0 = 0$ la legge si riduce a $s = \\tfrac{1}{2} a t^2$.\n\n"
    "Esempio: con $a = 2\\,\\mathrm{m/s^2}$ e $t = 3\\,\\mathrm{s}$ si ottiene "
    "$s = 9\\,\\mathrm{m}$.\n\n"
    "Nota: la velocit\u00e0 cresce in modo lineare, lo spazio cresce come $t^2$."
)
