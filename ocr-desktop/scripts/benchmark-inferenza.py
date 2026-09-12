"""Confronto locale di thread e riuso del prompt sul motore distribuito.

Richiede Pillow. Usa una porta temporanea e termina soltanto il proprio server.
Il file immagine viene adattato al canvas 960x1248; non misura CER/WER.
"""

import argparse
import base64
import hashlib
import io
import json
import os
import re
import statistics
from pathlib import Path
import socket
import subprocess
import time
import urllib.error
import urllib.request

from PIL import Image


def richiesta(porta, endpoint, corpo=None):
    inizio = time.monotonic()
    dati = None if corpo is None else json.dumps(corpo).encode()
    req = urllib.request.Request(
        f"http://127.0.0.1:{porta}/{endpoint}", data=dati,
        headers={"Content-Type": "application/json"},
    )
    # Il server e' locale: ignora eventuali proxy dell'ambiente.
    with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(
        req, timeout=600 if corpo is not None else 2
    ) as risposta:
        if corpo and corpo.get("stream"):
            testo, istanti = [], []
            finale = None
            primo_testo = None
            for riga in risposta:
                if not riga.startswith(b"data:"):
                    continue
                evento = json.loads(riga[5:])
                if "error" in evento:
                    raise RuntimeError(evento["error"])
                adesso = time.monotonic() - inizio
                if evento.get("tokens"):
                    istanti.append(adesso)
                if evento.get("content"):
                    testo.append(evento["content"])
                    if primo_testo is None:
                        primo_testo = adesso
                if evento.get("stop"):
                    finale = evento
                    break
            if finale is None:
                raise RuntimeError("Streaming terminato senza evento finale")
            intervalli = sorted((b - a) * 1000 for a, b in zip(istanti, istanti[1:]))
            finale["content"] = "".join(testo)
            finale["stream_metrics"] = {
                "primo_token_secondi": istanti[0] if istanti else None,
                "primo_testo_secondi": primo_testo,
                "eventi_token": len(istanti),
                "intervallo_eventi_p50_ms": statistics.median(intervalli) if intervalli else None,
                "intervallo_eventi_p95_ms": intervalli[min(len(intervalli)-1, int(len(intervalli)*0.95))] if intervalli else None,
                "intervallo_eventi_max_ms": max(intervalli) if intervalli else None,
            }
            return finale
        return json.load(risposta)


def interrompi_greedy(porta, corpo):
    """Prepara un retry chiudendo lo streaming, come l'abort della cascata."""
    corpo = dict(corpo, stream=True, cache_prompt=False, n_predict=4096)
    for chiave in ("repeat_penalty", "frequency_penalty", "presence_penalty", "min_p"):
        corpo.pop(chiave, None)
    req = urllib.request.Request(
        f"http://127.0.0.1:{porta}/completion", data=json.dumps(corpo).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(req, timeout=600) as risposta:
        token = 0
        for riga in risposta:
            if not riga.startswith(b"data:"):
                continue
            evento = json.loads(riga[5:])
            if "error" in evento:
                raise RuntimeError(evento["error"])
            token += len(evento.get("tokens", []))
            if token >= 32 or evento.get("stop"):
                break
    scadenza = time.monotonic() + 60
    while any(s["is_processing"] for s in richiesta(porta, "slots")):
        if time.monotonic() > scadenza:
            raise TimeoutError("Slot occupata dopo l'interruzione")
        time.sleep(0.05)


def misura(args, thread, immagine):
    with socket.socket() as ascolto:
        ascolto.bind(("127.0.0.1", 0))
        porta = ascolto.getsockname()[1]
    comando = [
        str(args.server.resolve()), "-m", str(args.modello.resolve()),
        "--mmproj", str(args.mmproj.resolve()), "-c", "8192", "-np", "1",
        "-t", str(thread), "-tb", str(args.thread_batch or thread),
        "--no-cache-prompt", "--cache-ram", "0", "--no-webui",
        "--host", "127.0.0.1", "--port", str(porta),
        "--override-kv", f"tokenizer.ggml.eot_token_id=int:{args.eot}",
    ]
    ambiente = os.environ.copy()
    librerie = "PATH" if os.name == "nt" else "LD_LIBRARY_PATH"
    ambiente[librerie] = str(args.server.resolve().parent) + os.pathsep + ambiente.get(librerie, "")
    if args.thread_vision is not None:
        ambiente["MTMD_N_THREADS"] = str(args.thread_vision)
    comando += ["--flash-attn", args.flash_attn]
    if args.batch:
        comando += ["-b", str(args.batch)]
    if args.ubatch:
        comando += ["-ub", str(args.ubatch)]
    if args.device == "none":
        comando += ["-ngl", "0", "--device", "none", "--no-mmproj-offload"]
    else:
        comando += ["-ngl", "999", "--device", args.device]
        ambiente["MTMD_BACKEND_DEVICE"] = args.device
    log = args.output.with_name(f"{args.output.stem}-t{thread}.log")
    with log.open("w", encoding="utf-8") as uscita:
        processo = subprocess.Popen(
            comando, env=ambiente, stdout=uscita, stderr=subprocess.STDOUT,
            stdin=subprocess.DEVNULL,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        try:
            if args.affinita is not None:
                if os.name == "nt":
                    import ctypes
                    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
                    kernel.SetProcessAffinityMask.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
                    kernel.SetProcessAffinityMask.restype = ctypes.c_int
                    if not kernel.SetProcessAffinityMask(int(processo._handle), args.affinita):
                        raise ctypes.WinError(ctypes.get_last_error())
                else:
                    os.sched_setaffinity(processo.pid, {i for i in range(args.affinita.bit_length()) if args.affinita & (1 << i)})
            scadenza = time.monotonic() + 180
            while True:
                if processo.poll() is not None:
                    raise RuntimeError(f"Server terminato: vedere {log}")
                try:
                    if richiesta(porta, "health").get("status") == "ok":
                        break
                except (OSError, urllib.error.URLError):
                    pass
                if time.monotonic() >= scadenza:
                    raise TimeoutError(f"Avvio scaduto: vedere {log}")
                time.sleep(0.2)
            marker = richiesta(porta, "props")["media_marker"]
            corpo = {
                "prompt": {
                    "prompt_string": f"[gMASK]<sop><|user|>\n{marker}Text Recognition:<|assistant|>\n",
                    "multimodal_data": [immagine],
                },
                "n_predict": args.token, "temperature": 0, "seed": 0,
                "cache_prompt": False, "return_tokens": True, "stream": args.stream,
            }
            # Riscaldamento escluso dai risultati; ogni coppia successiva
            # confronta la stessa penalita' con e senza cache del prompt.
            richiesta(porta, "completion", corpo)
            if args.retry:
                corpo.update(repeat_penalty=1.0, frequency_penalty=0.35,
                             presence_penalty=0.20, min_p=0.05)
            righe = []
            for ripetizione in range(args.ripetizioni):
                riferimento = None
                for cache in ((False,) if args.senza_cache else (False, True)):
                    if cache and args.retry:
                        interrompi_greedy(porta, corpo)
                    corpo["cache_prompt"] = cache
                    offset_log = log.stat().st_size
                    inizio = time.monotonic()
                    esito = richiesta(porta, "completion", corpo)
                    if "error" in esito:
                        raise RuntimeError(esito["error"])
                    testo = esito.get("content", "")
                    with log.open("rb") as lettura_log:
                        lettura_log.seek(offset_log)
                        log_richiesta = lettura_log.read().decode("utf-8", errors="replace")
                    vision = [float(v) for v in re.findall(r"vision encode = ([\d.]+) ms", log_richiesta)]
                    prefill = [float(v) for v in re.findall(r"image prefill = ([\d.]+) ms", log_richiesta)]
                    if not cache:
                        riferimento = testo
                    riga = {
                        "device": args.device, "thread": thread,
                        "thread_batch": args.thread_batch or thread,
                        "thread_vision": args.thread_vision or thread,
                        "ripetizione": ripetizione + 1, "cache_prompt": cache,
                        "retry_dopo_abort": args.retry and cache,
                        "secondi": time.monotonic() - inizio,
                        "timings": esito.get("timings"),
                        "stream": esito.get("stream_metrics"),
                        "vision_encode_ms": sum(vision) if vision else None,
                        "image_prefill_ms": sum(prefill) if prefill else None,
                        "tokens_cached": esito.get("tokens_cached"),
                        "stop_type": esito.get("stop_type"),
                        "testo_sha256": hashlib.sha256(testo.encode()).hexdigest(),
                        "testo_uguale_senza_cache": testo == riferimento,
                    }
                    righe.append(riga)
                    print(json.dumps(riga), flush=True)
            return {"comando": comando, "affinita": args.affinita, "misure": righe}
        finally:
            processo.terminate()
            try:
                processo.wait(timeout=15)
            except subprocess.TimeoutExpired:
                processo.kill()
                processo.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("immagine", type=Path)
    parser.add_argument("--server", type=Path, required=True)
    parser.add_argument("--modello", type=Path, required=True)
    parser.add_argument("--mmproj", type=Path, required=True)
    parser.add_argument("--device", default="none", help="none, Vulkan0, CUDA0, ...")
    parser.add_argument("--thread", type=int, nargs="+")
    parser.add_argument("--thread-batch", type=int)
    parser.add_argument("--thread-vision", type=int, help="Richiede la patch del motore ITA-OCR")
    parser.add_argument("--flash-attn", choices=["auto", "on", "off"], default="auto")
    parser.add_argument("--batch", type=int)
    parser.add_argument("--ubatch", type=int)
    parser.add_argument("--stream", action="store_true")
    parser.add_argument("--senza-cache", action="store_true", help="Misura soltanto richieste senza riuso")
    parser.add_argument("--affinita", type=lambda s: int(s, 0), help="Maschera CPU del solo server, es. 0xffff; adattare alla topologia")
    parser.add_argument("--token", type=int, default=128)
    parser.add_argument("--ripetizioni", type=int, default=3)
    parser.add_argument("--retry", action="store_true",
                        help="Confronta il primo stadio con penalita', riusando un greedy interrotto a 32 token")
    parser.add_argument("--eot", type=int, default=59246,
                        help="Lo stop ufficiale diverso dall'EOS del GGUF")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.affinita is not None and (args.affinita <= 0 or (os.name != "nt" and not hasattr(os, "sched_setaffinity"))):
        parser.error("affinita non valida o non supportata dal sistema")
    if args.thread is None:
        disponibili = os.cpu_count() or 1
        args.thread = list(dict.fromkeys([
            max(4, min(32, disponibili - 2)), min(8, disponibili), min(4, disponibili),
        ]))
    if args.token < 1 or args.ripetizioni < 1 or any(t < 1 for t in args.thread):
        parser.error("token, ripetizioni e thread devono essere positivi")
    for nome in ("thread_batch", "thread_vision", "batch", "ubatch"):
        valore = getattr(args, nome)
        if valore is not None and valore < 1:
            parser.error(f"{nome} deve essere positivo")
    with Image.open(args.immagine) as originale:
        originale = originale.convert("RGB")
        scala = min(960 / originale.width, 1248 / originale.height)
        ridotta = originale.resize(
            (max(1, int(originale.width * scala + 0.5)),
             max(1, int(originale.height * scala + 0.5))), Image.Resampling.LANCZOS,
        )
    canvas = Image.new("RGB", (960, 1248), "white")
    canvas.paste(ridotta, ((960 - ridotta.width) // 2, (1248 - ridotta.height) // 2))
    png = io.BytesIO()
    canvas.save(png, format="PNG")
    immagine = base64.b64encode(png.getvalue()).decode("ascii")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    risultato = {"immagine_sha256": hashlib.sha256(png.getvalue()).hexdigest(),
                 "token_massimi": args.token, "profili": []}
    for thread in args.thread:
        risultato["profili"].append(misura(args, thread, immagine))
        args.output.write_text(json.dumps(risultato, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
