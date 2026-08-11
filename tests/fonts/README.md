# Test fonts

Faces the test suite resolves against, so the font pre-flight
([ARCHITECTURE §7.1](../../docs/ARCHITECTURE.md#71-fonts-and-scripts)) can be
tested on a machine with nothing installed and no SILE bundle unpacked.

They are here rather than assumed present for the reason P5.3 gives: a suite
that depends on the operating system's fonts fails when somebody updates one,
and passes for the wrong reason when somebody installs one.

| | |
|---|---|
| `DejaVuSerif.ttf`, `DejaVuSerif-Bold.ttf` | **DejaVu Fonts 2.37.** Latin, Greek, Cyrillic; **no Indic**. The application's built-in default, and the face that makes the coverage check's failing case reproducible. Under the [DejaVu Fonts License](https://dejavu-fonts.github.io/License.html) — a Bitstream Vera derivative that permits redistribution. |

The Tamil counterpart lives in [`spike/assets/fonts/`](../../spike/assets/fonts/)
and is deliberately not duplicated here: the coverage tests need one font that
covers a script and one that does not, and those two directories are exactly
that pair.
