#!/usr/bin/env python3
"""Band power (dB, full scale) of a WAV segment: `bands.py FILE START END`.
Bands: 20-150, 150-400, 400-1k, 1-3k, 3-8k Hz, plus the share of energy
below 150 Hz and the integrated loudness of the segment."""
import subprocess, sys, re
import numpy as np
sys.path.insert(0, __file__.rsplit('/', 1)[0])
from analyze import load

EDGES = [(20, 150), (150, 400), (400, 1000), (1000, 3000), (3000, 8000)]

def bands(path, start, end, hp200=False):
    x, sr = load(path)
    seg = x[int(start * sr):int(end * sr)].mean(axis=1)
    if hp200:
        # A laptop speaker's proxy: a 4-pole high-pass at 200 Hz.
        spec = np.fft.rfft(seg)
        f = np.fft.rfftfreq(len(seg), 1 / sr)
        spec *= 1 / np.sqrt(1 + (200 / np.maximum(f, 1)) ** 8)
        seg = np.fft.irfft(spec, len(seg))
    spec = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2
    freqs = np.fft.rfftfreq(len(seg), 1 / sr)
    total = spec[(freqs >= 20) & (freqs < 20000)].sum()
    out = [10 * np.log10(spec[(freqs >= a) & (freqs < b)].sum() / len(seg) ** 2 + 1e-20) for a, b in EDGES]
    low_share = spec[(freqs >= 20) & (freqs < 150)].sum() / total
    af = "highpass=f=200:poles=2,highpass=f=200:poles=2,ebur128" if hp200 else "ebur128"
    ff = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-ss", str(start), "-t", str(end - start), "-i", path,
                         "-af", af, "-f", "null", "-"], capture_output=True, text=True).stderr
    m = re.findall(r"I:\s+(-?[\d.]+) LUFS", ff)
    return out, low_share, float(m[-1]) if m else float("nan")

if __name__ == "__main__":
    hp = "--hp200" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--hp200"]
    b, share, lufs = bands(args[0], float(args[1]), float(args[2]), hp)
    print(args[0].split('/')[-1] + (" (laptop proxy)" if hp else ""), args[1], args[2], "LUFS %.1f" % lufs, "low<150 %.0f%%" % (100 * share),
          " ".join("%.1f" % v for v in b))
