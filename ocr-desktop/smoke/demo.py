"""Pagina sintetica per test e schermate: nessun documento reale o dataset."""
import io
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
