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
   { key = "hyphenate", kind = "boolean", default = true },
   -- What appears on the page. Each of these hides something the document
   -- still carries: the model is unchanged and the XML is unchanged, because
   -- ADR-002 says the document says what and the class says how. Re-running
   -- with verse numbers back on needs no re-emission.
   { key = "chapternumbers", kind = "boolean", default = true },
   { key = "versenumbers", kind = "boolean", default = true },
   -- Verse 1 goes unnumbered where the chapter number already marks it.
   { key = "hidefirstverse", kind = "boolean", default = false },
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
   { key = "headerleft", kind = "string", default = "book_name" },
   { key = "headercenter", kind = "string", default = "empty" },
   { key = "headerright", kind = "string", default = "reference_range" },
   { key = "footerleft", kind = "string", default = "empty" },
   { key = "footercenter", kind = "string", default = "page_number" },
   { key = "footerright", kind = "string", default = "empty" },
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
local slot_content, set_line, restart_notes

function class:endPage ()
   local o = self._bcopts
   set_line(SILE.getFrame("runningHead"), { o.headerleft, o.headercenter, o.headerright })
   set_line(SILE.getFrame("folio"), { o.footerleft, o.footercenter, o.footerright })
   return plain.endPage(self)
end

function class:registerCommands ()
   plain.registerCommands(self)

   -- The range of references actually present on this page. `first-reference`
   -- and `last-reference` read infonode's per-page collection, so this is the
   -- mechanism SRS "running head with reference range" needs.
   self:registerCommand("page-reference-range", function (_, _)
      SILE.call("first-reference", { showbook = false })
      SILE.typesetter:typeset("–")
      SILE.call("last-reference", { showbook = false })
   end)

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
      local first = self._bcopts.hidefirstverse and tostring(options.start) == "1"
      if self._bcopts.versenumbers and not first then
         SILE.call("bc:verse-number", options, content)
      end
      SILE.call("save-verse-number", options, flat(content))
   end)

   self:registerCommand("bc:chapter", function (options, content)
      SILE.call("save-chapter-number", options, flat(content))
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
local function scratch ()
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
   return s
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
--- What one slot puts on the page.
--
-- Nothing at all for `empty`, and nothing for a slot whose content this page
-- does not have — a page with no verse on it has no reference range, and a
-- head reading "–" would be worse than a head reading nothing.
function slot_content (slot)
   local s = scratch()
   if slot == "page_number" then
      -- Through the counters package rather than the raw value, so a project
      -- numbering its front matter in roman gets roman here too.
      local counters = SILE.documentState.documentClass.packages.counters
      SILE.typesetter:typeset(counters:formatCounter(SILE.scratch.counters.folio))
   elseif slot == "reference_range" then
      SILE.call("page-reference-range")
   elseif slot == "first_reference" then
      SILE.call("first-reference", { showbook = false })
   elseif slot == "last_reference" then
      SILE.call("last-reference", { showbook = false })
   elseif slot == "book_name" then
      local book = SILE.scratch.chapterverse and SILE.scratch.chapterverse.book
      if book then
         SILE.typesetter:typeset(tostring(book))
      end
   elseif slot == "alt_book_name" then
      if s.altbook and s.altbook ~= "" then
         SILE.typesetter:typeset(s.altbook)
      end
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
      if slot ~= "empty" then
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
      -- the final book fills the balanced frames with infinite glue that can
      -- never be balanced, and SILE spins forever rather than failing.
      local s = scratch()
      if s.books > 0 then
         SILE.typesetter:leaveHmode()
         SILE.call("eject")
      end
      s.books = s.books + 1
      restart_notes("per_book")

      SILE.call("bc:book", {}, { options.name or options.code or "" })
      scratch().altbook = options.altname or options.name or options.code or ""
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
      SILE.call("par")
      SILE.call("nobreak")
      skip(s.space_below)
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

   self:registerCommand("item", function (options, content)
      local level = tonumber(options.level) or 1
      local s = style("list." .. level)
      if s.indent then
         SILE.call("glue", { width = s.indent })
      end
      SILE.process(content)
      SILE.call("par")
   end)

   -- Tables are laid out as simple tab-separated rows at M0. Real column
   -- measurement is P4.7; emitting the structure now means the contract does
   -- not change when the layout improves.
   self:registerCommand("table", function (_, content)
      skip("3pt")
      elements(content)
      skip("3pt")
   end)

   self:registerCommand("row", function (options, content)
      if options.header == "true" then
         SILE.call("font", { weight = 700 }, function ()
            elements(content)
         end)
      else
         elements(content)
      end
      SILE.call("par")
   end)

   self:registerCommand("cell", function (options, content)
      SILE.process(content)
      if options.align == "end" then
         SILE.call("hfill")
      else
         SILE.call("qquad")
      end
   end)

   self:registerCommand("break", function (_, _)
      skip("4pt")
   end)

   self:registerCommand("chapter", function (options, _)
      -- `n` arrives as a string and stays one all the way into chapterverse.
      -- Spike F-9: anything SILE later stringifies must already be a string,
      -- or the running head renders "table: 0x55f…".
      SILE.call("bc:chapter", {}, { tostring(options.n or "") })
   end)

   self:registerCommand("bc:chapter-number", function (_, content)
      SILE.call("noindent")
      styled("chapter", function ()
         SILE.process(content)
      end)
      SILE.call("kern", { width = "4pt" })
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
      -- included PDF brings its whole page box. Width is bounded to the
      -- measure so a large asset cannot overflow the column.
      local width = options.size == "span" and "100%fw" or "100%fw"
      pcall(function ()
         SILE.call("img", { src = options.src, width = width })
      end)
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
