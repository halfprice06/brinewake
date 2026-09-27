#!/usr/bin/env python3
"""Review-only diagnostics for the BRINEWAKE native art v3 atlas.

This script reads atlas PNG/JSON pairs and writes inspection artifacts.  It does
not author, resize, or rewrite project art.  Every displayed crop is taken from
its manifest rectangle and enlarged with nearest-neighbor at exactly 2x; the
input files remain untouched and no alpha smoothing is introduced.
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CURRENT_PNG = ROOT / "art/exports/game-assets-v3.png"
DEFAULT_CURRENT_JSON = ROOT / "art/exports/game-assets-v3.json"
DEFAULT_ARCHIVE_PNG = ROOT / "art/archive/art-v2/game-assets.png"
DEFAULT_ARCHIVE_JSON = ROOT / "art/archive/art-v2/game-assets.json"
DEFAULT_OUTPUT = ROOT / "output/art-v3"

UNIT_NAMES = (
    "hook",
    "riveter",
    "bulwark",
    "sounder",
    "wick",
    "skipper",
    "reedguard",
    "loom",
)
STRUCTURE_NAMES = (
    "union_hq",
    "assembly_hq",
    "union_works",
    "assembly_works",
    "dropoff",
    "condenser",
    "tower",
    "gate",
    "salvage",
    "well",
)
WALK_RE = re.compile(r"^(hook|riveter|bulwark|sounder|wick|skipper|reedguard|loom)_([0-7])_walk_([0-3])$")
FACING_RE = re.compile(r"^(hook|riveter|bulwark|sounder|wick|skipper|reedguard|loom)_([0-7])$")

SCALE = 2
BG = (27, 42, 50, 255)
PANEL = (32, 50, 59, 255)
GRID = (55, 75, 83, 255)
TEXT = (225, 234, 226, 255)
MUTED = (157, 178, 179, 255)
ACCENT_OLD = (221, 155, 120, 255)
ACCENT_NEW = (255, 216, 142, 255)


@dataclass(frozen=True)
class Atlas:
    label: str
    png_path: Path
    manifest_path: Path
    image: Image.Image
    manifest: dict[str, Any]
    entries: dict[str, dict[str, Any]]
    validation: dict[str, Any]


def font() -> ImageFont.ImageFont:
    """Return a small deterministic font available on the review host."""

    for path in (
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ):
        try:
            return ImageFont.truetype(path, 12)
        except (OSError, ValueError):
            continue
    return ImageFont.load_default()


FONT = font()


def label_for_key(key: str, meta: dict[str, Any]) -> dict[str, Any]:
    """Classify a manifest key and preserve its native dimensions for review."""

    if key in UNIT_NAMES:
        return {
            "kind": "unit_alias",
            "unit": key,
            "label": f"{key.title()} alias",
        }
    walk = WALK_RE.match(key)
    if walk:
        unit, facing, phase = walk.groups()
        return {
            "kind": "movement",
            "unit": unit,
            "facing": int(facing),
            "phase": int(phase),
            "label": f"{unit.title()} · face {facing} · phase {phase}",
        }
    facing = FACING_RE.match(key)
    if facing:
        unit, direction = facing.groups()
        return {
            "kind": "facing",
            "unit": unit,
            "facing": int(direction),
            "label": f"{unit.title()} · face {direction}",
        }
    if key in STRUCTURE_NAMES:
        return {"kind": "structure", "label": key.replace("_", " ").title()}
    return {"kind": "other", "label": key.replace("_", " ").title()}


def is_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def alpha_stats(image: Image.Image) -> dict[str, Any]:
    alpha = image.getchannel("A")
    hist = alpha.histogram()
    intermediate = sum(hist[1:255])
    opaque = hist[255]
    transparent = hist[0]
    values = [index for index, count in enumerate(hist) if count]
    return {
        "pixels": image.width * image.height,
        "transparent": transparent,
        "opaque": opaque,
        "intermediate": intermediate,
        "unique_alpha_values": values,
        "binary": intermediate == 0,
        "nonempty": alpha.getbbox() is not None,
    }


def crop_from_entry(image: Image.Image, meta: dict[str, Any]) -> Image.Image:
    x, y, w, h = (meta[field] for field in ("x", "y", "w", "h"))
    return image.crop((x, y, x + w, y + h))


def validate_atlas(label: str, png_path: Path, manifest_path: Path) -> Atlas:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    image = Image.open(png_path).convert("RGBA")
    entries = manifest.get("sprites", {})
    if not isinstance(entries, dict):
        entries = {}

    dimensions_match = [manifest.get("width"), manifest.get("height")] == [image.width, image.height]
    rect_errors: list[dict[str, Any]] = []
    anchor_errors: list[str] = []
    empty_entries: list[str] = []
    intermediate_entries: list[str] = []
    entry_stats: dict[str, dict[str, Any]] = {}

    required_rect_fields = ("x", "y", "w", "h", "anchor_x", "anchor_y")
    for key, meta in entries.items():
        if not isinstance(meta, dict):
            rect_errors.append({"key": key, "reason": "metadata is not an object"})
            continue
        missing = [field for field in required_rect_fields if field not in meta]
        if missing:
            rect_errors.append({"key": key, "reason": "missing fields", "fields": missing})
            continue
        values = {field: meta[field] for field in required_rect_fields}
        if any(not is_int(value) for value in values.values()):
            rect_errors.append({"key": key, "reason": "rectangle fields are not integer", "values": values})
            continue
        x, y, w, h = (meta[field] for field in ("x", "y", "w", "h"))
        if w <= 0 or h <= 0 or x < 0 or y < 0 or x + w > image.width or y + h > image.height:
            rect_errors.append(
                {
                    "key": key,
                    "reason": "rectangle outside PNG or non-positive",
                    "rect": [x, y, w, h],
                    "png": [image.width, image.height],
                }
            )
            continue
        ax, ay = meta["anchor_x"], meta["anchor_y"]
        if not (0 <= ax <= w and 0 <= ay <= h):
            anchor_errors.append(key)
        stats = alpha_stats(crop_from_entry(image, meta))
        entry_stats[key] = stats
        if not stats["nonempty"]:
            empty_entries.append(key)
        if not stats["binary"]:
            intermediate_entries.append(key)

    global_alpha = alpha_stats(image)
    aliases: dict[str, dict[str, Any]] = {}
    for unit in UNIT_NAMES:
        base = entries.get(unit)
        facing_zero = entries.get(f"{unit}_0")
        record: dict[str, Any] = {
            "metadata_equal": base == facing_zero if base is not None and facing_zero is not None else False,
            "pixels_equal": False,
        }
        if isinstance(base, dict) and isinstance(facing_zero, dict):
            valid_base = base.get("x", -1) >= 0 and base.get("y", -1) >= 0
            valid_zero = facing_zero.get("x", -1) >= 0 and facing_zero.get("y", -1) >= 0
            if valid_base and valid_zero:
                try:
                    first = crop_from_entry(image, base)
                    second = crop_from_entry(image, facing_zero)
                    # RGBA getbbox() defaults to the alpha channel and can miss
                    # RGB differences when both crops are fully opaque.  A
                    # bytewise comparison covers every channel and metadata
                    # equality is checked separately above.
                    record["pixels_equal"] = first.size == second.size and first.tobytes() == second.tobytes()
                except (KeyError, TypeError, ValueError):
                    record["pixels_equal"] = False
        aliases[unit] = record

    labels: dict[str, dict[str, Any]] = {}
    for key, meta in entries.items():
        info = label_for_key(key, meta if isinstance(meta, dict) else {})
        if isinstance(meta, dict):
            info["native_dimensions"] = [meta.get("w"), meta.get("h")]
            info["anchor"] = [meta.get("anchor_x"), meta.get("anchor_y")]
            info["rect"] = [meta.get("x"), meta.get("y"), meta.get("w"), meta.get("h")]
        labels[key] = info

    movement_keys = sorted(key for key in entries if WALK_RE.match(key))
    idle_keys = sorted(key for key in entries if FACING_RE.match(key))
    structure_keys = [key for key in STRUCTURE_NAMES if key in entries]
    expected_idle_keys = sorted(f"{unit}_{facing}" for unit in UNIT_NAMES for facing in range(8))
    expected_movement_keys = sorted(
        f"{unit}_{facing}_walk_{phase}"
        for unit in UNIT_NAMES
        for facing in range(8)
        for phase in range(4)
    )
    expected_aliases = all(record["metadata_equal"] and record["pixels_equal"] for record in aliases.values())
    validation = {
        "png": str(png_path),
        "manifest": str(manifest_path),
        "png_dimensions": [image.width, image.height],
        "manifest_dimensions": [manifest.get("width"), manifest.get("height")],
        "dimensions_match": dimensions_match,
        "entry_count": len(entries),
        "rect_count": len(entry_stats),
        "all_rects_valid": not rect_errors and len(entry_stats) == len(entries),
        "rect_errors": rect_errors,
        "anchor_errors": anchor_errors,
        "all_anchors_valid": not anchor_errors and len(entry_stats) == len(entries),
        "empty_entries": empty_entries,
        "no_empty_entries": not empty_entries,
        "intermediate_alpha_entries": intermediate_entries,
        "all_entry_alpha_binary": not intermediate_entries and len(entry_stats) == len(entries),
        "global_alpha": global_alpha,
        "movement_entry_count": len(movement_keys),
        "idle_entry_count": len(idle_keys),
        "structure_entry_count": len(structure_keys),
        "idle_keys": idle_keys,
        "movement_keys": movement_keys,
        "idle_key_set_complete": idle_keys == expected_idle_keys,
        "movement_key_set_complete": movement_keys == expected_movement_keys,
        "structure_keys": structure_keys,
        "aliases": aliases,
        "all_aliases_equal": expected_aliases,
        "key_labels": labels,
        "entry_alpha_stats": entry_stats,
    }
    return Atlas(label, png_path, manifest_path, image, manifest, entries, validation)


def text_size(draw: ImageDraw.ImageDraw, value: str) -> tuple[int, int]:
    box = draw.textbbox((0, 0), value, font=FONT)
    return box[2] - box[0], box[3] - box[1]


def composite_crop(crop: Image.Image, scale: int = SCALE) -> Image.Image:
    """Make a review crop on a solid background with nearest-neighbor pixels."""

    scaled = crop.resize((crop.width * scale, crop.height * scale), Image.Resampling.NEAREST)
    background = Image.new("RGBA", scaled.size, BG)
    background.alpha_composite(scaled)
    return background


def draw_panel(
    atlas: Atlas,
    key: str,
    panel_size: tuple[int, int],
    label_color: tuple[int, int, int, int],
    title: str | None = None,
) -> Image.Image:
    meta = atlas.entries[key]
    crop = crop_from_entry(atlas.image, meta)
    art = composite_crop(crop)
    panel_w, panel_h = panel_size
    out = Image.new("RGBA", (panel_w, panel_h), PANEL)
    draw = ImageDraw.Draw(out)
    header = title or key
    dims = f"{meta['w']}x{meta['h']} native @2x"
    draw.text((8, 6), header, fill=label_color, font=FONT)
    draw.text((8, 20), dims, fill=MUTED, font=FONT)
    x = 8 + max(0, (panel_w - 16 - art.width) // 2)
    y = 38 + max(0, (panel_h - 38 - art.height) // 2)
    out.alpha_composite(art, (x, y))
    draw.rectangle((x - 1, y - 1, x + art.width, y + art.height), outline=GRID, width=1)
    return out


def comparison_artifact(old: Atlas, new: Atlas, keys: Iterable[str], output: Path, title: str) -> dict[str, Any]:
    records = [key for key in keys if key in old.entries and key in new.entries]
    missing = [key for key in keys if key not in old.entries or key not in new.entries]
    max_w = max((max(old.entries[key]["w"], new.entries[key]["w"]) * SCALE for key in records), default=0)
    max_h = max((max(old.entries[key]["h"], new.entries[key]["h"]) * SCALE for key in records), default=0)
    panel_size = (max_w + 24, max_h + 42)
    pair_w = panel_size[0] * 2 + 28
    pair_h = panel_size[1] + 30
    pairs: list[Image.Image] = []
    for key in records:
        pair = Image.new("RGBA", (pair_w, pair_h), BG)
        pair.alpha_composite(draw_panel(old, key, panel_size, ACCENT_OLD, f"{key} · previous v2"), (4, 22))
        pair.alpha_composite(draw_panel(new, key, panel_size, ACCENT_NEW, f"{key} · root redraw"), (panel_size[0] + 20, 22))
        ImageDraw.Draw(pair).text((4, 4), key, fill=TEXT, font=FONT)
        pairs.append(pair)
    columns = 2
    rows = (len(pairs) + columns - 1) // columns
    width = max(pair_w * columns + 8, 240)
    height = 30 + pair_h * rows
    canvas = Image.new("RGBA", (width, height), BG)
    draw = ImageDraw.Draw(canvas)
    draw.text((8, 8), title + " · exact crops · nearest 2x", fill=TEXT, font=FONT)
    for index, pair in enumerate(pairs):
        x = (index % columns) * pair_w + 4
        y = 30 + (index // columns) * pair_h
        canvas.alpha_composite(pair, (x, y))
    canvas.convert("RGB").save(output)
    return {
        "path": str(output.relative_to(ROOT)),
        "keys": records,
        "missing": missing,
        "scale": SCALE,
        "layout": "2 comparison pairs per row",
        "no_clipping": True,
    }


def direction_sheet(atlas: Atlas, unit: str, output: Path) -> dict[str, Any]:
    keys = [f"{unit}_{facing}" for facing in range(8)]
    available = [key for key in keys if key in atlas.entries]
    cell_w = 64 * SCALE
    cell_h = 64 * SCALE
    left = 86
    canvas = Image.new("RGBA", (left + cell_w * 8, 38 + cell_h + 24), BG)
    draw = ImageDraw.Draw(canvas)
    draw.text((8, 8), f"{unit.title()} · 8 direction sheet", fill=TEXT, font=FONT)
    for facing, key in enumerate(keys):
        x = left + facing * cell_w
        draw.text((x + cell_w // 2 - 8, 24), str(facing), fill=MUTED, font=FONT)
        if key in atlas.entries:
            canvas.alpha_composite(composite_crop(crop_from_entry(atlas.image, atlas.entries[key])), (x, 38))
        else:
            draw.rectangle((x, 38, x + cell_w - 1, 38 + cell_h - 1), outline=ACCENT_OLD)
    canvas.convert("RGB").save(output)
    return {"path": str(output.relative_to(ROOT)), "unit": unit, "keys": available, "missing": [key for key in keys if key not in available], "scale": SCALE}


def all_direction_sheet(atlas: Atlas, output: Path) -> dict[str, Any]:
    cell_w = 64 * SCALE
    cell_h = 64 * SCALE
    left = 86
    row_h = 38 + cell_h
    canvas = Image.new("RGBA", (left + cell_w * 8, row_h * len(UNIT_NAMES)), BG)
    draw = ImageDraw.Draw(canvas)
    for row, unit in enumerate(UNIT_NAMES):
        y = row * row_h
        draw.text((8, y + 8), unit.title(), fill=TEXT, font=FONT)
        for facing in range(8):
            x = left + facing * cell_w
            key = f"{unit}_{facing}"
            draw.text((x + cell_w // 2 - 8, y + 8), str(facing), fill=MUTED, font=FONT)
            if key in atlas.entries:
                canvas.alpha_composite(composite_crop(crop_from_entry(atlas.image, atlas.entries[key])), (x, y + 38))
    canvas.convert("RGB").save(output)
    return {"path": str(output.relative_to(ROOT)), "units": list(UNIT_NAMES), "scale": SCALE, "facings": 8}


def units_overview_1x(atlas: Atlas, output: Path) -> dict[str, Any]:
    """Render one native current unit cell per tile without any enlargement."""

    cell_w = 64
    cell_h = 64
    col_w = 104
    # The second row's label plus its 64px native crop needs 104px of vertical
    # room; the prior 90px row silently clipped the bottom of those cells.
    row_h = 104
    canvas = Image.new("RGBA", (col_w * 4, row_h * 2), BG)
    draw = ImageDraw.Draw(canvas)
    draw.text((8, 6), "Current units · native 1x overview", fill=TEXT, font=FONT)
    for index, unit in enumerate(UNIT_NAMES):
        col = index % 4
        row = index // 4
        x = col * col_w
        y = row * row_h + 20
        draw.text((x + 8, y), f"{unit.title()} · 64x64", fill=TEXT, font=FONT)
        key = unit if unit in atlas.entries else f"{unit}_0"
        if key in atlas.entries:
            art = composite_crop(crop_from_entry(atlas.image, atlas.entries[key]), scale=1)
            canvas.alpha_composite(art, (x + 20, y + 18))
        else:
            draw.rectangle((x + 20, y + 18, x + 83, y + 81), outline=ACCENT_OLD)
    canvas.convert("RGB").save(output)
    return {"path": str(output.relative_to(ROOT)), "units": list(UNIT_NAMES), "scale": 1, "native_dimensions": [64, 64]}


def movement_sheet(atlas: Atlas, output: Path) -> dict[str, Any]:
    cell_w = 64 * SCALE
    cell_h = 64 * SCALE
    left = 104
    row_h = 38 + cell_h
    canvas = Image.new("RGBA", (left + cell_w * 4, row_h * len(UNIT_NAMES)), BG)
    draw = ImageDraw.Draw(canvas)
    for row, unit in enumerate(UNIT_NAMES):
        y = row * row_h
        draw.text((8, y + 8), f"{unit.title()} · face 0", fill=TEXT, font=FONT)
        for phase in range(4):
            x = left + phase * cell_w
            draw.text((x + cell_w // 2 - 8, y + 8), str(phase), fill=MUTED, font=FONT)
            key = f"{unit}_0_walk_{phase}"
            if key in atlas.entries:
                canvas.alpha_composite(composite_crop(crop_from_entry(atlas.image, atlas.entries[key])), (x, y + 38))
    canvas.convert("RGB").save(output)
    return {"path": str(output.relative_to(ROOT)), "units": list(UNIT_NAMES), "facing": 0, "phases": 4, "scale": SCALE}


def movement_gif(atlas: Atlas, output: Path) -> dict[str, Any]:
    cell_w = 64 * SCALE
    cell_h = 64 * SCALE
    left = 104
    row_h = 38 + cell_h
    top = 28
    columns = 2
    roster_rows = (len(UNIT_NAMES) + columns - 1) // columns
    frames: list[Image.Image] = []
    for phase in range(4):
        # A compact 2x4 roster keeps the loop readable while leaving room for a
        # complete header.  Each frame shows every unit at this one phase.
        frame = Image.new("RGBA", (left + cell_w * columns, top + row_h * roster_rows), BG)
        draw = ImageDraw.Draw(frame)
        draw.text((8, 8), f"Movement p{phase} · face 0 · 2x", fill=TEXT, font=FONT)
        for index, unit in enumerate(UNIT_NAMES):
            row = index // columns
            column = index % columns
            x = left + column * cell_w
            y = top + row * row_h
            draw.text((x + 4, y + 8), unit.title(), fill=TEXT, font=FONT)
            key_phase = f"{unit}_0_walk_{phase}"
            if key_phase in atlas.entries:
                frame.alpha_composite(composite_crop(crop_from_entry(atlas.image, atlas.entries[key_phase])), (x, y + 38))
            else:
                # Keep a phase-wide frame useful even when an input atlas is partial.
                key_idle = f"{unit}_0"
                if key_idle in atlas.entries:
                    frame.alpha_composite(composite_crop(crop_from_entry(atlas.image, atlas.entries[key_idle])), (x, y + 38))
        frames.append(frame.convert("RGB"))
    frames[0].save(output, format="GIF", save_all=True, append_images=frames[1:], duration=160, loop=0, disposal=2)
    return {
        "path": str(output.relative_to(ROOT)),
        "frames": 4,
        "duration_ms": 160,
        "loop": True,
        "facing": 0,
        "scale": SCALE,
        "layout": "2 columns x 4 rows",
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--current-png", type=Path, default=DEFAULT_CURRENT_PNG)
    parser.add_argument("--current-json", type=Path, default=DEFAULT_CURRENT_JSON)
    parser.add_argument("--archive-png", type=Path, default=DEFAULT_ARCHIVE_PNG)
    parser.add_argument("--archive-json", type=Path, default=DEFAULT_ARCHIVE_JSON)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    current = validate_atlas("current", args.current_png, args.current_json)
    archive = validate_atlas("archive", args.archive_png, args.archive_json)

    artifacts: dict[str, Any] = {}
    artifacts["units_comparison"] = comparison_artifact(
        archive,
        current,
        UNIT_NAMES,
        output_dir / "unit-comparison-2x.png",
        "8-unit before / after",
    )
    artifacts["structures_comparison"] = comparison_artifact(
        archive,
        current,
        STRUCTURE_NAMES,
        output_dir / "structure-comparison-2x.png",
        "10-structure before / after",
    )
    artifacts["units_overview_1x"] = units_overview_1x(current, output_dir / "units-overview-1x.png")
    artifacts["directions_combined"] = all_direction_sheet(current, output_dir / "units-directions-2x.png")
    artifacts["movement_combined"] = movement_sheet(current, output_dir / "movement-phases-2x.png")
    artifacts["movement_gif"] = movement_gif(current, output_dir / "movement-loop.gif")
    artifacts["directions_by_unit"] = []
    for unit in UNIT_NAMES:
        artifacts["directions_by_unit"].append(direction_sheet(current, unit, output_dir / f"{unit}-directions-2x.png"))

    expected_current = {
        "entry_count": 360,
        "idle_entry_count": 64,
        "movement_entry_count": 256,
        "structure_entry_count": 10,
    }
    expected_checks = {
        "current_entry_count_360": current.validation["entry_count"] == expected_current["entry_count"],
        "current_idle_entries_64": current.validation["idle_entry_count"] == expected_current["idle_entry_count"],
        "current_idle_key_set_complete": current.validation["idle_key_set_complete"],
        "current_movement_entries_256": current.validation["movement_entry_count"] == expected_current["movement_entry_count"],
        "current_movement_key_set_complete": current.validation["movement_key_set_complete"],
        "current_structures_10": current.validation["structure_entry_count"] == expected_current["structure_entry_count"],
        "current_dimensions_match": current.validation["dimensions_match"],
        "archive_dimensions_match": archive.validation["dimensions_match"],
        "current_rects_valid": current.validation["all_rects_valid"],
        "archive_rects_valid": archive.validation["all_rects_valid"],
        "current_anchors_valid": current.validation["all_anchors_valid"],
        "archive_anchors_valid": archive.validation["all_anchors_valid"],
        "current_no_empty_entries": current.validation["no_empty_entries"],
        "archive_no_empty_entries": archive.validation["no_empty_entries"],
        "current_alpha_binary": current.validation["all_entry_alpha_binary"] and current.validation["global_alpha"]["binary"],
        "archive_alpha_binary": archive.validation["all_entry_alpha_binary"] and archive.validation["global_alpha"]["binary"],
        "current_alias_equality": current.validation["all_aliases_equal"],
        "archive_alias_equality": archive.validation["all_aliases_equal"],
    }
    report = {
        "tool": "tools/art_v3/review.py",
        "purpose": "Review-only native pixel diagnostics; source and atlas inputs are never rewritten.",
        "scale": SCALE,
        "resampling": "nearest",
        "alpha_smoothing": False,
        "expected_current": expected_current,
        "checks": expected_checks,
        "all_requested_checks_pass": all(expected_checks.values()),
        "current": current.validation,
        "archive": archive.validation,
        "artifacts": artifacts,
    }
    report_path = output_dir / "validation-art.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"report": str(report_path), "all_requested_checks_pass": report["all_requested_checks_pass"], "artifacts": artifacts}, indent=2))
    return 0 if report["all_requested_checks_pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
