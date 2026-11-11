"""Russian intelligibility of rendered WAV's, scored by Whisper.

Not the English hill-climb judge (`speech_wer.py`, model small.en). This one transcribes
with the multilingual `small` model, language=ru, greedy, and reports word and character
error rates. It does not render and it does not decide a tuning loop: a singer can pass
this and still sound wrong, and a change that only flatters the recogniser is not a win.

usage: russian_wer.py <manifest.tsv> [--language ru|uk] [--out report.json]

Each manifest line is `label<TAB>wav<TAB>reference`. Blank lines and lines starting with
# are skipped. ё is folded to е for the headline rates, because Whisper writes е; the
unfolded rates are printed beside them.

Set up once with tools/asr/setup.sh. The multilingual model downloads into
~/.cache/chimera-asr/models on first use.
"""

import json
import os
import re
import sys
import wave

import numpy as np
from faster_whisper import WhisperModel

CACHE = os.path.expanduser("~/.cache/chimera-asr/models")


def fold(text):
    text = text.lower().replace("ё", "е")
    # є і ї ґ sit outside the а–я block. Dropping them would erase the Ukrainian vowels.
    text = re.sub(r"[^а-яєіїґa-z0-9 ]+", " ", text)
    return " ".join(text.split())


def edits(ref, hyp):
    d = list(range(len(hyp) + 1))
    for i, r in enumerate(ref, 1):
        prev, d[0] = d[0], i
        for j, h in enumerate(hyp, 1):
            prev, d[j] = d[j], min(d[j] + 1, d[j - 1] + 1, prev + (r != h))
    return d[len(hyp)]


def load16k(path):
    with wave.open(path) as w:
        ch, rate = w.getnchannels(), w.getframerate()
        x = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768.0
    x = x.reshape(-1, ch).mean(axis=1)
    n = int(round(len(x) * 16000 / rate))
    return (np.fft.irfft(np.fft.rfft(x)[: n // 2 + 1], n) * (n / len(x))).astype(np.float32)


def main():
    args = sys.argv[1:]
    out = None
    language = "ru"
    if "--out" in args:
        i = args.index("--out")
        out = args[i + 1]
        del args[i : i + 2]
    if "--language" in args:
        i = args.index("--language")
        language = args[i + 1]
        del args[i : i + 2]
    manifest = args[0]
    rows = []
    for line in open(manifest):
        line = line.rstrip("\n")
        if not line or line.startswith("#"):
            continue
        label, wav, ref = line.split("\t", 2)
        rows.append((label, wav, ref))
    model = WhisperModel(
        "small", device="cpu", compute_type="int8", cpu_threads=os.cpu_count(), download_root=CACHE
    )
    scored = []
    we = ce = ww = cc = 0
    for label, wav, ref in rows:
        segs, _ = model.transcribe(
            load16k(wav),
            language=language,
            beam_size=1,
            temperature=0.0,
            condition_on_previous_text=False,
            vad_filter=False,
        )
        heard = " ".join(s.text.strip() for s in segs)
        rw, hw = fold(ref).split(), fold(heard).split()
        rc, hc = list("".join(rw)), list("".join(hw))
        e_w, e_c = edits(rw, hw), edits(rc, hc)
        we += e_w
        ce += e_c
        ww += len(rw)
        cc += len(rc)
        scored.append(
            {
                "label": label,
                "ref": ref,
                "heard": heard,
                "word_errors": e_w,
                "words": len(rw),
                "char_errors": e_c,
                "chars": len(rc),
            }
        )
        print(f"{label}\t{e_w}/{len(rw)} words\t{e_c}/{len(rc)} chars\t{heard}")
    wer = round(100.0 * we / ww, 2) if ww else None
    cer = round(100.0 * ce / cc, 2) if cc else None
    print(f"word error rate {wer}%  ({we}/{ww})")
    print(f"char error rate {cer}%  ({ce}/{cc})")
    if out:
        with open(out, "w") as f:
            json.dump({"wer": wer, "cer": cer, "items": scored}, f, ensure_ascii=False, indent=1)
            f.write("\n")


main()
