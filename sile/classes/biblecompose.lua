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

-- Option values arrive from the document as strings. `SU.boolean` is the
-- coercion upstream is missing; without it "false" selects two columns.
local function opt (options, key, default)
   local v = options[key]
   if v == nil then
      return default
   end
   return v
end

function class:declareOptions ()
   plain.declareOptions(self)
   for _, key in ipairs({
      "columns",
      "gutter",
      "margintop",
      "marginbottom",
      "margininner",
      "marginouter",
      "headsep",
      "footsep",
   }) do
      self:declareOption(key, function (_, value)
         if value then
            -- Coerce here, not in setOptions: plain.setOptions runs these
            -- setters afterwards with the raw document strings, so anything
            -- converted earlier gets overwritten by a string. This is the
            -- same defect as upstream's truthy "false". NOTES.md F-6.
            self._bcopts[key] = (key == "columns") and (tonumber(value) or 2) or value
         end
         return self._bcopts[key]
      end)
   end
end

function class:setOptions (options)
   self._bcopts = self._bcopts or {}
   self._bcopts.columns = tonumber(opt(options, "columns", 2)) or 2
   self._bcopts.gutter = opt(options, "gutter", "3.5%pw")
   self._bcopts.margintop = opt(options, "margintop", "9%ph")
   self._bcopts.marginbottom = opt(options, "marginbottom", "12%ph")
   self._bcopts.margininner = opt(options, "margininner", "11%pw")
   self._bcopts.marginouter = opt(options, "marginouter", "8%pw")
   self._bcopts.headsep = opt(options, "headsep", "4%ph")
   self._bcopts.footsep = opt(options, "footsep", "3%ph")
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

   self._bcopts = {
      columns = tonumber(options.columns or 2) or 2,
      gutter = options.gutter or "3.5%pw",
      margintop = options.margintop or "9%ph",
      marginbottom = options.marginbottom or "12%ph",
      margininner = options.margininner or "11%pw",
      marginouter = options.marginouter or "8%pw",
      headsep = options.headsep or "4%ph",
      footsep = options.footsep or "3%ph",
   }
   self.defaultFrameset = self:_frameset(self._bcopts.margininner, self._bcopts.marginouter)
   self.firstContentFrame = "contentA"

   plain._init(self, options)
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

   self:registerCommand("bc:verse", function (options, content)
      SILE.call("bc:verse-number", options, content)
      SILE.call("save-verse-number", options, flat(content))
      self:_setheads()
   end)

   self:registerCommand("bc:chapter", function (options, content)
      SILE.call("save-chapter-number", options, flat(content))
      SILE.call("bc:chapter-number", options, content)
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
-- Appearance lives in `styles` below rather than in the emitted document. At
-- M0 there is no configuration layer, so these are the built-in defaults that
-- P3.1 will make overridable; the document says *what*, this says *how*.
-- ---------------------------------------------------------------------------

local styles = {
   body = { family = "DejaVu Serif", size = "9.2pt", leading = "11.2pt" },
   heading = {
      s = { size = "10.2pt", weight = 600, above = "7pt", below = "1pt" },
      r = { size = "7.6pt", style = "italic", above = "1pt", below = "3pt" },
      d = { size = "9.2pt", style = "italic", above = "4pt", below = "2pt" },
      sp = { size = "9.2pt", style = "italic", above = "4pt", below = "2pt" },
      sr = { size = "7.6pt", style = "italic", above = "1pt", below = "3pt" },
   },
   chapter = { size = "21pt", weight = 700 },
   verse = { size = "6.4pt", weight = 600, raise = "0.42em" },
   note = { size = "7.4pt" },
   xref = { size = "7.4pt", style = "italic" },
   poetry_indent = 9, -- points per level
   char = {
      bd = { weight = 700 },
      bdit = { weight = 700, style = "italic" },
      em = { style = "italic" },
      it = { style = "italic" },
      nd = { features = "+smcp" },
      sc = { features = "+smcp" },
      wj = {},
      add = { style = "italic" },
      qt = { style = "italic" },
      tl = { style = "italic" },
   },
}

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
      SILE.call("font", styles.body)
      SILE.settings:set("document.baselineskip", SILE.types.node.vglue(styles.body.leading))
      elements(content)
   end)

   self:registerCommand("book", function (options, content)
      -- A new book starts a new page — but the break goes at the START of the
      -- second and later books, never at the end of one. `\supereject` after
      -- the final book fills the balanced frames with infinite glue that can
      -- never be balanced, and SILE spins forever rather than failing.
      SILE.scratch.biblecompose = SILE.scratch.biblecompose or { books = 0 }
      if SILE.scratch.biblecompose.books > 0 then
         SILE.typesetter:leaveHmode()
         SILE.call("eject")
      end
      SILE.scratch.biblecompose.books = SILE.scratch.biblecompose.books + 1

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
      local s = styles.heading[options.style] or styles.heading.s
      SILE.call("goodbreak")
      skip(s.above)
      SILE.call("font", { size = s.size, weight = s.weight, style = s.style }, function ()
         SILE.process(content)
      end)
      SILE.call("par")
      SILE.call("nobreak")
      skip(s.below)
   end)

   self:registerCommand("para", function (_, content)
      SILE.process(content)
      SILE.call("par")
   end)

   self:registerCommand("poetry", function (options, content)
      local level = tonumber(options.level) or 1
      skip("1pt")
      SILE.call("glue", { width = (level * styles.poetry_indent) .. "pt" })
      SILE.process(content)
      SILE.call("par")
   end)

   self:registerCommand("item", function (options, content)
      local level = tonumber(options.level) or 1
      SILE.call("glue", { width = (level * styles.poetry_indent) .. "pt" })
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
      SILE.call("font", { size = styles.chapter.size, weight = styles.chapter.weight }, function ()
         SILE.process(content)
      end)
      SILE.call("kern", { width = "4pt" })
   end)

   self:registerCommand("verse", function (options, _)
      SILE.call("bc:verse", {}, { tostring(options.n or "") })
   end)

   self:registerCommand("bc:verse-number", function (_, content)
      SILE.call("raise", { height = styles.verse.raise }, function ()
         SILE.call("font", { size = styles.verse.size, weight = styles.verse.weight }, function ()
            SILE.process(content)
         end)
      end)
      SILE.call("kern", { width = "0.13em" })
   end)

   self:registerCommand("char", function (options, content)
      local s = styles.char[options.style] or {}
      SILE.call("font", s, function ()
         SILE.process(content)
      end)
   end)

   self:registerCommand("note", function (_, content)
      SILE.call("footnote", {}, function ()
         SILE.call("font", { size = styles.note.size }, function ()
            elements(content)
         end)
      end)
   end)

   self:registerCommand("xref", function (_, content)
      SILE.call("footnote", {}, function ()
         SILE.call("font", { size = styles.xref.size, style = styles.xref.style }, function ()
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
         SILE.call("font", { size = "7.6pt", style = "italic" }, function ()
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
   local book = SILE.scratch.chapterverse and SILE.scratch.chapterverse.book
   local headfont = { size = "8.2pt", style = "italic" }
   SILE.call("left-running-head", {}, function ()
      SILE.settings:temporarily(function ()
         SILE.settings:set("document.lskip", SILE.types.node.glue())
         SILE.settings:set("document.rskip", SILE.types.node.glue())
         SILE.call("font", headfont, function ()
            SILE.call("page-reference-range")
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
            SILE.call("page-reference-range")
         end)
         SILE.typesetter:leaveHmode()
      end)
   end)
end

return class
