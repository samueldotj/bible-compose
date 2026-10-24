/**
 * The font catalogue, generated from Google Fonts (FONT-006).
 *
 *     node scripts/font-catalogue.mjs            # rewrite the generated table
 *     node scripts/font-catalogue.mjs --check    # fail if it is out of date
 *
 * # Why a generator rather than a table somebody types
 *
 * Every row carries a byte count and a SHA-256, because the thing being
 * described is a binary blob this application will write into a publisher's
 * book. Those are not values a person can check by reading, and a table of
 * fifty hand-copied digests is a table with a typo in it — one that would
 * surface as "the font BibleCompose downloaded is not the font it expected",
 * which looks exactly like an attack and would be a transcription slip.
 *
 * So the spec below says only what a human can judge — which families suit
 * which script, and in what order to offer them — and everything mechanical is
 * fetched once and written out.
 *
 * # What this verifies while it generates
 *
 * Fetching the files is the cheap part. The reason this is worth running is
 * that it checks each one against what the spec claims:
 *
 *   * the family name in the font's own `name` table is the name the spec
 *     uses, so the string a build resolves against is not a guess;
 *   * the font's `cmap` has a glyph for every script the spec assigns it,
 *     probed with that script's sample letter — which is the claim that
 *     matters, since a suggestion that cannot draw the script is worse than no
 *     suggestion at all;
 *   * the default instance of a variable font is the weight a page should be
 *     set in. A project font crosses to the backend as a *path* (FONT-003), so
 *     it is pinned to the file's default instance — and two of the families
 *     looked at while this was written default to ExtraLight, which is how
 *     Noto Serif SC and TC came to be left out rather than shipped thin.
 *
 * `--check` does none of it. It re-renders the table from the file's own
 * recorded values and compares the text, so CI stays offline (NFR-004) and a
 * hand edit to the generated file is still caught.
 *
 * # Adding a family
 *
 * Add it to `TYPEFACES`, run this, commit both files. If the family is not on
 * Google Fonts it does not belong here — the catalogue is one host and one
 * licence family on purpose (ARCHITECTURE section 7.6).
 */

import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const OUT = join(ROOT, "crates", "biblecompose-app", "src", "typeface", "catalogue.rs");

/**
 * The commit of `google/fonts` every file is read from.
 *
 * A revision and not `main`, for the reason the parser is pinned to one: a
 * moving target would let a font change underneath a checksum, and the first
 * anybody heard of it would be a download that refuses to verify.
 */
const REVISION = "e7db6ca603fd4862010383ff17ffd6cc4ffac1c0";

const HOST = "https://raw.githubusercontent.com";
const REPO = "google/fonts";

/**
 * The scripts a suggestion can be made for, and how to recognise one.
 *
 * `languages` is what makes a suggestion possible before any Scripture has
 * been read — a project's BCP-47 tag is the only thing a new project has. It
 * is a primary-subtag list and deliberately not exhaustive: a tag nobody
 * listed falls through to the codepoints, which are the better evidence
 * anyway.
 *
 * `sample` is a printable base letter, never a combining mark: a mark has no
 * standalone glyph, so probing with one would report every font as lacking the
 * script.
 *
 * `subsumedBy` names the scripts that already imply this one. Japanese and
 * Korean text carries Han characters, so a Japanese document contains two
 * scripts by any honest reading and needs one font — and a second list of
 * Chinese faces beside it would be a question nobody asked.
 */
const SCRIPTS = [
  {
    id: "latn",
    name: "Latin",
    sample: "A",
    ranges: [[0x0041, 0x024f], [0x1e00, 0x1eff]],
    languages: ["en", "es", "pt", "fr", "de", "it", "nl", "pl", "ro", "vi", "id",
      "ms", "tl", "sw", "ha", "yo", "ig", "zu", "lus", "kha", "grt", "tr", "af",
      "so", "rw", "ny", "sn", "st", "tn", "xh", "qu", "ay", "gn", "ht"],
  },
  {
    id: "cyrl",
    name: "Cyrillic",
    sample: "А",
    ranges: [[0x0400, 0x04ff], [0x0500, 0x052f]],
    languages: ["ru", "uk", "be", "bg", "sr", "mk", "kk", "ky", "mn", "tg", "ab"],
  },
  {
    id: "grek",
    name: "Greek",
    sample: "α",
    ranges: [[0x0370, 0x03ff], [0x1f00, 0x1fff]],
    languages: ["el", "grc"],
  },
  {
    id: "hebr",
    name: "Hebrew",
    sample: "א",
    ranges: [[0x0590, 0x05ff], [0xfb1d, 0xfb4f]],
    languages: ["he", "hbo", "yi"],
  },
  {
    id: "arab",
    name: "Arabic",
    sample: "ا",
    ranges: [[0x0600, 0x06ff], [0x0750, 0x077f], [0x08a0, 0x08ff],
      [0xfb50, 0xfdff], [0xfe70, 0xfeff]],
    languages: ["ar", "fa", "ur", "sd", "ks", "ps", "ug", "ckb"],
  },
  {
    id: "syrc",
    name: "Syriac",
    sample: "ܐ",
    ranges: [[0x0700, 0x074f], [0x0860, 0x086f]],
    languages: ["syr", "aii", "cld", "tru"],
  },
  {
    id: "deva",
    name: "Devanagari",
    sample: "क",
    ranges: [[0x0900, 0x097f], [0xa8e0, 0xa8ff]],
    languages: ["hi", "mr", "ne", "sa", "kok", "doi", "brx", "mai", "bho",
      "awa", "mag", "hne", "new", "gom"],
  },
  {
    id: "beng",
    name: "Bengali",
    sample: "ক",
    ranges: [[0x0980, 0x09ff]],
    languages: ["bn", "as", "bpy", "mni-beng"],
  },
  {
    id: "guru",
    name: "Gurmukhi",
    sample: "ਕ",
    ranges: [[0x0a00, 0x0a7f]],
    languages: ["pa"],
  },
  {
    id: "gujr",
    name: "Gujarati",
    sample: "ક",
    ranges: [[0x0a80, 0x0aff]],
    languages: ["gu"],
  },
  {
    id: "orya",
    name: "Odia",
    sample: "କ",
    ranges: [[0x0b00, 0x0b7f]],
    languages: ["or"],
  },
  {
    id: "taml",
    name: "Tamil",
    sample: "க",
    ranges: [[0x0b80, 0x0bff]],
    languages: ["ta"],
  },
  {
    id: "telu",
    name: "Telugu",
    sample: "క",
    ranges: [[0x0c00, 0x0c7f]],
    languages: ["te"],
  },
  {
    id: "knda",
    name: "Kannada",
    sample: "ಕ",
    ranges: [[0x0c80, 0x0cff]],
    languages: ["kn", "tcy"],
  },
  {
    id: "mlym",
    name: "Malayalam",
    sample: "ക",
    ranges: [[0x0d00, 0x0d7f]],
    languages: ["ml"],
  },
  {
    id: "sinh",
    name: "Sinhala",
    sample: "ක",
    ranges: [[0x0d80, 0x0dff]],
    languages: ["si"],
  },
  {
    id: "thai",
    name: "Thai",
    sample: "ก",
    ranges: [[0x0e00, 0x0e7f]],
    languages: ["th"],
  },
  {
    id: "laoo",
    name: "Lao",
    sample: "ກ",
    ranges: [[0x0e80, 0x0eff]],
    languages: ["lo"],
  },
  {
    id: "khmr",
    name: "Khmer",
    sample: "ក",
    ranges: [[0x1780, 0x17ff], [0x19e0, 0x19ff]],
    languages: ["km"],
  },
  {
    id: "mymr",
    name: "Myanmar",
    sample: "က",
    ranges: [[0x1000, 0x109f], [0xaa60, 0xaa7f]],
    languages: ["my", "shn", "kar", "mnw"],
  },
  {
    id: "tibt",
    name: "Tibetan",
    sample: "ཀ",
    ranges: [[0x0f00, 0x0fff]],
    languages: ["bo", "dz"],
  },
  {
    id: "ethi",
    name: "Ethiopic",
    sample: "ሀ",
    ranges: [[0x1200, 0x137f], [0x1380, 0x139f], [0x2d80, 0x2ddf]],
    languages: ["am", "ti", "gez", "tig", "wal"],
  },
  {
    id: "copt",
    name: "Coptic",
    sample: "Ⲁ",
    ranges: [[0x2c80, 0x2cff], [0x03e2, 0x03ef]],
    languages: ["cop"],
  },
  {
    id: "geor",
    name: "Georgian",
    sample: "ა",
    ranges: [[0x10a0, 0x10ff], [0x2d00, 0x2d2f]],
    languages: ["ka"],
  },
  {
    id: "armn",
    name: "Armenian",
    sample: "ա",
    ranges: [[0x0530, 0x058f], [0xfb13, 0xfb17]],
    languages: ["hy"],
  },
  {
    id: "mtei",
    name: "Meitei Mayek",
    sample: "ꯀ",
    ranges: [[0xabc0, 0xabff], [0xaae0, 0xaaff]],
    languages: ["mni"],
  },
  {
    id: "olck",
    name: "Ol Chiki",
    sample: "ᱚ",
    ranges: [[0x1c50, 0x1c7f]],
    languages: ["sat"],
  },
  {
    id: "jpan",
    name: "Japanese",
    // Kana and not the kanji: kana is what makes a document Japanese rather
    // than Chinese, and the Han block is its own row below.
    sample: "あ",
    ranges: [[0x3040, 0x309f], [0x30a0, 0x30ff], [0x31f0, 0x31ff]],
    languages: ["ja"],
  },
  {
    id: "kore",
    name: "Korean",
    sample: "가",
    ranges: [[0x1100, 0x11ff], [0x3130, 0x318f], [0xa960, 0xa97f],
      [0xac00, 0xd7af], [0xd7b0, 0xd7ff]],
    languages: ["ko"],
  },
  {
    id: "hani",
    name: "Han",
    sample: "一",
    ranges: [[0x3400, 0x4dbf], [0x4e00, 0x9fff], [0xf900, 0xfaff]],
    languages: ["zh", "yue", "nan", "hak"],
    subsumedBy: ["jpan", "kore"],
  },
];

/**
 * What to offer, in the order to offer it.
 *
 * Order within a script is a judgement and the only judgement here: the face
 * SIL designed for Scripture comes before the Noto that merely covers the
 * script, because it was drawn for running text at a Bible's size and
 * generally ships a real bold and italic rather than one weight axis.
 *
 * `scripts` is a claim this generator checks against the font's `cmap`. A
 * family listed for a script it cannot draw fails the run.
 */
const TYPEFACES = [
  {
    family: "Charis SIL",
    dir: "charissil",
    scripts: ["latn", "cyrl"],
    note: "SIL's face for long passages, drawn to stay legible at a Bible's size and on poor paper",
    files: [
      ["CharisSIL-Regular.ttf", "regular"],
      ["CharisSIL-Bold.ttf", "bold"],
      ["CharisSIL-Italic.ttf", "italic"],
      ["CharisSIL-BoldItalic.ttf", "bold-italic"],
    ],
  },
  {
    family: "Gentium Book Plus",
    dir: "gentiumbookplus",
    scripts: ["latn", "grek", "cyrl"],
    note: "SIL's Scripture face, the heavier of the two Gentiums; covers polytonic Greek",
    files: [
      ["GentiumBookPlus-Regular.ttf", "regular"],
      ["GentiumBookPlus-Bold.ttf", "bold"],
      ["GentiumBookPlus-Italic.ttf", "italic"],
      ["GentiumBookPlus-BoldItalic.ttf", "bold-italic"],
    ],
  },
  {
    family: "Gentium Plus",
    dir: "gentiumplus",
    scripts: ["latn", "grek", "cyrl"],
    note: "The lighter Gentium, for a roomier page; covers polytonic Greek",
    files: [
      ["GentiumPlus-Regular.ttf", "regular"],
      ["GentiumPlus-Bold.ttf", "bold"],
      ["GentiumPlus-Italic.ttf", "italic"],
      ["GentiumPlus-BoldItalic.ttf", "bold-italic"],
    ],
  },
  {
    family: "Noto Serif",
    dir: "notoserif",
    scripts: ["latn", "cyrl", "grek"],
    note: "Noto's Latin serif, as a fallback when neither Gentium suits",
    files: [
      ["NotoSerif[wdth,wght].ttf", "regular"],
      ["NotoSerif-Italic[wdth,wght].ttf", "italic"],
    ],
  },
  {
    family: "Noto Serif Hebrew",
    dir: "notoserifhebrew",
    scripts: ["hebr"],
    note: "Covers pointed Hebrew, including the cantillation marks",
    files: [["NotoSerifHebrew[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Scheherazade New",
    dir: "scheherazadenew",
    scripts: ["arab"],
    note: "SIL's Naskh face, drawn for Scripture and fully vocalised text",
    files: [
      ["ScheherazadeNew-Regular.ttf", "regular"],
      ["ScheherazadeNew-Bold.ttf", "bold"],
    ],
  },
  {
    family: "Noto Naskh Arabic",
    dir: "notonaskharabic",
    scripts: ["arab"],
    note: "Noto's Naskh, as an alternative to Scheherazade",
    files: [["NotoNaskhArabic[wght].ttf", "regular"]],
  },
  {
    family: "Noto Sans Syriac",
    dir: "notosanssyriac",
    scripts: ["syrc"],
    note: "The only Syriac on Google Fonts; there is no serif companion",
    files: [["NotoSansSyriac[wght].ttf", "regular"]],
  },
  {
    family: "Annapurna SIL",
    dir: "annapurnasil",
    scripts: ["deva"],
    note: "SIL's Devanagari, with a real bold rather than a weight axis",
    files: [
      ["AnnapurnaSIL-Regular.ttf", "regular"],
      ["AnnapurnaSIL-Bold.ttf", "bold"],
    ],
  },
  {
    family: "Noto Serif Devanagari",
    dir: "notoserifdevanagari",
    scripts: ["deva"],
    note: "Noto's Devanagari serif",
    files: [["NotoSerifDevanagari[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Bengali",
    dir: "notoserifbengali",
    scripts: ["beng"],
    note: "Covers Bengali and Assamese",
    files: [["NotoSerifBengali[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Gurmukhi",
    dir: "notoserifgurmukhi",
    scripts: ["guru"],
    note: "Noto's Gurmukhi serif",
    files: [["NotoSerifGurmukhi[wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Gujarati",
    dir: "notoserifgujarati",
    scripts: ["gujr"],
    note: "Noto's Gujarati serif",
    files: [["NotoSerifGujarati[wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Oriya",
    dir: "notoseriforiya",
    scripts: ["orya"],
    note: "Noto's Odia serif",
    files: [["NotoSerifOriya[wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Tamil",
    dir: "notoseriftamil",
    scripts: ["taml"],
    note: "Noto's Tamil serif, and one of the few with a drawn italic",
    files: [
      ["NotoSerifTamil[wdth,wght].ttf", "regular"],
      ["NotoSerifTamil-Italic[wdth,wght].ttf", "italic"],
    ],
  },
  {
    family: "Noto Serif Telugu",
    dir: "notoseriftelugu",
    scripts: ["telu"],
    note: "Noto's Telugu serif",
    files: [["NotoSerifTelugu[wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Kannada",
    dir: "notoserifkannada",
    scripts: ["knda"],
    note: "Noto's Kannada serif",
    files: [["NotoSerifKannada[wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Malayalam",
    dir: "notoserifmalayalam",
    scripts: ["mlym"],
    note: "Noto's Malayalam serif",
    files: [["NotoSerifMalayalam[wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Sinhala",
    dir: "notoserifsinhala",
    scripts: ["sinh"],
    note: "Noto's Sinhala serif",
    files: [["NotoSerifSinhala[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Thai",
    dir: "notoserifthai",
    scripts: ["thai"],
    note: "Noto's Thai serif",
    files: [["NotoSerifThai[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Lao",
    dir: "notoseriflao",
    scripts: ["laoo"],
    note: "Noto's Lao serif",
    files: [["NotoSerifLao[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Khmer",
    dir: "notoserifkhmer",
    scripts: ["khmr"],
    note: "Noto's Khmer serif",
    files: [["NotoSerifKhmer[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Padauk",
    dir: "padauk",
    scripts: ["mymr"],
    note: "SIL's Myanmar face, with the Shan and Mon letters many Noto builds leave out",
    files: [
      ["Padauk-Regular.ttf", "regular"],
      ["Padauk-Bold.ttf", "bold"],
    ],
  },
  {
    family: "Noto Serif Myanmar",
    dir: "notoserifmyanmar",
    scripts: ["mymr"],
    note: "A Myanmar serif, and one of the few Noto families still cut as separate weights",
    files: [
      ["NotoSerifMyanmar-Regular.ttf", "regular"],
      ["NotoSerifMyanmar-Bold.ttf", "bold"],
    ],
  },
  {
    family: "Noto Serif Tibetan",
    dir: "notoseriftibetan",
    scripts: ["tibt"],
    note: "Noto's Tibetan serif",
    files: [["NotoSerifTibetan[wght].ttf", "regular"]],
  },
  {
    family: "Abyssinica SIL",
    dir: "abyssinicasil",
    scripts: ["ethi"],
    note: "SIL's Ethiopic, in the manuscript tradition Amharic Scripture is set in",
    files: [["AbyssinicaSIL-Regular.ttf", "regular"]],
  },
  {
    family: "Noto Serif Ethiopic",
    dir: "notoserifethiopic",
    scripts: ["ethi"],
    note: "Noto's Ethiopic serif, covering the Ge'ez extensions as well",
    files: [["NotoSerifEthiopic[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Sans Coptic",
    dir: "notosanscoptic",
    scripts: ["copt"],
    note: "The only Coptic on Google Fonts; there is no serif companion",
    files: [["NotoSansCoptic-Regular.ttf", "regular"]],
  },
  {
    family: "Noto Serif Georgian",
    dir: "notoserifgeorgian",
    scripts: ["geor"],
    note: "Noto's Georgian serif",
    files: [["NotoSerifGeorgian[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Serif Armenian",
    dir: "notoserifarmenian",
    scripts: ["armn"],
    note: "Noto's Armenian serif",
    files: [["NotoSerifArmenian[wdth,wght].ttf", "regular"]],
  },
  {
    family: "Noto Sans Meetei Mayek",
    dir: "notosansmeeteimayek",
    scripts: ["mtei"],
    note: "The only Meitei Mayek on Google Fonts; there is no serif companion",
    files: [["NotoSansMeeteiMayek[wght].ttf", "regular"]],
  },
  {
    family: "Noto Sans Ol Chiki",
    dir: "notosansolchiki",
    scripts: ["olck"],
    note: "The only Ol Chiki on Google Fonts; there is no serif companion",
    files: [["NotoSansOlChiki[wght].ttf", "regular"]],
  },
  {
    family: "Shippori Mincho",
    dir: "shipporimincho",
    scripts: ["jpan"],
    note: "A mincho face for Japanese, cut as separate weights rather than one axis",
    files: [
      ["ShipporiMincho-Regular.ttf", "regular"],
      ["ShipporiMincho-Bold.ttf", "bold"],
    ],
  },
  {
    // "NanumMyeongjo", with no space: what Google Fonts calls the family and
    // what the font calls itself differ here, and a build resolves by the
    // font's own name. The generator refused the spaced spelling, which is the
    // check earning its keep.
    family: "NanumMyeongjo",
    dir: "nanummyeongjo",
    scripts: ["kore"],
    note: "A myeongjo face for Korean. Hangul only: a project mixing in Han will hear about it from the coverage check",
    files: [
      ["NanumMyeongjo-Regular.ttf", "regular"],
      ["NanumMyeongjo-Bold.ttf", "bold"],
    ],
  },
  {
    // The only Han family on Google Fonts with a drawn bold. Every Noto Serif
    // CJK build there is one variable file whose default instance is
    // ExtraLight, and a project font is pinned to its default instance
    // (FONT-003) — so packaging one would set a Chinese Bible in ExtraLight
    // and nothing would say so. The generator refuses it on exactly that
    // ground, which is why this is a kai face and not a song one.
    family: "LXGW WenKai TC",
    dir: "lxgwwenkaitc",
    scripts: ["hani"],
    note: "Covers both simplified and traditional characters, and is the one Han family here with a drawn bold",
    files: [
      ["LXGWWenKaiTC-Regular.ttf", "regular"],
      ["LXGWWenKaiTC-Bold.ttf", "bold"],
    ],
  },
];

// ---------------------------------------------------------------- font tables

/** A big-endian UTF-16 name record, as a JavaScript string. */
function utf16(buf, from, to) {
  const copy = Buffer.from(buf.subarray(from, to));
  copy.swap16();
  return copy.toString("utf16le");
}

function tables(buf) {
  const found = new Map();
  const count = buf.readUInt16BE(4);
  for (let i = 0; i < count; i++) {
    const p = 12 + i * 16;
    found.set(buf.toString("ascii", p, p + 4), buf.readUInt32BE(p + 8));
  }
  return found;
}

/**
 * The family names the font calls itself.
 *
 * Both name 1 and name 16, because the two disagree on a variable font whose
 * default instance is not Regular: Noto Serif SC calls name 1 "Noto Serif SC
 * ExtraLight" and only name 16 "Noto Serif SC". A resolver reads 16 where it
 * exists, so a spec entry matching either is correct.
 */
function families(buf) {
  const at = tables(buf).get("name");
  if (at === undefined) return [];
  const count = buf.readUInt16BE(at + 2);
  const strings = at + buf.readUInt16BE(at + 4);
  const out = [];
  for (let i = 0; i < count; i++) {
    const r = at + 6 + i * 12;
    if (buf.readUInt16BE(r) !== 3 || buf.readUInt16BE(r + 2) !== 1) continue;
    const id = buf.readUInt16BE(r + 6);
    if (id !== 1 && id !== 16) continue;
    const len = buf.readUInt16BE(r + 8);
    const off = buf.readUInt16BE(r + 10);
    out.push(utf16(buf, strings + off, strings + off + len));
  }
  return out;
}

/**
 * Whether the font has a glyph for one character.
 *
 * Formats 4 and 12 only, which is every font in the catalogue and every font
 * Google Fonts ships. An unsupported subtable is reported rather than treated
 * as absent, so a format nobody anticipated fails the run instead of quietly
 * removing a family's scripts.
 */
function hasGlyph(buf, code) {
  const at = tables(buf).get("cmap");
  if (at === undefined) throw new Error("no cmap table");

  const count = buf.readUInt16BE(at + 2);
  let best = null;
  for (let i = 0; i < count; i++) {
    const r = at + 4 + i * 8;
    const platform = buf.readUInt16BE(r);
    const encoding = buf.readUInt16BE(r + 2);
    const sub = at + buf.readUInt32BE(r + 4);
    const unicode =
      (platform === 3 && (encoding === 1 || encoding === 10)) || platform === 0;
    if (!unicode) continue;
    // Prefer format 12: it is the one that reaches past the BMP, and a font
    // carrying both keeps the same mappings in each.
    const format = buf.readUInt16BE(sub);
    if (best === null || format === 12) best = { sub, format };
  }
  if (best === null) throw new Error("no Unicode cmap subtable");

  const { sub, format } = best;
  if (format === 12) {
    const groups = buf.readUInt32BE(sub + 12);
    for (let i = 0; i < groups; i++) {
      const g = sub + 16 + i * 12;
      if (code >= buf.readUInt32BE(g) && code <= buf.readUInt32BE(g + 4)) return true;
    }
    return false;
  }
  if (format === 4) {
    if (code > 0xffff) return false;
    const segs = buf.readUInt16BE(sub + 6) / 2;
    const ends = sub + 14;
    const starts = ends + segs * 2 + 2;
    const deltas = starts + segs * 2;
    const rangeOffsets = deltas + segs * 2;
    for (let i = 0; i < segs; i++) {
      const end = buf.readUInt16BE(ends + i * 2);
      if (code > end) continue;
      const start = buf.readUInt16BE(starts + i * 2);
      if (code < start) return false;
      const rangeOffset = buf.readUInt16BE(rangeOffsets + i * 2);
      if (rangeOffset === 0) {
        return ((code + buf.readInt16BE(deltas + i * 2)) & 0xffff) !== 0;
      }
      const glyphAt = rangeOffsets + i * 2 + rangeOffset + (code - start) * 2;
      if (glyphAt + 1 >= buf.length) return false;
      return buf.readUInt16BE(glyphAt) !== 0;
    }
    return false;
  }
  throw new Error(`cmap format ${format} is not one this script reads`);
}

/** The default value of the weight axis, or null for a static font. */
function defaultWeight(buf) {
  const at = tables(buf).get("fvar");
  if (at === undefined) return null;
  const axes = at + buf.readUInt16BE(at + 4);
  const count = buf.readUInt16BE(at + 8);
  const size = buf.readUInt16BE(at + 10);
  for (let i = 0; i < count; i++) {
    const a = axes + i * size;
    if (buf.toString("ascii", a, a + 4) === "wght") return buf.readInt32BE(a + 8) / 65536;
  }
  return null;
}

// -------------------------------------------------------------------- fetching

async function get(path) {
  const url = `${HOST}/${REPO}/${REVISION}/${path.replace(/\[/g, "%5B").replace(/\]/g, "%5D")}`;
  const response = await fetch(url, { headers: { "User-Agent": "biblecompose-font-catalogue" } });
  if (!response.ok) throw new Error(`${response.status} for ${url}`);
  return Buffer.from(await response.arrayBuffer());
}

const sha256 = (buf) => createHash("sha256").update(buf).digest("hex");

/** Fetch and check one family, returning the row to be written. */
async function resolve(spec) {
  const licence = await get(`ofl/${spec.dir}/OFL.txt`);
  const text = licence.toString("utf8");
  if (!text.includes("SIL OPEN FONT LICENSE")) {
    throw new Error(`${spec.family}: OFL.txt does not read like the Open Font License`);
  }

  const files = [];
  for (const [name, face] of spec.files) {
    const data = await get(`ofl/${spec.dir}/${name}`);

    const named = families(data);
    if (!named.includes(spec.family)) {
      throw new Error(
        `${spec.family}: ${name} calls itself ${named.map((n) => JSON.stringify(n)).join(" / ")}`,
      );
    }

    for (const id of spec.scripts) {
      const script = SCRIPTS.find((s) => s.id === id);
      if (!script) throw new Error(`${spec.family}: no script called ${id}`);
      if (!hasGlyph(data, script.sample.codePointAt(0))) {
        throw new Error(`${spec.family}: ${name} has no glyph for ${script.name} ${script.sample}`);
      }
    }

    const weight = defaultWeight(data);
    if (weight !== null && weight !== 400) {
      throw new Error(
        `${spec.family}: ${name} is a variable font whose default weight is ${weight}. ` +
          `A project font crosses to the backend by path, so the page would be set at ` +
          `${weight} and nothing would say so.`,
      );
    }

    files.push({ name, face, bytes: data.length, sha256: sha256(data), variable: weight !== null });
    console.log(`  ${name} ${data.length} bytes`);
  }

  return { ...spec, licence: { bytes: licence.length, sha256: sha256(licence) }, files };
}

// --------------------------------------------------------------------- writing

const FACES = {
  regular: "Regular",
  bold: "Bold",
  italic: "Italic",
  "bold-italic": "BoldItalic",
};

const rust = (value) => JSON.stringify(value);
const hex = (code) => `0x${code.toString(16).padStart(4, "0")}`;

function render(rows) {
  const lines = [];
  const put = (line = "") => lines.push(line);

  put("//! The fonts BibleCompose can fetch, and where each one lives.");
  put("//!");
  put("//! Generated by `scripts/font-catalogue.mjs` from the `google/fonts`");
  put("//! repository at one pinned commit. **Do not edit by hand**: every row");
  put("//! carries the byte count and SHA-256 of a file that was fetched and");
  put("//! checked, and a value typed here is a value nobody verified.");
  put("//!");
  put("//! Each font was confirmed to name itself as this table does and to have a");
  put("//! glyph for every script assigned to it. See the generator for what else");
  put("//! it refuses to write.");
  put();
  put("use super::{Face, Licence, Script, SourceFile, Typeface};");
  put();
  put("/// The commit of `google/fonts` every file below was read from.");
  put(`pub const REVISION: &str = ${rust(REVISION)};`);
  put();
  put("/// The one host this application ever contacts (ARCHITECTURE section 7.6).");
  put(`pub const HOST: &str = ${rust(HOST)};`);
  put();
  put("/// The repository path, between the host and the pinned commit.");
  put(`pub const REPOSITORY: &str = ${rust(REPO)};`);
  put();

  put("/// The scripts a suggestion can be made for.");
  // Left alone by rustfmt: this is generated, nobody edits it, and a formatter
  // reflowing fifty language tags across four lines makes a diff that looks
  // like a change to the catalogue whenever one tag is added.
  put("#[rustfmt::skip]");
  put("pub static SCRIPTS: &[Script] = &[");
  for (const s of SCRIPTS) {
    put("    Script {");
    put(`        id: ${rust(s.id)},`);
    put(`        name: ${rust(s.name)},`);
    put(`        sample: '${s.sample === "'" ? "\\'" : s.sample}',`);
    put(`        ranges: &[${s.ranges.map(([a, b]) => `(${hex(a)}, ${hex(b)})`).join(", ")}],`);
    put(`        languages: &[${s.languages.map(rust).join(", ")}],`);
    put(`        subsumed_by: &[${(s.subsumedBy ?? []).map(rust).join(", ")}],`);
    put("    },");
  }
  put("];");
  put();

  put("/// What to offer for each script, in the order to offer it.");
  put("#[rustfmt::skip]");
  put("pub static TYPEFACES: &[Typeface] = &[");
  for (const t of rows) {
    put("    Typeface {");
    put(`        family: ${rust(t.family)},`);
    put(`        directory: ${rust(t.dir)},`);
    put(`        scripts: &[${t.scripts.map(rust).join(", ")}],`);
    put(`        note: ${rust(t.note)},`);
    put("        licence: Licence {");
    put(`            bytes: ${t.licence.bytes},`);
    put(`            sha256: ${rust(t.licence.sha256)},`);
    put("        },");
    put("        files: &[");
    for (const f of t.files) {
      put("            SourceFile {");
      put(`                name: ${rust(f.name)},`);
      put(`                face: Face::${FACES[f.face]},`);
      put(`                bytes: ${f.bytes},`);
      put(`                sha256: ${rust(f.sha256)},`);
      put(`                variable: ${f.variable},`);
      put("            },");
    }
    put("        ],");
    put("    },");
  }
  put("];");
  put();

  return lines.join("\n");
}

// ------------------------------------------------------------------------ main

const normalise = (text) => text.replace(/\r\n/g, "\n").trimEnd();

const check = process.argv.includes("--check");

if (check) {
  // Offline on purpose (NFR-004): re-render from the values the file already
  // records and compare. It catches a hand edit and a generator change, which
  // is what a checked-in generated file can be wrong about without the network.
  const current = readFileSync(OUT, "utf8");
  const parsed = parseBack(current);
  const expected = render(parsed);
  if (normalise(current) !== normalise(expected)) {
    console.error(
      `${OUT} is not what the generator would write.\n` +
        "Run `node scripts/font-catalogue.mjs` and commit the result.",
    );
    process.exit(1);
  }
  console.log(`The font catalogue is current: ${TYPEFACES.length} families.`);
} else {
  const rows = [];
  for (const spec of TYPEFACES) {
    console.log(spec.family);
    rows.push(await resolve(spec));
  }
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, render(rows));
  const total = rows.flatMap((r) => r.files).reduce((a, f) => a + f.bytes, 0);
  console.log(
    `\n${OUT}\n${rows.length} families, ${rows.flatMap((r) => r.files).length} files, ` +
      `${(total / 1048576).toFixed(1)} MB in all`,
  );
}

/**
 * The recorded values, read back out of the generated file.
 *
 * Only what `--check` needs: the per-file numbers it cannot recompute offline.
 * Everything else comes from the spec above, which is the point — a spec edit
 * without a regeneration is exactly the drift being looked for.
 */
function parseBack(text) {
  const rows = [];
  for (const spec of TYPEFACES) {
    const start = text.indexOf(`family: ${rust(spec.family)},`);
    if (start < 0) throw new Error(`${spec.family} is not in the generated file`);
    const end = text.indexOf("    Typeface {", start + 1);
    const block = text.slice(start, end < 0 ? text.length : end);

    const licence = /licence: Licence \{\s*bytes: (\d+),\s*sha256: "([0-9a-f]{64})",/.exec(block);
    if (!licence) throw new Error(`${spec.family} has no licence row`);

    const files = spec.files.map(([name, face]) => {
      const pattern = new RegExp(
        `name: ${rust(name).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")},\\s*` +
          `face: Face::\\w+,\\s*bytes: (\\d+),\\s*sha256: "([0-9a-f]{64})",\\s*variable: (true|false),`,
      );
      const found = pattern.exec(block);
      if (!found) throw new Error(`${spec.family}: ${name} has no row`);
      return {
        name,
        face,
        bytes: Number(found[1]),
        sha256: found[2],
        variable: found[3] === "true",
      };
    });

    rows.push({
      ...spec,
      licence: { bytes: Number(licence[1]), sha256: licence[2] },
      files,
    });
  }
  return rows;
}
