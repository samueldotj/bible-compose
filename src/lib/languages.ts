/**
 * Languages offered when a project is started.
 *
 * A suggestion list, not a schema. `project.language` is a BCP-47 tag and any
 * tag is valid, so the control is a text field with these behind it — a
 * publisher setting a language nobody has heard of is the ordinary case in
 * this field, and a closed dropdown would be the application telling them
 * their language does not exist.
 *
 * What is here is a starting point: the languages a first project is most
 * likely to be in, weighted towards the ones with active Scripture
 * translation. It is not a ranking and it does not need to be complete —
 * anything missing can be typed.
 */
export interface Language {
  /** The BCP-47 tag written into `biblecompose.toml`. */
  readonly tag: string;
  /** What to call it in the list. Its own name where there is room for one. */
  readonly name: string;
}

export const LANGUAGES: readonly Language[] = [
  { tag: "en", name: "English" },
  { tag: "es", name: "Español — Spanish" },
  { tag: "pt", name: "Português — Portuguese" },
  { tag: "fr", name: "Français — French" },
  { tag: "de", name: "Deutsch — German" },
  { tag: "it", name: "Italiano — Italian" },
  { tag: "nl", name: "Nederlands — Dutch" },
  { tag: "ru", name: "Русский — Russian" },
  { tag: "uk", name: "Українська — Ukrainian" },
  { tag: "pl", name: "Polski — Polish" },
  { tag: "ro", name: "Română — Romanian" },
  { tag: "el", name: "Ελληνικά — Greek" },
  { tag: "grc", name: "Ἑλληνική — Ancient Greek" },
  { tag: "he", name: "עברית — Hebrew" },
  { tag: "hbo", name: "Biblical Hebrew" },
  { tag: "ar", name: "العربية — Arabic" },
  { tag: "fa", name: "فارسی — Persian" },
  { tag: "tr", name: "Türkçe — Turkish" },
  { tag: "sw", name: "Kiswahili — Swahili" },
  { tag: "am", name: "አማርኛ — Amharic" },
  { tag: "ha", name: "Hausa" },
  { tag: "yo", name: "Yorùbá" },
  { tag: "ig", name: "Igbo" },
  { tag: "zu", name: "isiZulu" },
  { tag: "hi", name: "हिन्दी — Hindi" },
  { tag: "bn", name: "বাংলা — Bengali" },
  { tag: "ta", name: "தமிழ் — Tamil" },
  { tag: "te", name: "తెలుగు — Telugu" },
  { tag: "ml", name: "മലയാളം — Malayalam" },
  { tag: "kn", name: "ಕನ್ನಡ — Kannada" },
  { tag: "mr", name: "मराठी — Marathi" },
  { tag: "gu", name: "ગુજરાતી — Gujarati" },
  { tag: "pa", name: "ਪੰਜਾਬੀ — Punjabi" },
  { tag: "or", name: "ଓଡ଼ିଆ — Odia" },
  { tag: "si", name: "සිංහල — Sinhala" },
  { tag: "ne", name: "नेपाली — Nepali" },
  { tag: "ur", name: "اردو — Urdu" },
  { tag: "my", name: "မြန်မာ — Burmese" },
  { tag: "th", name: "ไทย — Thai" },
  { tag: "lo", name: "ລາວ — Lao" },
  { tag: "km", name: "ខ្មែរ — Khmer" },
  { tag: "vi", name: "Tiếng Việt — Vietnamese" },
  { tag: "id", name: "Bahasa Indonesia" },
  { tag: "ms", name: "Bahasa Melayu — Malay" },
  { tag: "tl", name: "Tagalog" },
  { tag: "zh", name: "中文 — Chinese" },
  { tag: "ja", name: "日本語 — Japanese" },
  { tag: "ko", name: "한국어 — Korean" },
];

/** The name for a tag, when the list happens to know one. */
export function languageName(tag: string): string | undefined {
  return LANGUAGES.find((l) => l.tag === tag.trim())?.name;
}
