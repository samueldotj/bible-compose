# Spike assets

## fonts/

**Noto Serif Tamil v2.004**, Regular and Bold, from the [notofonts/tamil](https://github.com/notofonts/tamil/releases/tag/NotoSerifTamil-v2.004) release.

Licensed under the [SIL Open Font License 1.1](https://openfontlicense.org/), which permits redistribution — so committing them here is deliberate rather than convenient. It keeps the spike reproducible: S0.5's result depends on *this* face, and a machine that resolves a different Tamil font would be testing something else.

These files are here to be **read from the project directory rather than installed**, which is the FONT-003 condition under test. fontconfig on the spike machine reports zero Tamil fonts, so nothing else could be supplying the glyphs.

Not a recommendation for the shipped default — that is [P6.2](../../docs/ROADMAP.md#phase-6--m6--release-quality), and it needs a licensing review across the whole default set rather than one face.

## images/

Synthetic test artwork for S0.7, generated for the spike:

| | |
|---|---|
| `map.png`, `map.jpg` | the same crude coastline, raster, 900×600 |
| `art.pdf` | vector artwork, produced by SILE, 3×2in |

Deliberately synthetic. S0.7 tests the placement path — scaling, aspect ratio, vector versus raster embedding — not the picture.
