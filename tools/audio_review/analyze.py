#!/usr/bin/env python3
"""Measure a soundtrack review rendered by `brinewake --soundtrack-review DIR`.

Writes DIR/report.md with, for every WAV: integrated loudness (EBU R128),
loudness range, true peak, crest factor, spectral balance by band, spectral
centroid, stereo width, a 5-second short-term loudness profile, and a count
of discontinuities that would be heard as clicks. For the effects gallery it
cuts each cue out using sfx-gallery.json and tabulates them side by side.
Spectrograms (log frequency) go to DIR/spectra/*.png.

These are measurements, not a listening test.
"""

import json
import os
import re
import subprocess
import sys
import wave

import numpy as np

BANDS = [(20, 60, "sub"), (60, 250, "low"), (250, 1000, "low-mid"), (1000, 4000, "mid"),
         (4000, 10000, "high"), (10000, 20000, "air")]


def load(path):
    with wave.open(path, "rb") as w:
        n = w.getnframes()
        ch = w.getnchannels()
        sr = w.getframerate()
        data = np.frombuffer(w.readframes(n), dtype=np.int16).astype(np.float32) / 32768.0
    return data.reshape(-1, ch), sr


def ebur128(path):
    out = subprocess.run(
        ["ffmpeg", "-hide_banner", "-nostats", "-i", path, "-af", "ebur128=peak=true", "-f", "null", "-"],
        capture_output=True, text=True).stderr
    summary = out[out.rfind("Summary:"):]
    def grab(label):
        m = re.search(label + r":\s+(-?[\d.]+|-inf)", summary)
        return float(m.group(1)) if m and m.group(1) != "-inf" else float("-inf")
    return grab("I"), grab("LRA"), grab("Peak")


def db(x):
    return 20 * np.log10(max(float(x), 1e-9))


def band_balance(mono, sr):
    n = 1 << 15
    if len(mono) < n:
        mono = np.pad(mono, (0, n - len(mono)))
    hops = range(0, len(mono) - n + 1, n // 2)
    spec = np.zeros(n // 2 + 1)
    win = np.hanning(n)
    count = 0
    for i in hops:
        spec += np.abs(np.fft.rfft(mono[i:i + n] * win)) ** 2
        count += 1
    spec /= max(count, 1)
    freqs = np.fft.rfftfreq(n, 1 / sr)
    total = spec[(freqs >= 20) & (freqs <= 20000)].sum() + 1e-20
    bands = {}
    for lo, hi, name in BANDS:
        e = spec[(freqs >= lo) & (freqs < hi)].sum()
        bands[name] = 10 * np.log10(e / total + 1e-12)
    centroid = float((freqs * spec).sum() / (spec.sum() + 1e-20))
    return bands, centroid


def clicks(mono, sr):
    """Sample steps far beyond the local slope: a crude click finder."""
    d = np.abs(np.diff(mono))
    d2 = np.abs(np.diff(mono, 2))
    local = np.convolve(d, np.ones(64) / 64, mode="same")[1:]
    suspicious = (d2 > 0.08) & (d2 > 12 * (local + 1e-4))
    return int(suspicious.sum())


def short_term(mono, sr, step=5.0):
    hop = int(sr * step)
    out = []
    for i in range(0, len(mono), hop):
        seg = mono[i:i + hop]
        if len(seg) < hop // 4:
            break
        out.append(round(db(np.sqrt(np.mean(seg ** 2))), 1))
    return out


def width(x, sr):
    """Side minus mid above 300 Hz, where width is heard."""
    if x.shape[1] < 2:
        return 0.0
    mid = 0.5 * (x[:, 0] + x[:, 1])
    side = 0.5 * (x[:, 0] - x[:, 1])
    def above(sig):
        spec = np.fft.rfft(sig[: min(len(sig), sr * 60)])
        freqs = np.fft.rfftfreq(min(len(sig), sr * 60), 1 / sr)
        spec[freqs < 300] = 0
        return np.sqrt(np.mean(np.fft.irfft(spec) ** 2))
    return db(above(side)) - db(above(mid))


def spectrogram(path, out):
    subprocess.run(
        ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error", "-i", path, "-lavfi",
         "showspectrumpic=s=1600x600:mode=combined:scale=log:fscale=log:color=intensity:legend=1",
         out], check=False)


def gallery(dir_, x, sr):
    sheet = json.load(open(os.path.join(dir_, "sfx-gallery.json")))
    rows = []
    for entry in sheet:
        if not entry:
            continue
        start = int(entry["ms"] * sr / 1000)
        length = max(int((entry["length_ms"] + 400) * sr / 1000), 1)
        seg = x[start:start + length]
        mono = seg.mean(axis=1)
        peak = db(np.abs(seg).max())
        rms = db(np.sqrt(np.mean(mono ** 2)))
        env = np.abs(mono)
        thresh = np.abs(mono).max() * 0.01
        above = np.nonzero(env > thresh)[0]
        audible_ms = (above[-1] - above[0]) * 1000 / sr if len(above) else 0
        bands, centroid = band_balance(mono, sr)
        rows.append((entry["cue"], peak, rms, audible_ms, centroid, bands["low"], bands["mid"], bands["high"]))
    return rows


def main(dir_):
    os.makedirs(os.path.join(dir_, "spectra"), exist_ok=True)
    lines = ["# Soundtrack review measurements", "",
             "Integrated loudness (LUFS), loudness range (LU), true peak (dBTP), crest (peak minus RMS, dB), "
             "band share (dB of total energy), centroid (Hz), stereo width (side minus mid above 300 Hz, dB), clicks (suspect "
             "discontinuities). Measurements only; nobody has listened.", "",
             "| file | LUFS | LRA | dBTP | crest | sub | low | low-mid | mid | high | air | centroid | width | clicks |",
             "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
    profiles = []
    files = sorted(f for f in os.listdir(dir_) if f.endswith(".wav"))
    gallery_rows = None
    for f in files:
        path = os.path.join(dir_, f)
        x, sr = load(path)
        mono = x.mean(axis=1)
        lufs, lra, tp = ebur128(path)
        peak = db(np.abs(x).max())
        rms = db(np.sqrt(np.mean(mono ** 2)))
        bands, centroid = band_balance(mono, sr)
        lines.append(
            f"| {f} | {lufs:.1f} | {lra:.1f} | {tp:.1f} | {peak - rms:.1f} | "
            + " | ".join(f"{bands[b[2]]:.1f}" for b in BANDS)
            + f" | {centroid:.0f} | {width(x, sr):.1f} | {clicks(mono, sr)} |")
        if f.startswith("music") or f.startswith("montage"):
            profiles.append((f, short_term(mono, sr)))
        spectrogram(path, os.path.join(dir_, "spectra", f.replace(".wav", ".png")))
        if f == "sfx-gallery.wav":
            gallery_rows = gallery(dir_, x, sr)
    lines += ["", "## Short-term level, 5-second windows (dB RMS)", ""]
    for f, p in profiles:
        lines.append(f"- {f}: " + " ".join(str(v) for v in p))
    if gallery_rows:
        lines += ["", "## Effects, one by one (full effects level)", "",
                  "| cue | peak dBFS | RMS dB | audible ms | centroid Hz | low | mid | high |",
                  "|---|---:|---:|---:|---:|---:|---:|---:|"]
        for r in gallery_rows:
            lines.append(f"| {r[0]} | {r[1]:.1f} | {r[2]:.1f} | {r[3]:.0f} | {r[4]:.0f} | {r[5]:.1f} | {r[6]:.1f} | {r[7]:.1f} |")
    open(os.path.join(dir_, "report.md"), "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main(sys.argv[1])
