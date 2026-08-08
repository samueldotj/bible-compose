"""Structural facts about a PDF, without poppler.

SILE's libtexpdf backend writes cross-reference and object streams, so the
objects that record page count and fonts are Flate-compressed and invisible to
a plain byte search. This decompresses every stream before looking.

Deliberately structural rather than visual: it is the shape the PDF assertions
at P5.3 need — page count, geometry, embedded fonts — none of which can be
asserted by byte comparison (SRS-REVIEW F4).
"""

import re
import sys
import zlib


def streams(data):
    """Yield every successfully inflated stream body."""
    for m in re.finditer(rb"stream\r?\n", data):
        start = m.end()
        end = data.find(b"endstream", start)
        if end < 0:
            continue
        try:
            yield zlib.decompress(data[start:end])
        except zlib.error:
            continue


def main(path):
    raw = open(path, "rb").read()
    blob = raw + b"".join(streams(raw))

    print(f"file       : {path}")
    print(f"bytes      : {len(raw):,}")
    print(f"header     : {raw[:8].decode('latin1')}")

    pages = len(re.findall(rb"/Type\s*/Page[^s]", blob))
    print(f"pages      : {pages}")

    for box in sorted({b.strip() for b in re.findall(rb"/MediaBox\s*\[([^\]]*)\]", blob)}):
        v = [float(x) for x in box.split()]
        w, h = (v[2] - v[0]) / 72, (v[3] - v[1]) / 72
        print(f"page size  : {w:.2f}in x {h:.2f}in  ({w * 25.4:.0f} x {h * 25.4:.0f} mm)")

    fonts = sorted({f.decode() for f in re.findall(rb"/BaseFont\s*/([A-Za-z0-9+#,._-]+)", blob)})
    print(f"fonts      : {len(fonts)}")
    for f in fonts:
        subset = "subset" if re.match(r"^[A-Z]{6}\+", f) else "full"
        print(f"             {f}  ({subset})")

    embedded = len(re.findall(rb"/FontFile[23]?\b", blob))
    print(f"font files : {embedded} embedded")
    if fonts and not embedded:
        print("             WARNING: fonts referenced but none embedded (PDF-003)")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: inspect-pdf.py FILE.pdf")
    main(sys.argv[1])
