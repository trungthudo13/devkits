"""Regenerate checked-in native icons from assets/devkits.svg.

Maintenance only (Ubuntu: python3-gi gir1.2-rsvg-2.0 python3-pil).
Normal application and installer builds use the checked-in outputs.
"""
from pathlib import Path
import hashlib
import io
import json
import xml.etree.ElementTree as ET
import gi
gi.require_version("Rsvg", "2.0")
from gi.repository import Rsvg
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent

def main():
    source = ROOT / "assets/devkits.svg"
    output = ROOT / "assets/icons"
    output.mkdir(parents=True, exist_ok=True)
    svg = ET.fromstring(source.read_bytes())
    svg.set("width", "1024")
    svg.set("height", "1024")
    handle = Rsvg.Handle.new_from_data(ET.tostring(svg))
    _, png = handle.get_pixbuf().save_to_bufferv("png", [], [])
    image = Image.open(io.BytesIO(png)).convert("RGBA")
    image.resize((256, 256), Image.Resampling.LANCZOS).save(output / "devkits.png")
    image.save(output / "devkits.ico", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
    image.save(output / "devkits.icns", format="ICNS")
    paths = [source, *(output / f"devkits.{ext}" for ext in ("png", "ico", "icns"))]
    manifest = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

if __name__ == "__main__":
    main()
