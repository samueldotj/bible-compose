--- biblecompose document class.
--
-- The rendering half of the contract in ADR-002: BibleCompose emits XML in a
-- semantic vocabulary, and this class decides how that vocabulary looks. It is
-- versioned and released with the application (SILE-009).
--
-- Written because SILE's bundled `bible` class cannot do the job: its
-- two-column path is unreachable (any value of `twocolumns`, including
-- "false", is truthy in Lua), and that path never loads `twoside`, so
-- `endPage` calls a nil `oddPage`. Measured in spike/NOTES.md F-5 and F-6.
--
-- What this keeps from upstream is the architecture, which is sound: masters
-- for mirrored page geometry, `twoside`, `infonode` + `chapterverse` for
-- page-scoped reference collection, and `insertions` for the note area. What
-- it does not keep is any of the hardcoding — geometry, the English word
-- "Chapter", and the Gentium font are all options here, because SRS CFG-002
-- requires them to come from settings.
--
-- **`balanced-frames` is deliberately not loaded**, and this is not a
-- preference. It reads a page-break penalty of -17777 or worse as a request to
-- balance the columns now, and `insertions` uses -20000 to force the break
-- after splitting a note that will not fit. The two together are a loop: the
-- balancer re-constrains both columns to the height of what is left in the
-- queue, the split insertion asks for another break, and the document runs to
-- thousands of pages and never finishes. Measured at 1,742 pages in 20 seconds
-- on a two-column page carrying one 29-line note; a note of 15 lines does not
-- reach it, which is why it survived until P4.1 went looking for a note long
-- enough to split. Its own documentation says the algorithm "does not work
-- particularly well"; nothing in this class ever asked it to balance, so all it
-- contributed was the hang.
--
-- Origin: the S0 typesetting spike. It sets a real Bible page in Latin and in
-- Tamil (spike/out/render/), and it carries one known defect — page 1 does not
-- follow the frame chain, so column B is empty on the first page of a document
-- while every later page is correct. Diagnosed in spike/NOTES.md F-8 and owned
-- by P0.4. Do not build on this class without reading that note.

local plain = require("classes.plain")

local class = pl.class(plain)
class._name = "biblecompose"

-- Every class option in one table, because the same list is needed in three
-- places — declaring, setting, and initialising — and three hand-kept copies
-- of a list is three chances for an option to work in one place and be
-- ignored in another.
--
-- The application supplies all of these from resolved settings (CFG-002),
-- passing them with `-O key=value`. The defaults below are what the class does
-- on its own, so it remains usable from a bare `sile` command line; the
-- geometry ones are percentages of the page for that reason, and are what
-- BibleCompose's own defaults were derived from at 6x9in.
local OPTIONS = {
   -- Geometry.
   { key = "columns", kind = "number", default = 2 },
   { key = "gutter", kind = "string", default = "3.5%pw" },
   { key = "margintop", kind = "string", default = "9%ph" },
   { key = "marginbottom", kind = "string", default = "12%ph" },
   { key = "margininner", kind = "string", default = "11%pw" },
   { key = "marginouter", kind = "string", default = "8%pw" },
   { key = "headsep", kind = "string", default = "4%ph" },
   { key = "footsep", kind = "string", default = "3%ph" },
   -- Typography.
   { key = "fontfamily", kind = "string", default = "DejaVu Serif" },
   -- A font the project ships is named by path, not by family (FONT-003).
   -- fontconfig has never heard of it, and S0.5 confirmed SILE loads a face
   -- by filename and subsets and embeds it correctly.
   { key = "fontfile", kind = "string", default = "" },
   { key = "fontsize", kind = "string", default = "9.2pt" },
   { key = "leading", kind = "string", default = "11.2pt" },
   { key = "language", kind = "string", default = "en" },
   -- What the file says about itself (PDF-005). Empty means unset, and an
   -- unset property is not written: an empty `/Title` in a properties panel
   -- reads as an answer, and it is not one.
   { key = "title", kind = "string", default = "" },
   { key = "author", kind = "string", default = "" },
   { key = "subject", kind = "string", default = "" },
   -- How much of the text the file can be pointed at: chapter, verse, none.
   -- Chapters by default; the application's settings file carries the
   -- measurement that decided it.
   { key = "anchors", kind = "string", default = "chapter" },
   { key = "hyphenate", kind = "boolean", default = true },
   -- What appears on the page. Each of these hides something the document
   -- still carries: the model is unchanged and the XML is unchanged, because
   -- ADR-002 says the document says what and the class says how. Re-running
   -- with verse numbers back on needs no re-emission.
   { key = "chapternumbers", kind = "boolean", default = true },
   { key = "versenumbers", kind = "boolean", default = true },
   -- Verse 1 goes unnumbered where the chapter number already marks it.
   { key = "hidefirstverse", kind = "boolean", default = false },
   -- Whether a chapter's opening initial drops into the text, and how far.
   -- Which run *is* the initial arrives in the document as `<initial>`: a
   -- syllable in an Indic script is several characters, and telling them
   -- apart takes Unicode segmentation this class has not got.
   { key = "dropcaps", kind = "boolean", default = false },
   -- What drops: "first_letter" or "chapter_number".
   { key = "dropcapof", kind = "string", default = "first_letter" },
   { key = "dropcaplines", kind = "string", default = "3" },
   -- How a paragraph and a line of verse are set.
   { key = "justify", kind = "boolean", default = true },
   { key = "poetryindent", kind = "boolean", default = true },
   { key = "footnotes", kind = "boolean", default = true },
   { key = "crossrefs", kind = "boolean", default = true },
   -- The apparatus. Two sequences, because a page carrying both has to say
   -- which mark belongs to which: numbers, letters, symbols, or none.
   { key = "footnotecallers", kind = "string", default = "numbers" },
   { key = "crossrefcallers", kind = "string", default = "letters" },
   -- per_chapter, per_book, never.
   { key = "restartnotes", kind = "string", default = "per_chapter" },
   -- note_area, inline, end_of_paragraph.
   { key = "crossrefplacement", kind = "string", default = "note_area" },
   -- Six slots, three at the top of the page and three at the foot. Each
   -- holds one of: empty, page_number, reference_range, first_reference,
   -- last_reference, book_name, alt_book_name. Positions rather than
   -- switches, because where a thing goes is as much a decision as whether
   -- it is there, and only the settings layer can make it.
   -- One set per side of the spread: verso is the left-hand (even) page,
   -- recto the right-hand (odd). `endPage` picks by the page's parity. Each
   -- is a template — text with fields in braces — that `slot_content` reads.
   { key = "versoheaderleft", kind = "string", default = "{Book}" },
   { key = "versoheadercenter", kind = "string", default = "" },
   { key = "versoheaderright", kind = "string", default = "{Range}" },
   { key = "versofooterleft", kind = "string", default = "" },
   { key = "versofootercenter", kind = "string", default = "{Page}" },
   { key = "versofooterright", kind = "string", default = "" },
   { key = "rectoheaderleft", kind = "string", default = "{Book}" },
   { key = "rectoheadercenter", kind = "string", default = "" },
   { key = "rectoheaderright", kind = "string", default = "{Range}" },
   { key = "rectofooterleft", kind = "string", default = "" },
   { key = "rectofootercenter", kind = "string", default = "{Page}" },
   { key = "rectofooterright", kind = "string", default = "" },
}

-- Option values arrive as strings. `SU.boolean` is the coercion upstream is
-- missing; without it "false" is truthy and selects two columns.
local function coerce (spec, value)
   if value == nil then
      return spec.default
   end
   if spec.kind == "number" then
      return tonumber(value) or spec.default
   end
   if spec.kind == "boolean" then
      return SU.boolean(value, spec.default)
   end
   return value
end

function class:declareOptions ()
   plain.declareOptions(self)
   for _, spec in ipairs(OPTIONS) do
      self:declareOption(spec.key, function (_, value)
         if value ~= nil then
            -- Coerce here, not in setOptions: plain.setOptions runs these
            -- setters afterwards with the raw document strings, so anything
            -- converted earlier gets overwritten by a string. This is the
            -- same defect as upstream's truthy "false". NOTES.md F-6.
            self._bcopts[spec.key] = coerce(spec, value)
         end
         return self._bcopts[spec.key]
      end)
   end
end

function class:setOptions (options)
   self._bcopts = self._bcopts or {}
   for _, spec in ipairs(OPTIONS) do
      self._bcopts[spec.key] = coerce(spec, options[spec.key])
   end
   plain.setOptions(self, options)
end

--- The one insertion class this document has, and the frame it fills.
--
-- One note area spanning the measure, and not one per column, which was tried
-- and is wrong here. A note is called while the *paragraph* is being typeset,
-- and SILE does not decide which column that paragraph's lines land in until
-- the page is broken — so at the moment a note would have to choose a column
-- frame, its own caller does not yet have one. Choosing anyway put notes under
-- column A whose callers had moved to column B.
local NOTES = "footnotes"

--- The most of a page the note area may take, in points.
--
-- **Absolute, not `75%ph`.** A relative length survives the fitting test, which
-- only compares it, and dies in the splitting path, which subtracts the top box
-- from it: `measurement.__sub` refuses a relative operand. So a note short
-- enough to fit typesets and a note long enough to split takes the class down
-- with `_error_if_relative`, which is the worst possible place for the
-- difference to show. Upstream's `footnotes` package has the same latent
-- defect; it goes unnoticed because a note that long is rare.
local function note_ceiling ()
   return SILE.types.length(SILE.types.measurement("75%ph"):tonumber())
end

-- Frames. `inner`/`outer` are resolved per master so the geometry mirrors
-- across the spread, which is what makes a bound book read correctly.
function class:_frameset (inner, outer)
   local o = self._bcopts
   local twocol = o.columns >= 2
   -- **The parentheses are load-bearing.** SILE's frame grammar makes `-`
   -- right-associative (`core/frameparser.lua`: `minus` recurses into
   -- `additive` on its right), so `100%ph - a - b` means `100%ph - (a - b)`.
   -- Written without them at 6x9in this read as 91%ph rather than 85%ph, and
   -- the whole text block sat 38.88pt lower than the configured bottom margin —
   -- on every page, silently, since the note frames are what the content frames
   -- derive their bottom from.
   local notefoot = "100%ph - (" .. o.marginbottom .. " + " .. o.footsep .. ")"
   local outermost = twocol and "right(contentB)" or "right(contentA)"
   local frames = {
      runningHead = {
         left = "left(contentA)",
         right = outermost,
         top = "top(contentA) - " .. o.headsep,
         bottom = "top(contentA)",
      },
      -- The folio sits *below* the text block, inside the bottom margin.
      -- Deriving its bottom from marginbottom (as upstream's arithmetic
      -- invites) puts bottom above top and overfills the frame by its own
      -- height on every page. NOTES.md F-7.
      --
      -- Off the note frame's *fixed* bottom rather than its top, so a page with
      -- notes and a page without put the folio in the same place.
      folio = {
         left = "left(contentA)",
         right = outermost,
         top = "bottom(footnotes) + " .. o.footsep,
         bottom = "bottom(footnotes) + " .. o.footsep .. " + 4%ph",
      },
      footnotes = {
         left = "left(contentA)",
         right = outermost,
         height = "0",
         bottom = notefoot,
      },
   }

   if twocol then
      frames.contentA = {
         left = inner,
         right = "left(gutter)",
         top = o.margintop,
         bottom = "top(footnotes)",
         next = "contentB",
      }
      frames.gutter = {
         left = "right(contentA)",
         right = "left(contentB)",
         width = o.gutter,
      }
      frames.contentB = {
         left = "right(gutter)",
         width = "width(contentA)",
         right = "100%pw - " .. outer,
         top = o.margintop,
         bottom = "top(footnotes)",
      }
   else
      frames.contentA = {
         left = inner,
         right = "100%pw - " .. outer,
         top = o.margintop,
         bottom = "top(footnotes)",
      }
   end
   return frames
end

function class:_init (options)
   -- Parse options and install the frameset BEFORE plain._init, because that
   -- is what lays out page 1. Defining masters afterwards (as upstream does)
   -- leaves the first page on plain's single full-width frame with no `next`
   -- chain, so column B is empty on page 1 and correct everywhere after.
   -- NOTES.md F-8.
   -- A 6x9in trim rather than plain's A4: this is a Bible class, and a
   -- default that has to be overridden on every document is not a default.
   options = options or {}
   if not options.papersize then
      options.papersize = "6in x 9in"
   end

   self._bcopts = {}
   for _, spec in ipairs(OPTIONS) do
      self._bcopts[spec.key] = coerce(spec, options[spec.key])
   end
   self.defaultFrameset = self:_frameset(self._bcopts.margininner, self._bcopts.marginouter)
   self.firstContentFrame = "contentA"

   plain._init(self, options)
   -- `color` is loaded unconditionally rather than when a style asks for one:
   -- styles are read after the class is initialised, and a package loaded
   -- halfway through a document is a difference between two builds of the
   -- same project (DET-001).
   self:loadPackage("color")
   self:loadPackage("masters")
   self:loadPackage("infonode")
   self:loadPackage("chapterverse")
   self:loadPackage("image")
   -- Destinations, bookmarks and document properties (PDF-005, SCR-008).
   self:loadPackage("pdf")
   -- The dropped initial, when `dropcaps` asks for one.
   self:loadPackage("dropcaps")
   -- **A paragraph shorter than its initial is padded to clear it.** The
   -- package makes room for the initial by indenting the paragraph's own
   -- lines and nothing else: a one-line opening — a psalm's first verse, say
   -- — leaves the initial hanging two lines below the paragraph, and whatever
   -- follows sets straight through it. Measured: a section heading's baseline
   -- 5.6pt above the initial's own.
   --
   -- `boxUpNodes` is where a paragraph becomes lines, so it is where their
   -- number is known. When the paragraph carrying the initial comes through
   -- short, the missing lines are added as glue behind it.
   self:registerPostinit(function (_)
      local box_up = SILE.typesetter.boxUpNodes
      SILE.typesetter.boxUpNodes = function (typesetter)
         local vboxes = box_up(typesetter)
         -- Read off `SILE.scratch` directly rather than through `scratch()`:
         -- that helper is a file-level local defined further down, and a
         -- closure made here would resolve the name to a global that does
         -- not exist. The table is created by the first `scratch()` call, so
         -- before that there is nothing pending and nothing to do.
         local s = SILE.scratch.biblecompose
         -- A footnote called from the opening paragraph is boxed up first,
         -- mid-paragraph, and is not the paragraph.
         if s and s.initial_lines and not s.in_note then
            local want = s.initial_lines
            s.initial_lines = nil
            local got = 0
            for _, v in ipairs(vboxes) do
               if v.is_vbox then
                  got = got + 1
               end
            end
            if got > 0 and got < want then
               local bs = SILE.settings:get("document.baselineskip").height:tonumber()
               vboxes[#vboxes + 1] = SILE.types.node.vglue((bs * (want - got)) .. "pt")
            end
         end
         return vboxes
      end
   end)

   self:registerPostinit(function (self_)
      local o = self_._bcopts
      self_.firstContentFrame = "contentA"
      self_:defineMaster({
         id = "right",
         firstContentFrame = "contentA",
         frames = self_:_frameset(o.margininner, o.marginouter),
      })
      self_:defineMaster({
         id = "left",
         firstContentFrame = "contentA",
         frames = self_:_frameset(o.marginouter, o.margininner),
      })

      -- Loaded for BOTH column counts. Upstream loads it only on the
      -- single-column path, which is what makes `endPage` fail in two.
      self_:loadPackage("twoside", { oddPageMaster = "right", evenPageMaster = "left" })

      -- **Page 1 has to be put on a master the same way every later page is**,
      -- and this is not cosmetic.
      --
      -- `defineMaster` builds its frames with `SILE.newFrame`, which registers
      -- each one in the global `SILE.frames` under its id — so defining the two
      -- masters above replaces `SILE.frames.contentA` with the left master's
      -- frame. Meanwhile the typesetter was created, one postinit earlier,
      -- holding the frame `initialFrame` copied out of `declareFrames`. From
      -- page 2 on, `twoside`'s newpage hook switches masters and the class
      -- repoints the typesetter, and the two agree. On page 1 nothing switched,
      -- and they did not.
      --
      -- The note machinery is where that split showed. `setShrinkage` takes the
      -- note height off `SILE.getFrame("contentA")`; `getTargetLength` reads it
      -- back off `SILE.typesetter.frame`. Two objects, one id: on page 1 the
      -- steal went into a frame the page was not being set in, the target never
      -- shrank, and the last lines of the column were typeset straight over the
      -- first lines of the notes — 70pt of overprinting on the opening page of
      -- Mark, and nothing wrong on any page after it. Spike F-10's first
      -- defect, and this is the whole of the fix.
      --
      -- Through `initialFrame` rather than `switchMaster`, which looks like the
      -- obvious call and is a trap: it installs the master's *own* frame
      -- objects, so the first page's shrinkage is committed onto the master
      -- itself and every later page that uses it starts already short by the
      -- notes of page 1. `initialFrame` deep-copies, which is exactly what
      -- `newPage` does for every page after this one.
      self_.pageTemplate = SILE.scratch.masters["right"]
      SILE.typesetter:initFrame(self_:initialFrame())
   end)

   -- These load here, not in postinit: page 1 is laid out before postinit runs,
   -- so a note frame or a balancer registered there never applies to it.
   -- NOTES.md F-8.
   --
   -- `insertions` directly, not the `footnotes` package on top of it. That
   -- package hardwires one insertion class named "footnote", a caller that is
   -- always the value of a global arabic counter, and a note body that always
   -- begins with that counter and a full stop. All three are settings here —
   -- there are two apparatus with two sequences, USFM notes carry callers of
   -- their own, and the sequence restarts — so what is left of the package
   -- after overriding it is the two lines below.
   self:loadPackage("insertions")
   self:loadPackage("raiselower")
   self:loadPackage("counters")
   local steal = self._bcopts.columns >= 2 and { "contentA", "contentB" } or { "contentA" }
   self:initInsertionClass(NOTES, {
      insertInto = NOTES,
      -- Every content frame, because the area spans them all. Upstream's
      -- `book` steals from one, which is right for one column and puts a note
      -- called in the left column under the right one in two.
      stealFrom = steal,
      -- Three quarters, so a page of notes is still a page of Scripture. Past
      -- it the insertion splits and the rest goes to the next page, which is
      -- what a long note is supposed to do. `_note_headroom` lowers it for the
      -- rest of a page once the first column is committed.
      maxHeight = note_ceiling(),
      topBox = SILE.types.node.vglue("2ex"),
      interInsertionSkip = SILE.types.length("1ex"),
   })

   if self._bcopts.columns >= 2 then
      -- A narrow column needs a looser breaker than a full measure does.
      SILE.settings:set("linebreak.tolerance", 9000)
      -- After the insertions package's own frame-break hook, which is what
      -- commits the column's height: this reads that committed height back.
      self:registerPostinit(function (self_)
         SILE.typesetter:registerFrameBreakHook(function (_, nodelist)
            self_:_note_headroom()
            return nodelist
         end)
      end)
   end

   -- The folio package prints the page number into the folio frame by itself
   -- and centred. That frame is now three slots wide and the number is only
   -- one of the things that can go in it, so the package is silenced and this
   -- class fills the frame in `endPage`. The frame stays either way: the text
   -- block's bottom is derived from it, and removing it would move type on the
   -- page as a side effect of hiding a number.
   SILE.scratch.counters.folio.off = true
end

--- What the note area may still grow to on this page.
--
-- The residue of spike F-10's first defect, and the only part of it that
-- survives one pass over the document.
--
-- SILE fills column A, breaks it, then fills column B. A note steals its height
-- from the columns when the page builder reaches it, so column A reserves for
-- every note seen while column A was being filled — including the ones whose
-- callers end up in column B, which is conservative and right. What it cannot
-- reserve for is a note called *after* the break, because column A is set by
-- then. Left alone, the note area grows upward past column A's last line and
-- prints over it: measured on Mark, 22 of 47 pages.
--
-- So once a column is committed, the note area is capped at the space actually
-- left beneath it. A note that no longer fits splits, and the remainder goes to
-- the next page — which is what the insertions machinery does with a note too
-- long for its page anyway, and what P4.1 asks a long note to do. Never below
-- what is already in the area, or a note already on the page would be asked to
-- shrink, which nothing can do.
function class:_note_headroom ()
   local left = SILE.getFrame(SILE.typesetter.frame.id)
   local area = SILE.getFrame(NOTES)
   local options = SILE.scratch.insertions.classes[NOTES]
   if not (left and area and options) then
      return
   end
   local room = area:bottom():tonumber() - left:bottom():tonumber()
   -- A plain function on the class, not a method: the insertions package
   -- exports it taking the class *name* as its first argument, unlike the
   -- two beside it.
   local used = self.thisPageInsertionBoxForClass(NOTES)
   used = used and (used.height:tonumber() + used.depth:tonumber()) or 0
   options.maxHeight = SILE.types.length(math.max(room, used))
end

--- Undo it, so the next page starts with the whole area available again.
function class:newPage ()
   local options = SILE.scratch.insertions and SILE.scratch.insertions.classes[NOTES]
   if options then
      options.maxHeight = note_ceiling()
   end
   return plain.newPage(self)
end

-- Declared here and defined below `style` and `face`, which they need and
-- which are locals further down the file. `endPage` and `registerCommands`
-- close over these names, so the names have to exist before they do — a Lua
-- upvalue is bound where the closure is written, not where it runs, so a
-- `local function` further down would be a nil global here.
local slot_content, set_line, restart_notes, carry_reference
-- `scratch` and the two anchor helpers, for the same reason: the chapter
-- and verse commands name a place in Scripture, and both are registered
-- above the state they read it from.
local scratch, anchor, destination, o_anchors, open_page

function class:endPage ()
   local o = self._bcopts
   -- Which side of the spread this page is. `twoside` answers by the folio's
   -- parity; page 1 is a recto, as in every bound book.
   local side = (self.oddPage and self:oddPage()) and "recto" or "verso"
   set_line(SILE.getFrame("runningHead"), {
      o[side .. "headerleft"], o[side .. "headercenter"], o[side .. "headerright"],
   })
   set_line(SILE.getFrame("folio"), {
      o[side .. "footerleft"], o[side .. "footercenter"], o[side .. "footerright"],
   })
   -- **After the head, not before.** The head for this page is built from the
   -- references collected on it; this records the last of them so that the
   -- *next* page knows which verse it opens in the middle of. Doing it first
   -- would answer the next page's question on this one.
   carry_reference()
   return plain.endPage(self)
end

function class:registerCommands ()
   plain.registerCommands(self)

   -- chapterverse stores content[1] verbatim and later tostring()s it. Reached
   -- through \define, content[1] is a content node, not a string, so the
   -- running head renders "table: 0x55f…" instead of "1:14". Flattening to
   -- text at the boundary is the fix. NOTES.md F-9.
   local function flat (content)
      return { SU.ast.contentToString(content) }
   end

   -- Saving the number and printing it are separate, and only printing is
   -- optional: the running head's reference range is a different setting, and
   -- a page whose verse numbers are hidden still knows which verses are on it.
   self:registerCommand("bc:verse", function (options, content)
      -- The number is withheld, never the verse: `save-verse-number` still
      -- runs below, so a running head asking for the reference range on a page
      -- that starts at verse 1 still knows where it starts.
      -- Anchored whether or not the number is printed, for the same reason
      -- the chapter is: the anchor is where the verse *is*, and that does not
      -- depend on whether a reader can see its number.
      local s = scratch()
      destination(anchor(s.code, s.chapter, options.start or options.n), "verse")
      -- With a dropped initial the opening is already marked — a superscript
      -- "1" wedged between the chapter's own line and a three-line letter is
      -- a second marker for the same place — so drop caps imply the setting.
      local first = (self._bcopts.hidefirstverse or self._bcopts.dropcaps)
         and tostring(options.start) == "1"
      if self._bcopts.versenumbers and not first then
         SILE.call("bc:verse-number", options, content)
      end
      SILE.call("save-verse-number", options, flat(content))
   end)

   self:registerCommand("bc:chapter", function (options, content)
      -- Where the chapter begins, for every chapter but a book's first: the
      -- book's own opening decides that one (see `book`). A page wins over a
      -- column, since a new page is a new column too.
      -- Through `scratch` and not `style`: this closure is written before
      -- `style` is declared, and would read a nil global.
      local s0 = scratch()
      local cs = (s0.styles or {}).chapter or {}
      s0.chapters_in_book = (s0.chapters_in_book or 0) + 1
      if s0.chapters_in_book > 1 then
         local where = cs.new_page or "continue"
         if where ~= "continue" then
            open_page(where)
         elseif SU.boolean(cs.new_column, false) then
            SILE.typesetter:leaveHmode()
            -- A forced break moves to the next frame, which in two columns
            -- is the next column. Not `\eject`: it is built on `reak`,
            -- and `reak` here is USFM's ``, a blank line. In one column
            -- there is no next frame, and only a `supereject` reaches the
            -- next page.
            SILE.call("vfill")
            if tonumber(self._bcopts.columns) == 1 then
               SILE.call("penalty", { penalty = -20000 })
            else
               SILE.call("penalty", { penalty = -10000 })
            end
         end
      end
      SILE.call("save-chapter-number", options, flat(content))
      -- Outside the `chapternumbers` guard below, and that is the point:
      -- SCR-001 says hiding a number does not remove its anchor, and an
      -- edition set without chapter numbers is still navigable by chapter.
      local s = scratch()
      s.chapter = tostring(options.n or "")
      local dest = anchor(s.code, s.chapter)
      destination(dest, "chapter")
      if dest ~= "" and s.chapter ~= "" and o_anchors() ~= "none" then
         SILE.call("pdf:bookmark", {
            dest = dest,
            title = (scratch().book_title or s.code) .. " " .. s.chapter,
            level = 2,
         })
      end
      -- The restart happens whether or not the number is printed. Hiding
      -- chapter numbers is a decision about the page; where a note sequence
      -- begins again is a decision about the apparatus, and a reader-format
      -- edition that hides one still has chapters.
      restart_notes("per_chapter")
      if self._bcopts.chapternumbers then
         SILE.call("bc:chapter-number", options, content)
      end
   end)

   self:registerCommand("bc:book", function (options, content)
      SILE.call("save-book-title", options, flat(content))
   end)

   self:registerXmlCommands()
end

-- ---------------------------------------------------------------------------
-- The XML vocabulary — the other half of ADR-002's contract.
--
-- BibleCompose emits XML; SILE maps each element name to a command of the same
-- name; these are those commands. Scripture arrives as text nodes and never as
-- syntax, which is the whole reason the contract is XML rather than SIL.
--
-- Two things are deliberate here.
--
-- Whitespace between BLOCK elements is formatting in the emitted file, not
-- content, so block containers process only their element children. Inside a
-- paragraph every character is Scripture and is processed verbatim.
--
-- Appearance arrives in the document, as a `<styles>` block of resolved
-- values (P3.4). It used to be a table here; it is not any more, because
-- STY-001 makes it a publisher's to change and a table in this file is not.
-- The document still says *what* and this file still says *how* — but "how"
-- is now a lookup rather than a constant.
--
-- Values arrive as attributes and never as command fragments (ADR-002). A
-- style file travels with a project, by email, from a third party, and a
-- property that could carry `\command` would make one executable.
-- ---------------------------------------------------------------------------

--- This class's document-scoped state, created once.
--
-- `SILE.scratch` is where SILE keeps per-document state, and a second document
-- processed in the same process must not inherit the first one's appearance or
-- its book count.
--
-- One initialiser, because two is how the first version of this broke: the
-- style lookup created the table without `books`, so the guard that gives the
-- second and later books a page break compared a number with nil.
function scratch ()
   local s = SILE.scratch.biblecompose
   if not s then
      s = {}
      SILE.scratch.biblecompose = s
   end
   s.books = s.books or 0
   s.styles = s.styles or {}
   -- How many marks each apparatus has issued since it last restarted, and the
   -- references waiting to be set under the paragraph that called them.
   s.issued = s.issued or { note = 0, ref = 0 }
   s.pending_refs = s.pending_refs or {}
   s.altbooks = s.altbooks or {}
   -- Where in Scripture the typesetter currently is, for naming destinations.
   -- The book code is carried rather than looked up because a verse knows its
   -- own number and nothing else about where it sits.
   s.code = s.code or ""
   s.chapter = s.chapter or ""
   return s
end

--- The name of one place in the text (SCR-008).
---
--- `JHN.3.16` — the book code, the chapter, the verse; the form Paratext and
--- every reference parser already speak, so a link written against it later
--- needs nothing from this file. A chapter is `JHN.3` and a book is `JHN`,
--- which makes a prefix of a verse name the name of the thing containing it.
---
--- The book code and not the printed name: a name is a setting, translated,
--- and may hold a space; a code is three characters and is the same in every
--- edition of the same Scripture.
function anchor (...)
   local parts = { ... }
   local out = {}
   for _, part in ipairs(parts) do
      local text = tostring(part or "")
      if text ~= "" then
         out[#out + 1] = text
      end
   end
   return table.concat(out, ".")
end

--- Put a destination here, if there is anywhere to put it and anyone asked.
---
--- Guarded on the book code because a document may carry a book that never
--- declared one, and a destination named `.3.16` is worse than none: it
--- collides with every other book missing a code in the same document, and a
--- PDF with two destinations of one name resolves to whichever came last.
---
--- `depth` is what the caller is: `"chapter"` for a book or a chapter,
--- `"verse"` for a verse. A verse anchor is skipped unless it was asked for,
--- because it is the one that costs — 15% of the build and 14% of the file on
--- a 4,950-verse document, measured.
--- What the project asked for. A function because the class options are not
--- reachable until a document is being processed.
function o_anchors ()
   return SILE.documentState.documentClass._bcopts.anchors
end

function destination (name, depth)
   local want = SILE.documentState.documentClass._bcopts.anchors
   if want == "none" then
      return
   end
   if depth == "verse" and want ~= "verse" then
      return
   end
   if name and name ~= "" then
      SILE.call("pdf:destination", { name = name })
   end
end

--- The resolved styles, keyed by selector.
local function sheet ()
   return scratch().styles
end

--- One selector's properties, or an empty table.
--
-- Empty is a real answer and not a missing one: the built-in sheet leaves most
-- paragraph markers unstyled, meaning "renders as body text".
local function style (selector)
   return sheet()[selector] or {}
end

--- The font attributes a style asks for, or nil when it asks for none.
--
-- `nil` rather than an empty table so a caller can skip the `\font` call
-- entirely — wrapping every run in a font switch that changes nothing costs a
-- shaper round trip per element.
local function face (s)
   local f = nil
   local function want (key, value)
      if value ~= nil then
         f = f or {}
         f[key] = value
      end
   end
   -- One or the other. A project font arrives as a path because fontconfig
   -- has never heard of it (FONT-003); anything else arrives as a family
   -- name, so its bold and italic remain reachable.
   want("filename", s.font_file)
   if s.font_file == nil then
      want("family", s.font_family)
   end
   want("size", s.font_size)
   want("weight", s.weight and tonumber(s.weight) or nil)
   if SU.boolean(s.italic, false) then
      want("style", "italic")
   end
   if SU.boolean(s.smallcaps, false) then
      want("features", "+smcp")
   end
   return f
end

--- Run `body` with a style's font and ink applied, if it asks for either.
--
-- Both are wrapped rather than set, so they end where the element ends: a
-- red-letter `\wj` must not leave the rest of the verse red, and a heading in
-- a display face must not carry it into the paragraph beneath.
--
-- Colour inside the font call rather than outside it because the font switch
-- is what changes the shaping, and a colour that wrapped it would be a group
-- around a group for nothing.
--- The references `chapterverse` collected on the page being closed.
--
-- Empty rather than nil when there are none, so a caller can index it without
-- asking first. A page of front matter has no verse on it and that is an
-- ordinary thing for a page to be.
local function collected ()
   local info = SILE.scratch.info
   return (info and info.thispage and info.thispage.references) or {}
end

--- What is on this page, including the verse it opened in the middle of.
--
-- `chapterverse` records a reference where a verse *number* is typeset, so a
-- page wholly inside one long verse collects nothing at all — and the head
-- went blank on it. Measured on a book whose second verse ran to fourteen
-- pages: the first page read `1:1–1:2` and the thirteen after it read nothing,
-- which is the head saying a page has no Scripture on it while a reader is
-- looking at Scripture.
--
-- So the last reference of each page is carried into the next, and a page that
-- collects none reports the verse it is still in. That is what a reader
-- searching for 1:2 needs the head to say.
local function page_references ()
   local refs = collected()
   if #refs > 0 then
      return refs
   end
   local carried = scratch().carried
   return carried and { carried } or {}
end

--- Which book this page is in.
--
-- From the page's own references and not from `chapterverse`'s running book,
-- which is a different thing: the running value is whatever was most recently
-- *typeset*, and SILE sets material well ahead of outputting the page it lands
-- on. With two books in a document that showed as a first page carrying
-- Genesis and headed `John 1:1–1:5` — the head naming a book the reader cannot
-- see, which is worse than naming none.
--
-- Every reference carries the book it belongs to, because `save-verse-number`
-- records one at the moment the verse number is set. So the page's own
-- collection answers it, and the running value is only a fallback for a page
-- with no verse on it at all and none carried into it — front matter, where it
-- is the best guess available and cannot be wrong about a page of Scripture.
local function page_book ()
   local first = page_references()[1]
   if first and first.book then
      return tostring(first.book)
   end
   local running = SILE.scratch.chapterverse and SILE.scratch.chapterverse.book
   return running and tostring(running) or nil
end

--- Remember where this page ended, for the next one. See [`page_references`].
function carry_reference ()
   local refs = collected()
   if #refs > 0 then
      scratch().carried = refs[#refs]
   end
end

--- One reference, as `chapter:verse`.
--
-- **Never with the book name**, because the book name is its own head slot,
-- and this is written here rather than passed as `showbook=false` because that
-- argument does not work. Upstream's `first-reference` takes options and
-- discards them — `function (_, _)`, then `format-reference` with an empty
-- table, which defaults `showbook` to true. So the same argument was honoured
-- by `last-reference` and ignored by `first-reference`, and the default head
-- read `Mark        Mark 1:1–1:10`: the book twice, once from each end of the
-- line, which is the arrangement the six slots exist to let a publisher avoid.
--
-- Overriding `format-reference` does not help either: the class registers its
-- commands during `plain._init`, and `chapterverse` is loaded after that and
-- registers its own on top. Reading the collection directly has no ordering to
-- get wrong.
--- `chapter:verse`, or nothing for a page that has no reference.
local function reference_text (ref)
   if not (ref and ref.chapter) then
      return nil
   end
   return tostring(ref.chapter) .. ":" .. tostring(ref.verse)
end

--- What one field of a head or foot template reads on this page, or nil for
-- a field this page has nothing for. The names are the resolver's
-- `HEAD_FIELDS`, compared the way it compares them: lower-case, without
-- underscores.
local function field_value (name)
   local key = name:gsub("_", ""):lower()
   local s = scratch()
   local refs = page_references()
   local first, last = refs[1], refs[#refs]
   if key == "page" then
      -- Through the counters package rather than the raw value, so a project
      -- numbering its front matter in roman gets roman here too.
      local counters = SILE.documentState.documentClass.packages.counters
      return counters:formatCounter(SILE.scratch.counters.folio)
   elseif key == "book" then
      return page_book()
   elseif key == "altbook" then
      local book = page_book()
      local alt = book and s.altbooks[book]
      return (alt and alt ~= "") and alt or nil
   elseif key == "range" then
      if not first then
         return nil
      end
      -- A page holding one verse gets that verse, not `1:5–1:5`: a range
      -- whose ends are the same place is not a range.
      if last.chapter ~= first.chapter or last.verse ~= first.verse then
         return reference_text(first) .. "–" .. reference_text(last)
      end
      return reference_text(first)
   elseif key == "firstreference" then
      return reference_text(first)
   elseif key == "lastreference" then
      return reference_text(last)
   elseif key == "firstchapter" then
      return first and first.chapter and tostring(first.chapter) or nil
   elseif key == "firstverse" then
      return first and first.chapter and tostring(first.verse) or nil
   elseif key == "lastchapter" then
      return last and last.chapter and tostring(last.chapter) or nil
   elseif key == "lastverse" then
      return last and last.chapter and tostring(last.verse) or nil
   end
   return nil
end

--- What one slot puts on the page: its template, with each field replaced by
-- what it reads here.
--
-- Nothing at all for an empty template, and nothing for a template whose
-- fields *all* have nothing on this page — a page with no verse on it has no
-- reference range, and a head reading "–" or ":" would be worse than a head
-- reading nothing. A field with nothing among fields with something is simply
-- left out: "{Book} {Range}" on such a page reads the book alone. The
-- resolver has already refused a template that names no real field or leaves
-- a brace open, so this reads what it is given.
function slot_content (template)
   local out, any_field, any_value = {}, false, false
   local i, n = 1, #template
   while i <= n do
      local c = template:sub(i, i)
      local following = template:sub(i + 1, i + 1)
      if (c == "{" or c == "}") and following == c then
         -- A doubled brace is a brace of the publisher's own.
         out[#out + 1] = c
         i = i + 2
      elseif c == "{" then
         local close = template:find("}", i, true)
         if not close then
            out[#out + 1] = template:sub(i)
            break
         end
         local v = field_value(template:sub(i + 1, close - 1))
         any_field = true
         if v then
            any_value = true
            out[#out + 1] = v
         end
         i = close + 1
      else
         out[#out + 1] = c
         i = i + 1
      end
   end
   if any_field and not any_value then
      return
   end
   local text = table.concat(out)
   if text ~= "" then
      SILE.typesetter:typeset(text)
   end
end

--- Set one line of three slots into a frame.
--
-- `\hfill` between the parts is what makes three slots out of one line: the
-- left one starts at the margin, the right one ends at it, and the middle one
-- lands between them wherever they leave it. An empty slot contributes
-- nothing but its glue, so two empties and a centre still centre.
function set_line (frame, slots)
   if not frame then
      return
   end
   local anything = false
   for _, slot in ipairs(slots) do
      if slot ~= "" then
         anything = true
      end
   end
   if not anything then
      return
   end

   SILE.typesetNaturally(frame, function ()
      SILE.settings:set("current.parindent", SILE.types.node.glue())
      SILE.settings:set("document.lskip", SILE.types.node.glue())
      SILE.settings:set("document.rskip", SILE.types.node.glue())
      SILE.settings:set("typesetter.parfillskip", SILE.types.node.glue())
      SILE.call("font", face(style("head")) or {}, function ()
         for i, slot in ipairs(slots) do
            if i > 1 then
               SILE.call("hfill")
            end
            slot_content(slot)
         end
      end)
      SILE.call("par")
   end)
end


local function styled (selector, body)
   local s = style(selector)
   local f = face(s)

   local inner = body
   if s.color then
      inner = function ()
         SILE.call("color", { color = s.color }, body)
      end
   end

   if f then
      SILE.call("font", f, inner)
   else
      inner()
   end
end

--- The alignment a style asks for, as the command that produces it.
--
-- Justified is SILE's own default for a paragraph, so it is the absence of a
-- command rather than one of its own.
local ALIGNMENT = {
   center = "center",
   end_ = "raggedleft",
   start = "raggedright",
}

local function alignment (s)
   -- A project set ragged overrides nothing a style says about centring: a
   -- centred line is still centred, and only the ones that would have been
   -- justified become ragged.
   if s.align == nil and not SILE.documentState.documentClass._bcopts.justify then
      return ALIGNMENT.start
   end
   if s.align == "center" then
      return ALIGNMENT.center
   elseif s.align == "end" then
      return ALIGNMENT.end_
   elseif s.align == "start" then
      return ALIGNMENT.start
   end
   return nil
end

--- Begin a page: the next one, or one on the side asked for, with a blank
-- page (no head, no folio) in between when the side needs it. `twoside`'s
-- `open-spread` does the measuring — it cannot know which page comes next
-- without ejecting and looking.
function open_page (where)
   SILE.typesetter:leaveHmode()
   if where == "left" then
      SILE.call("open-spread", { double = false, odd = false, blank = true })
   elseif where == "right" then
      SILE.call("open-spread", { double = false, odd = true, blank = true })
   else
      SILE.call("supereject")
   end
end

--- Set `body` as the first thing in its paragraph, though it is not.
--
-- The chapter's anchor — and, for an initial, the verse's — comes before it:
-- a PDF destination and the running head's note of where the verse is. Those
-- open the paragraph: SILE's `newPar` runs on the first node, pushes the
-- paragraph indent, and copies `current.hangIndent` into what the
-- line-breaker reads. By the time `\dropcap` sets it, nobody copies it
-- again, and the result is the dropped thing hanging in the margin with the
-- text at the margin beside it. Upstream never sees this because nothing
-- precedes `\dropcap` there.
--
-- So the paragraph is put back to unopened: its nodes are taken off, `body`
-- opens it — the package sets the hang and no indent — and the anchors go
-- back in after, a few points to the right of where they were. The first two
-- nodes are `newPar`'s own, the zero box and the indent glue, and are not
-- kept.
local function dropped (body)
   local nodes = SILE.typesetter.state.nodes
   local anchors = {}
   for i = #nodes, 3, -1 do
      anchors[#anchors + 1] = table.remove(nodes, i)
   end
   for i = #nodes, 1, -1 do
      nodes[i] = nil
   end
   body()
   for i = #anchors, 1, -1 do
      SILE.typesetter:pushHorizontal(anchors[i])
   end
end

--- Set `body` inside a rule of the given thickness, a little padding between.
--
-- Drawn the way the `rules` package draws a rule — four of them, on the box's
-- own `outputYourself` — because the framebox package is not in this
-- runtime and a rule is all a border is.
local function bordered (thickness, body)
   local hbox = SILE.typesetter:makeHbox(body)
   local bw = SILE.types.measurement(thickness):tonumber()
   local pad = bw + 1.5
   local width = hbox.width:tonumber() + 2 * pad
   local height = hbox.height:tonumber() + pad
   local depth = hbox.depth:tonumber() + pad
   SILE.typesetter:pushHbox({
      width = SILE.types.length(width),
      height = SILE.types.length(height),
      depth = SILE.types.length(depth),
      value = hbox,
      outputYourself = function (node, typesetter, line)
         local x = typesetter.frame.state.cursorX
         local y = typesetter.frame.state.cursorY
         local top = y - height
         SILE.outputter:drawRule(x, top, width, bw)
         SILE.outputter:drawRule(x, y + depth - bw, width, bw)
         SILE.outputter:drawRule(x, top, bw, height + depth)
         SILE.outputter:drawRule(x + width - bw, top, bw, height + depth)
         typesetter.frame:advanceWritingDirection(pad)
         node.value:outputYourself(typesetter, line)
         typesetter.frame:advanceWritingDirection(pad)
      end,
   })
end

--- Process only element children, discarding whitespace laid out for humans.
local function elements (content)
   for _, item in ipairs(content) do
      if type(item) == "table" then
         SILE.process({ item })
      end
   end
end

local function skip (height)
   if height then
      SILE.call("skip", { height = height })
   end
end

-- ---------------------------------------------------------------------------
-- Tables (SCR-009).
--
-- A table is set in two passes, which is the fewest that can produce a column:
-- measure every cell, then set every cell to the width of the widest one in
-- its column. There is no way to know a column's width from the first cell in
-- it, so nothing can be typeset until the last row has been read.
--
-- Cells are measured as hboxes and therefore never wrap. That is a deliberate
-- limit rather than an oversight: a wrapping cell needs a nested typesetter
-- with its own measure, and the tables USFM carries are genealogies and
-- censuses -- a name and a number. What a too-wide table gets instead is a
-- warning on the backend's stderr, which reaches the build log, and a row that
-- visibly runs past the measure rather than one that silently looks fine.
-- ---------------------------------------------------------------------------

local TABLE = {
   -- Between columns. Enough to read as a division without becoming one.
   gutter = SILE.types.measurement("9pt"),
   -- Between rows.
   leading = "2pt",
   -- Above and below the whole table.
   around = "5pt",
}

--- The measured cells of one table, row by row.
---
--- Each cell keeps its natural hbox, so the second pass sets what the first
--- pass measured and no content is shaped twice.
local function measure_table (content)
   local rows = {}
   for _, node in ipairs(content) do
      if type(node) == "table" and node.command == "row" then
         local header = node.options and node.options.header == "true"
         local cells = {}
         for _, child in ipairs(node) do
            if type(child) == "table" and child.command == "cell" then
               local options = child.options or {}
               local hbox
               local weight = header and 600 or nil
               SILE.settings:temporarily(function ()
                  -- The cell's own typography, and the header's emphasis over
                  -- it. Measuring in a different font from the one it is set
                  -- in would produce columns that do not line up, so the
                  -- switch has to be inside the measurement.
                  styled("cell", function ()
                     if weight then
                        SILE.call("font", { weight = weight }, function ()
                           hbox = SILE.typesetter:makeHbox(child)
                        end)
                     else
                        hbox = SILE.typesetter:makeHbox(child)
                     end
                  end)
               end)
               cells[#cells + 1] = {
                  hbox = hbox,
                  align = options.align or "start",
                  span = math.max(1, math.floor(tonumber(options.span) or 1)),
               }
            end
         end
         rows[#rows + 1] = { header = header, cells = cells }
      end
   end
   return rows
end

--- The width of each column, in points.
---
--- Single-column cells set the widths. A cell that spans several is fitted
--- afterwards and only if it does not already fit: the columns it covers are
--- already as wide as their own contents need, and widening them for a
--- spanning cell that fits would push every row apart for nothing. The
--- shortfall goes to the last column it covers, which is the one choice that
--- leaves the columns to its left where the rest of the table put them.
local function column_widths (rows)
   local widths = {}
   local function at (col)
      return widths[col] or 0
   end

   for _, row in ipairs(rows) do
      local col = 1
      for _, cell in ipairs(row.cells) do
         if cell.span == 1 then
            local w = cell.hbox.width:tonumber()
            if w > at(col) then
               widths[col] = w
            end
         end
         col = col + cell.span
      end
   end

   local gutter = TABLE.gutter:tonumber()
   for _, row in ipairs(rows) do
      local col = 1
      for _, cell in ipairs(row.cells) do
         if cell.span > 1 then
            local have = (cell.span - 1) * gutter
            for i = col, col + cell.span - 1 do
               widths[i] = at(i)
               have = have + widths[i]
            end
            local want = cell.hbox.width:tonumber()
            if want > have then
               local last = col + cell.span - 1
               widths[last] = at(last) + (want - have)
            end
         end
         col = col + cell.span
      end
   end

   -- A column nothing landed in still has a width, so the arithmetic below
   -- does not have to care whether a row was short.
   local count = 0
   for _, row in ipairs(rows) do
      local n = 0
      for _, cell in ipairs(row.cells) do
         n = n + cell.span
      end
      count = math.max(count, n)
   end
   for i = 1, count do
      widths[i] = at(i)
   end
   return widths, count
end

--- Set one row, every cell padded out to its column.
---
--- Padding is a kern rather than glue on purpose. Glue between the cells would
--- be a legal line break, and a row broken in half is worse than a row that
--- runs wide; a kern makes the row a single unbreakable object, which is what
--- a row is.
local function set_row (row, widths)
   local gutter = TABLE.gutter
   local col = 1
   for index, cell in ipairs(row.cells) do
      local width = 0
      for i = col, col + cell.span - 1 do
         width = width + (widths[i] or 0)
      end
      width = width + (cell.span - 1) * gutter:tonumber()

      local slack = width - cell.hbox.width:tonumber()
      if slack < 0 then
         slack = 0
      end
      if cell.align == "end" then
         SILE.typesetter:pushHorizontal(SILE.types.node.kern(slack))
         SILE.typesetter:pushHbox(cell.hbox)
      else
         SILE.typesetter:pushHbox(cell.hbox)
         SILE.typesetter:pushHorizontal(SILE.types.node.kern(slack))
      end
      if index < #row.cells then
         SILE.typesetter:pushHorizontal(SILE.types.node.kern(gutter))
      end
      col = col + cell.span
   end
   SILE.typesetter:leaveHmode()
end

-- ---------------------------------------------------------------------------
-- Notes and cross-references (SCR-003 – SCR-005, USFM-002).
--
-- Two apparatus, kept apart at every level a reader can see: their own caller
-- sequences, their own styles, and — for references — their own placement.
-- What they share is the note area and the machinery below.
-- ---------------------------------------------------------------------------

--- The classic footnote symbols, in the classic order.
--
-- Doubled after the sixth rather than continuing into an eighth glyph nobody
-- recognises: `**` reads as "the seventh mark" and `⁂` does not.
local SYMBOLS = { "*", "†", "‡", "§", "‖", "¶" }

--- The nth mark of a sequence, counting from 1.
local function mark_for (kind, n)
   if kind == "none" then
      return ""
   elseif kind == "letters" then
      -- a…z, then aa, bb, cc. Repetition rather than base-26 counting,
      -- because `aa` after `z` is what a reader expects and `ba` is not.
      local cycle = math.floor((n - 1) / 26) + 1
      local letter = string.char(string.byte("a") + (n - 1) % 26)
      return string.rep(letter, cycle)
   elseif kind == "symbols" then
      local cycle = math.floor((n - 1) / #SYMBOLS) + 1
      return string.rep(SYMBOLS[(n - 1) % #SYMBOLS + 1], cycle)
   end
   return tostring(n)
end

--- The next mark for one apparatus, advancing its sequence.
--
-- `which` is the key in `scratch().issued`; `kind` the configured sequence.
local function next_mark (which, kind)
   local s = scratch()
   s.issued[which] = s.issued[which] + 1
   return mark_for(kind, s.issued[which])
end

--- What USFM's `caller` attribute asks for.
--
-- `+` is "give me the next mark", `-` is "no mark at all", and anything else is
-- the mark the editor chose, printed as written. An editor's own mark does not
-- take a place in the sequence: it was chosen precisely so that it would not be
-- one of them.
local function caller_for (caller, which, kind)
   if caller == "-" then
      return ""
   elseif caller == "+" or caller == nil or caller == "" then
      return next_mark(which, kind)
   end
   return caller
end

--- Both sequences start again, if the policy says they do at this boundary.
function restart_notes (at)
   if SILE.documentState.documentClass._bcopts.restartnotes == at then
      local s = scratch()
      s.issued.note = 0
      s.issued.ref = 0
   end
end

--- The mark, raised and reduced, where the note was called.
local function typeset_caller (mark, selector)
   if mark == "" then
      return
   end
   SILE.call("raise", { height = "0.7ex" }, function ()
      -- Off the note's own style rather than a fixed fraction, so a publisher
      -- who sets notes larger gets callers to match. Sized down again from
      -- there because a caller is a mark and not a word.
      local size = style(selector).font_size
      SILE.call("font", size and { size = size } or { size = "0.75em" }, function ()
         SILE.typesetter:typeset(mark)
      end)
   end)
end

--- Put one note into the note area.
--
-- Modelled on upstream's `\footnote`, which this class no longer loads, and
-- which is worth reading beside this: the frame swap, the settings push and the
-- two restores are all its work and all necessary. What differs is that the
-- caller and the origin reference are ours rather than a global counter and a
-- full stop.
local function insert_note (selector, mark, origin, body)
   local options = SILE.scratch.insertions.classes[NOTES]
   if not options then
      return
   end

   local frame = SILE.getFrame(options.insertInto.frame)
   local old_target = SILE.typesetter.getTargetLength
   local old_frame = SILE.typesetter.frame
   -- The note is measured, not fitted: how much of it lands on this page is the
   -- page builder's decision, made later, when it knows what else is there.
   SILE.typesetter.getTargetLength = function ()
      return SILE.types.length(0xFFFFFF)
   end
   SILE.settings:pushState()
   SILE.settings:toplevelState()
   SILE.typesetter:initFrame(frame)
   -- Hanging indentation belongs to the paragraph that called the note, not to
   -- the note. Upstream resets the same four, for the same reason.
   for _, key in ipairs({
      "current.hangAfter",
      "current.hangIndent",
      "linebreak.hangAfter",
      "linebreak.hangIndent",
   }) do
      SILE.settings:set(key, SILE.settings.defaults[key])
   end

   -- A note's body is a `<para>` like any other, so the `para` command runs
   -- inside it — and would flush the references waiting for the paragraph that
   -- called the note into the note itself. Found exactly that way: with
   -- `end_of_paragraph` placement the references appeared in the note area,
   -- under the footnote whose body happened to be next.
   local s = scratch()
   local outer_note = s.in_note
   s.in_note = true

   local material
   -- The font is applied outside the vbox, because a baselineskip expressed as
   -- a ratio has to be resolved against the size the note is actually set in.
   styled(selector, function ()
      material = SILE.call("vbox", {}, function ()
         SILE.call("noindent")
         if mark ~= "" then
            SILE.typesetter:typeset(mark)
            SILE.call("kern", { width = "0.4em" })
         end
         -- `\fr` and `\xo`: the reference the note is about. USFM separates it
         -- from the note's text precisely so that it can be set apart, and an
         -- edition that prints notes keyed by symbol relies on it to say which
         -- verse a note belongs to.
         if origin and origin ~= "" then
            styled("character." .. (selector == "reference" and "xo" or "fr"), function ()
               SILE.typesetter:typeset(origin)
            end)
            SILE.call("kern", { width = "0.4em" })
         end
         body()
      end)
   end)

   s.in_note = outer_note
   SILE.settings:popState()
   SILE.typesetter.getTargetLength = old_target
   SILE.typesetter.frame = old_frame
   SILE.documentState.documentClass:insert(NOTES, material)
end

--- Set the references a paragraph gathered, under that paragraph.
--
-- Only reached by `end_of_paragraph` placement. Called from `para` after its
-- content and before its space below, so the references sit with the paragraph
-- and a page break between them is as unlikely as SILE can make it.
local function flush_pending_refs ()
   local s = scratch()
   if s.in_note or #s.pending_refs == 0 then
      return
   end
   local waiting = s.pending_refs
   s.pending_refs = {}
   SILE.call("par")
   styled("reference", function ()
      for i, item in ipairs(waiting) do
         if i > 1 then
            SILE.call("kern", { width = "0.6em" })
         end
         if item.mark ~= "" then
            SILE.typesetter:typeset(item.mark)
            SILE.call("kern", { width = "0.3em" })
         end
         if item.origin and item.origin ~= "" then
            styled("character.xo", function ()
               SILE.typesetter:typeset(item.origin)
            end)
            SILE.call("kern", { width = "0.3em" })
         end
         SILE.process(item.content)
      end
   end)
   SILE.call("par")
end

function class:registerXmlCommands ()
   -- The root. The version attribute is the compatibility contract (SILE-009);
   -- refusing an unknown one turns a mismatched install into one clear line
   -- rather than a page of Lua stack traces.
   self:registerCommand("biblecompose", function (options, content)
      local want = "1"
      if options.version and options.version ~= want then
         SU.error(
            "this SILE class speaks BibleCompose contract version "
               .. want
               .. " but the document declares "
               .. tostring(options.version)
               .. " — the application and the class must be released together"
         )
      end
      local o = self._bcopts
      -- The body font comes from settings, not from `styles`. Everything else
      -- in `styles` is sized relative to it in spirit but not yet in code —
      -- P3.1 makes the rest overridable and can then express them as ratios.
      if o.fontfile ~= "" then
         SILE.call("font", { filename = o.fontfile, size = o.fontsize })
      else
         SILE.call("font", { family = o.fontfamily, size = o.fontsize })
      end
      SILE.settings:set("document.baselineskip", SILE.types.node.vglue(o.leading))

      -- What the file says about itself (PDF-005).
      --
      -- Only what was given. An empty value is written as no property rather
      -- than as an empty one, because a properties panel showing `Title:` with
      -- nothing after it looks like an answer and is not.
      --
      -- `Lang` is not one of these. It belongs in the document catalogue and
      -- these go in the info dictionary, so it is set below where the
      -- outputter can put it in the right place.
      for key, value in pl.tablex.sort({ Title = o.title, Author = o.author, Subject = o.subject }) do
         if value ~= "" then
            SILE.call("pdf:metadata", { key = key, value = value })
         end
      end

      -- Hyphenation is per-language in SILE, and the way to have none is a
      -- language with no patterns. "und" is that language, and saying so
      -- here keeps the whole of it in one place rather than reaching into
      -- the hyphenator.
      SILE.settings:set("document.language", o.hyphenate and o.language or "und")

      -- Last-resort stretch, so a paragraph that cannot be broken within
      -- tolerance is set loose rather than overfull.
      --
      -- Without it, Scripture in a script that does not hyphenate runs off
      -- the measure and off the paper: measured on one book of Tamil in two
      -- columns, 20.6% of lines ended outside the column and the worst was
      -- 113pt past it, on a page 432pt wide. English on the same page never
      -- overflowed by a point, which is what makes this a breakpoint problem
      -- rather than a frame problem — Latin has a break every five or six
      -- characters and Tamil does not.
      --
      -- A quarter of the measure rather than a fixed length, because the
      -- measure is what it has to rescue: the same absolute stretch is
      -- nothing across a single-column page and a disfigurement in a narrow
      -- column. It costs nothing where the breaker was already succeeding —
      -- TeX only spends emergency stretch on a paragraph that would
      -- otherwise fail, so Latin setting is unchanged, byte for byte.
      local ok, measure = pcall(function ()
         return SILE.getFrame("contentA"):width()
      end)
      if ok and measure then
         SILE.settings:set("linebreak.emergencyStretch", measure * 0.25)
      end

      elements(content)
   end)

   -- The resolved style map. Read before anything is set, because every
   -- element below looks into it.
   self:registerCommand("styles", function (_, content)
      local into = sheet()
      for _, item in ipairs(content) do
         if type(item) == "table" and item.command == "style" then
            local o = item.options or {}
            local key = o["for"]
            if key then
               into[key] = o
            end
         end
      end
   end)

   -- Never reached: `styles` consumes its children itself. Registered so that
   -- a `<style>` arriving anywhere else is inert rather than "unknown command".
   self:registerCommand("style", function (_, _) end)

   self:registerCommand("book", function (options, content)
      -- A new book starts a new page — but the break goes at the START of the
      -- second and later books, never at the end of one. `\supereject` after
      -- the *final* book fills the frames with infinite glue and, while
      -- `balanced-frames` was loaded, could never be balanced away; SILE spun
      -- rather than failing.
      --
      -- **`\supereject` and not `\eject`.** `eject` is a `reak`, which the
      -- typesetter reads as "move on to the next frame" — and in two columns
      -- the next frame is the next *column*. A second book therefore began in
      -- column B of the page the first one ended on: measured on a two-book
      -- document that came out one page long, with Genesis and John under a
      -- single running head. Only a penalty at or past `supereject_penalty`
      -- reaches `newPage`.
      local s = scratch()
      if s.books > 0 then
         -- A book opens where its chapters open, when they ask for a side;
         -- otherwise on the next page.
         local where = style("chapter").new_page or "continue"
         if where == "left" or where == "right" then
            open_page(where)
         else
            SILE.typesetter:leaveHmode()
            SILE.call("supereject")
         end
      end
      s.books = s.books + 1
      s.chapters_in_book = 0
      restart_notes("per_book")
      -- Nothing is carried across a book. A title page holding no verse should
      -- not be headed with the last verse of the book before it.
      s.carried = nil

      scratch().book_title = options.name or options.code or ""
      SILE.call("bc:book", {}, { options.name or options.code or "" })
      -- Keyed by the name the head shows, so a page can look up the alt form
      -- of *its own* book rather than of whichever was set last.
      local named = options.name or options.code or ""
      scratch().altbooks[named] = options.altname or named
      -- After `bc:book`, so the destination lands with the book's first
      -- content rather than before the page break that precedes it.
      s.code = options.code or ""
      s.chapter = ""
      destination(anchor(s.code), "chapter")
      if s.code ~= "" and o_anchors() ~= "none" then
         SILE.call("pdf:bookmark", { dest = anchor(s.code), title = named, level = 1 })
      end
      if options.name then
         SILE.call("center", {}, function ()
            SILE.call("font", { size = "16pt", weight = 600 }, function ()
               SILE.typesetter:typeset(options.name)
            end)
         end)
         SILE.call("par")
         skip("5pt")
      end
      elements(content)
   end)

   self:registerCommand("heading", function (options, content)
      local selector = "heading." .. (options.style or "s") .. (options.level or "1")
      local s = style(selector)
      SILE.call("goodbreak")
      skip(s.space_above)
      local align = alignment(s)
      styled(selector, function ()
         if align then
            SILE.call(align, {}, function ()
               SILE.process(content)
            end)
         else
            SILE.process(content)
         end
      end)
      -- A heading is not left standing at the foot of a column (SCR-011), and
      -- the order of these five calls is the whole of how that is arranged.
      --
      -- `\novbreak` comes *before* `\par`, which looks redundant and is not.
      -- It ends the paragraph itself, by way of `leaveHmode`, and then pushes
      -- its penalty — so the penalty is the node immediately after the heading
      -- box. `\par` then sees vertical mode with a penalty on the end of the
      -- queue and returns without doing anything, and that is the point:
      -- reaching it the other way round lets `\par` push `document.parskip`
      -- first, and glue directly after a box is a legal break, so the page
      -- would divide under the heading no matter how many penalties followed.
      --
      -- The penalty after the skip matters for the same reason, one node
      -- further on. `vertical` is what makes both of them land in the
      -- page-breaking list rather than the line-breaking one.
      SILE.call("novbreak")
      SILE.call("par")
      SILE.call("novbreak")
      skip(s.space_below)
      SILE.call("novbreak")
   end)

   self:registerCommand("para", function (options, content)
      local selector = "paragraph." .. (options.style or "p")
      local s = style(selector)
      skip(s.space_above)
      SILE.settings:temporarily(function ()
         if s.indent then
            SILE.settings:set("document.lskip", SILE.types.node.glue(s.indent))
         end
         local align = alignment(s)
         styled(selector, function ()
            if align then
               SILE.call(align, {}, function ()
                  SILE.process(content)
               end)
            else
               SILE.process(content)
               SILE.call("par")
            end
         end)
      end)
      -- Before the space below, so a paragraph and the references it gathered
      -- stay one block rather than two separated by it.
      flush_pending_refs()
      skip(s.space_below)
   end)

   self:registerCommand("poetry", function (options, content)
      local selector = "poetry." .. (options.style or "q") .. (options.level or "1")
      local s = style(selector)
      skip(s.space_above or "1pt")
      local align = alignment(s)
      -- An indent on a poetry line is a first-line indent of the whole line,
      -- so it is glue at the start rather than a left skip: a line that wraps
      -- should hang, which is what a reader expects of verse.
      if s.indent and not align and self._bcopts.poetryindent then
         SILE.call("glue", { width = s.indent })
      end
      styled(selector, function ()
         if align then
            SILE.call(align, {}, function ()
               SILE.process(content)
            end)
         else
            SILE.process(content)
            SILE.call("par")
         end
      end)
      skip(s.space_below)
   end)

   -- A list item is indented as a block, not as a first line. `\li2` inside a
   -- narrow column wraps, and a wrapped item whose second line returns to the
   -- margin has lost the only thing its indent was saying. So this is `lskip`
   -- and not the leading glue poetry uses, where hanging is the point.
   self:registerCommand("item", function (options, content)
      local selector = "list." .. (options.style or "li") .. (options.level or "1")
      local s = style(selector)
      skip(s.space_above)
      SILE.settings:temporarily(function ()
         if s.indent then
            SILE.settings:set("document.lskip", SILE.types.node.glue(s.indent))
         end
         -- No first-line indent on top of the left skip: an item's first line
         -- starts where the rest of it does.
         SILE.settings:set("current.parindent", SILE.types.node.glue())
         SILE.settings:set("document.parindent", SILE.types.node.glue())
         styled(selector, function ()
            SILE.process(content)
            SILE.call("par")
         end)
      end)
      flush_pending_refs()
      skip(s.space_below)
   end)

   self:registerCommand("table", function (_, content)
      skip(TABLE.around)
      SILE.settings:temporarily(function ()
         SILE.settings:set("document.lskip", SILE.types.node.glue())
         SILE.settings:set("document.rskip", SILE.types.node.glue())
         SILE.settings:set("document.parindent", SILE.types.node.glue())
         SILE.settings:set("current.parindent", SILE.types.node.glue())
         -- Nothing in a row stretches, so justification has nothing to do and
         -- would only report the row as underfull on every line.
         SILE.settings:set("document.spaceskip", nil)

         local rows = measure_table(content)
         local widths, columns = column_widths(rows)

         local total = (columns - 1) * TABLE.gutter:tonumber()
         for i = 1, columns do
            total = total + widths[i]
         end
         local measure = SILE.typesetter.frame:width():tonumber()
         if total > measure then
            SU.warn(
               ("table is %.1fpt wide in a %.1fpt column and will run past it; "):format(
                  total,
                  measure
               ) .. "shorten a cell or set the book in one column"
            )
         end

         for index, row in ipairs(rows) do
            set_row(row, widths)
            if index < #rows then
               skip(TABLE.leading)
            end
         end
      end)
      skip(TABLE.around)
   end)

   -- `row` and `cell` are read by `measure_table` rather than processed, so
   -- they are registered only to keep SILE from reporting them unknown if one
   -- ever reaches the processor on its own. A cell outside a table has no
   -- column to belong to, and setting its text is the most honest thing left.
   self:registerCommand("row", function (_, content)
      elements(content)
      SILE.call("par")
   end)

   self:registerCommand("cell", function (_, content)
      SILE.process(content)
   end)

   self:registerCommand("break", function (_, _)
      skip("4pt")
   end)

   self:registerCommand("chapter", function (options, _)
      -- `n` arrives as a string and stays one all the way into chapterverse.
      -- Spike F-9: anything SILE later stringifies must already be a string,
      -- or the running head renders "table: 0x55f…".
      -- `n` is passed as an option as well as as content: the content is what
      -- gets set, and `bc:chapter` needs the number itself to name the
      -- chapter's anchor. Reading it back out of the content would work and
      -- would be the only place in this file that parsed its own output.
      SILE.call("bc:chapter", { n = tostring(options.n or "") }, { tostring(options.n or "") })
   end)

   -- The chapter number, four ways: in the text with the first line running
   -- into it (the default), on a line of its own, dropped into the lines
   -- that follow, or any of those inside a rule. The `chapter` style
   -- decides; see `docs/GUIDE.md` for what each key does.
   self:registerCommand("bc:chapter-number", function (_, content)
      local s = style("chapter")
      local boxed = SU.boolean(s.border, false)
      local function number ()
         if boxed then
            bordered(s.border_width or "0.5pt", function ()
               styled("chapter", function ()
                  SILE.process(content)
               end)
            end)
         else
            styled("chapter", function ()
               SILE.process(content)
            end)
         end
      end

      local drops = self._bcopts.dropcaps
      local number_drops = drops and self._bcopts.dropcapof == "chapter_number"
      if SU.boolean(s.own_line, false) or (drops and not number_drops) then
         -- On a line of its own, with the style's space around it and its
         -- alignment along it. Also when the initial drops: the number used
         -- to be the large thing at the chapter's corner, and two large
         -- things at one corner would fight. Through `dropped`, because the
         -- chapter's anchor has already opened the paragraph — with the
         -- paragraph indent — and the number's line must not carry that.
         dropped(function ()
            skip(s.space_above)
            SILE.call("noindent")
            SILE.call(alignment(s) or ALIGNMENT.start, {}, function ()
               number()
               SILE.call("par")
            end)
            skip(s.space_below)
            -- The text that follows opens a chapter, not a paragraph in the
            -- middle of one: flush, as the first paragraph under a heading.
            SILE.call("noindent")
         end)
         return
      end

      if number_drops then
         -- Dropped into the text as an initial would be, and for the same
         -- reason reopening the paragraph. The style's face goes to the
         -- package as font options and its size does not: the package sets
         -- the size that spans the lines, and a `\font` inside with the
         -- style's own size would undo that.
         dropped(function ()
            local opts = { lines = tonumber(self._bcopts.dropcaplines) or 3, join = false }
            for key, value in pairs(face(s) or {}) do
               if key ~= "size" then
                  opts[key] = value
               end
            end
            if s.color then
               opts.color = s.color
            end
            SILE.call("dropcap", opts, content)
         end)
         return
      end

      SILE.call("noindent")

      if s.gap_before then
         SILE.call("kern", { width = s.gap_before })
      end
      number()
      SILE.call("kern", { width = s.gap_after or "4pt" })
   end)

   -- The chapter's opening initial: its first syllable, marked by the
   -- emitter. Dropped when the option says so; plain text otherwise, and in
   -- both cases followed by the rest of the word with nothing between, since
   -- the document put nothing between.
   self:registerCommand("initial", function (_, content)
      -- Only when the first letter is the dropped thing. When the number
      -- is, the letter stays: two initials at one corner would be the fight
      -- the number moved off the line to avoid.
      if not self._bcopts.dropcaps or self._bcopts.dropcapof == "chapter_number" then
         SILE.process(content)
         return
      end
      -- **The initial has to be the first thing in its paragraph, and it is
      -- not.** The verse anchor comes before it — a PDF destination and the
      -- running head's note of where the verse is — and pushing those opens
      -- the paragraph: SILE's `newPar` runs on the first node, pushes the
      -- paragraph indent, and copies `current.hangIndent` into what the
      -- line-breaker reads. By the time `\dropcap` sets it, nobody copies it
      -- again, and the result is the initial hanging in the margin with the
      -- text at the margin beside it. Upstream never sees this because
      -- nothing precedes `\dropcap` there.
      --
      -- So the paragraph is put back to unopened: its nodes are taken off,
      -- the package opens it — with the hang set and no indent — and the
      -- anchors go back in after the initial, a few points to the right of
      -- where they were. The first two nodes are `newPar`'s own, the zero
      -- box and the indent glue, and are not kept.
      -- `join` sets the first line hard against the initial, which is right
      -- for a letter that is the start of its word; the standoff applies to
      -- the lines below it.
      local lines = tonumber(self._bcopts.dropcaplines) or 3
      dropped(function ()
         SILE.call("dropcap", { lines = lines, join = true }, content)
      end)
      -- For the padding above: this paragraph has to run to that many lines.
      scratch().initial_lines = lines
   end)

   self:registerCommand("verse", function (options, _)
      SILE.call("bc:verse", { start = options.start }, { tostring(options.n or "") })
   end)

   self:registerCommand("bc:verse-number", function (_, content)
      local s = style("verse")
      SILE.call("raise", { height = s.raise or "0pt" }, function ()
         styled("verse", function ()
            SILE.process(content)
         end)
      end)
      SILE.call("kern", { width = "0.13em" })
   end)

   self:registerCommand("char", function (options, content)
      -- An unstyled character marker still processes its content. \wj is the
      -- example that matters: red-letter text is a style a publisher may set
      -- and may equally leave alone, and either way the words are Scripture.
      styled("character." .. (options.style or ""), function ()
         SILE.process(content)
      end)
   end)

   -- Hidden rather than omitted from the document: a publisher who turns
   -- footnotes back on gets them without a re-emission, and the application's
   -- warning about an unsupported marker inside a note stays true either way.
   --
   -- A hidden note does not take a mark either. Leaving the sequence to run
   -- would number the notes that are printed 3, 7, 11 — which is not what
   -- "hidden" means anywhere else in this class.
   self:registerCommand("note", function (options, content)
      if not self._bcopts.footnotes then
         return
      end
      local selector = "note." .. (options.style or "f")
      local mark = caller_for(options.caller, "note", self._bcopts.footnotecallers)
      typeset_caller(mark, selector)
      insert_note(selector, mark, options.origin, function ()
         elements(content)
      end)
   end)

   -- A cross-reference is not a footnote (SCR-004), and the three placements
   -- are the whole of the difference a publisher can see. Its own sequence in
   -- every one of them, so a page carrying both apparatus reads `1` for a note
   -- and `a` for a reference rather than interleaving one count between them.
   self:registerCommand("xref", function (options, content)
      if not self._bcopts.crossrefs then
         return
      end
      local placement = self._bcopts.crossrefplacement
      -- A reference set in the text *is* its own mark: there is nothing for a
      -- caller to point at, and one would be a mark beside the thing it marks.
      local kind = placement == "inline" and "none" or self._bcopts.crossrefcallers
      local mark = caller_for(options.caller, "ref", kind)

      if placement == "inline" then
         styled("reference", function ()
            SILE.typesetter:typeset("[")
            SILE.process(content)
            SILE.typesetter:typeset("]")
         end)
         return
      end

      if placement == "end_of_paragraph" then
         typeset_caller(mark, "reference")
         table.insert(scratch().pending_refs, {
            mark = mark,
            origin = options.origin,
            content = content,
         })
         return
      end

      typeset_caller(mark, "reference")
      insert_note("reference", mark, options.origin, function ()
         SILE.process(content)
      end)
   end)

   self:registerCommand("figure", function (options, content)
      SILE.call("par")
      skip("4pt")
      -- S0.7 established that PNG, JPG and vector PDF all place, and that an
      -- included PDF brings its whole page box with it.
      --
      -- **One width, and no branch on `size`.** USFM's two values are `col`
      -- and `span`, and `100%fw` is the frame the text is flowing through — so
      -- in a single column it is the measure, which is what `span` asks for,
      -- and in two it is the column, which is what `col` asks for. There is no
      -- mid-flow way to reach across a gutter, so a two-column `span` cannot
      -- be honoured; the application says so once, in pre-flight, rather than
      -- this file quietly setting it narrow. The branch that used to be here
      -- had the same value on both sides.
      --
      -- **And no `pcall`.** It used to wrap this, and a project naming two
      -- figures that did not exist built to "completed" with two holes in the
      -- PDF. Every reason a figure will not draw — absent, outside the
      -- project, not an image — is now a diagnostic before the backend is
      -- started, so anything that fails here is a surprise and should behave
      -- like one.
      SILE.call("img", { src = options.src, width = "100%fw" })
      if content and #content > 0 then
         SILE.call("par")
         styled("caption", function ()
            SILE.process(content)
         end)
      end
      SILE.call("par")
      skip("4pt")
   end)

   -- Milestones are not tree-shaped and carry no appearance of their own; they
   -- are preserved in the contract so a later release can act on them.
   self:registerCommand("milestone", function (_, _) end)

   -- Rendered inert rather than dropped. The application has already warned
   -- about it (FUN-003); showing nothing here would make the warning look
   -- wrong to anyone comparing the log against the page.
   self:registerCommand("unsupported", function (_, _) end)
end

return class
