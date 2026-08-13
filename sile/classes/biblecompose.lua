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
-- page-scoped reference collection, `footnotes` for note frames, and
-- `balanced-frames` for column balancing. What it does not keep is any of the
-- hardcoding — geometry, the English word "Chapter", and the Gentium font are
-- all options here, because SRS CFG-002 requires them to come from settings.
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
   { key = "footnotes", kind = "boolean", default = true },
   { key = "crossrefs", kind = "boolean", default = true },
   { key = "runningheads", kind = "boolean", default = true },
   { key = "headbook", kind = "boolean", default = true },
   { key = "headref", kind = "boolean", default = true },
   { key = "folio", kind = "boolean", default = true },
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

-- Frames. `inner`/`outer` are resolved per master so the geometry mirrors
-- across the spread, which is what makes a bound book read correctly.
function class:_frameset (inner, outer)
   local o = self._bcopts
   local twocol = o.columns >= 2
   local frames = {
      runningHead = {
         left = "left(contentA)",
         right = twocol and "right(contentB)" or "right(contentA)",
         top = "top(contentA) - " .. o.headsep,
         bottom = "top(contentA)",
      },
      -- The folio sits *below* the text block, inside the bottom margin.
      -- Deriving its bottom from marginbottom (as upstream's arithmetic
      -- invites) puts bottom above top and overfills the frame by its own
      -- height on every page. NOTES.md F-7.
      folio = {
         left = "left(contentA)",
         right = twocol and "right(contentB)" or "right(contentA)",
         top = "bottom(footnotes) + " .. o.footsep,
         bottom = "bottom(footnotes) + " .. o.footsep .. " + 4%ph",
      },
      footnotes = {
         left = "left(contentA)",
         right = twocol and "right(contentB)" or "right(contentA)",
         height = "0",
         bottom = "100%ph - " .. o.marginbottom .. " - " .. o.footsep,
      },
   }

   if twocol then
      frames.contentA = {
         left = inner,
         right = "left(gutter)",
         top = o.margintop,
         bottom = "top(footnotes)",
         next = "contentB",
         balanced = true,
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
         balanced = true,
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
   end)

   -- These two load here, not in postinit: page 1 is laid out before postinit
   -- runs, so a footnote frame or a balancer registered there never applies to
   -- it. NOTES.md F-8.
   --
   -- One footnote frame spanning the full measure, stealing from every content
   -- frame. Upstream inserts into footnotesB only, so a note called in the
   -- left column lands under the right one.
   local steal = self._bcopts.columns >= 2 and { "contentA", "contentB" } or { "contentA" }
   self:loadPackage("footnotes", { insertInto = "footnotes", stealFrom = steal })

   if self._bcopts.columns >= 2 then
      self:loadPackage("balanced-frames")
      SILE.settings:set("linebreak.tolerance", 9000)
   end

   -- `folio` is loaded by plain, and the package's own way of being silent is
   -- this counter flag rather than an option — so turning page numbers off is
   -- setting it, not skipping the frame. The frame stays: the text block's
   -- bottom is derived from it, and removing it would move type on the page
   -- as a side effect of hiding a number.
   if not self._bcopts.folio then
      SILE.scratch.counters.folio.off = true
   end
end

-- Upstream writes this table but never creates it.
local function headers ()
   if not SILE.scratch.headers then
      SILE.scratch.headers = {}
   end
   return SILE.scratch.headers
end

function class:endPage ()
   local h = headers()
   local content = self:oddPage() and h.right or h.left
   if content then
      SILE.typesetNaturally(SILE.getFrame("runningHead"), function ()
         SILE.settings:set("current.parindent", SILE.types.node.glue())
         SILE.settings:set("document.lskip", SILE.types.node.glue())
         SILE.settings:set("document.rskip", SILE.types.node.glue())
         SILE.process(content)
         SILE.call("par")
      end)
   end
   return plain.endPage(self)
end

function class:registerCommands ()
   plain.registerCommands(self)

   self:registerCommand("left-running-head", function (_, content)
      local closure = SILE.settings:wrap()
      headers().left = function ()
         closure(content)
      end
   end)

   self:registerCommand("right-running-head", function (_, content)
      local closure = SILE.settings:wrap()
      headers().right = function ()
         closure(content)
      end
   end)

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
      if self._bcopts.versenumbers then
         SILE.call("bc:verse-number", options, content)
      end
      SILE.call("save-verse-number", options, flat(content))
      self:_setheads()
   end)

   self:registerCommand("bc:chapter", function (options, content)
      SILE.call("save-chapter-number", options, flat(content))
      if self._bcopts.chapternumbers then
         SILE.call("bc:chapter-number", options, content)
      end
      self:_setheads()
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

      SILE.call("bc:book", {}, { options.name or options.code or "" })
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
      if s.indent and not align then
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
      SILE.call("bc:verse", {}, { tostring(options.n or "") })
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
   self:registerCommand("note", function (_, content)
      if not self._bcopts.footnotes then
         return
      end
      SILE.call("footnote", {}, function ()
         styled("note.f", function ()
            elements(content)
         end)
      end)
   end)

   self:registerCommand("xref", function (_, content)
      if not self._bcopts.crossrefs then
         return
      end
      SILE.call("footnote", {}, function ()
         styled("reference", function ()
            SILE.process(content)
         end)
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

function class:_setheads ()
   local o = self._bcopts
   -- Nothing is registered at all when running heads are off, so `endPage`
   -- has no content to typeset and the frame is simply never filled.
   if not o.runningheads then
      return
   end

   local book = o.headbook and SILE.scratch.chapterverse and SILE.scratch.chapterverse.book
   local headfont = face(style("head")) or {}
   SILE.call("left-running-head", {}, function ()
      SILE.settings:temporarily(function ()
         SILE.settings:set("document.lskip", SILE.types.node.glue())
         SILE.settings:set("document.rskip", SILE.types.node.glue())
         SILE.call("font", headfont, function ()
            if o.headref then
               SILE.call("page-reference-range")
            end
            SILE.call("hfill")
            if book then
               SILE.typesetter:typeset(tostring(book))
            end
         end)
         SILE.typesetter:leaveHmode()
      end)
   end)
   SILE.call("right-running-head", {}, function ()
      SILE.settings:temporarily(function ()
         SILE.settings:set("document.lskip", SILE.types.node.glue())
         SILE.settings:set("document.rskip", SILE.types.node.glue())
         SILE.settings:set("typesetter.parfillskip", SILE.types.node.glue())
         SILE.call("font", headfont, function ()
            if book then
               SILE.typesetter:typeset(tostring(book))
            end
            SILE.call("hfill")
            if o.headref then
               SILE.call("page-reference-range")
            end
         end)
         SILE.typesetter:leaveHmode()
      end)
   end)
end

return class
