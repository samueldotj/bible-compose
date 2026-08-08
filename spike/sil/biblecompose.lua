--- biblecompose document class — S0 spike seed.
--
-- Written because SILE's bundled `bible` class cannot do the job: its
-- two-column path is unreachable (any value of `twocolumns`, including
-- "false", is truthy in Lua), and that path never loads `twoside`, so
-- `endPage` calls a nil `oddPage`. See ../NOTES.md F-5 and F-6.
--
-- What this keeps from upstream is the architecture, which is sound: masters
-- for mirrored page geometry, `twoside`, `infonode` + `chapterverse` for
-- page-scoped reference collection, `footnotes` for note frames, and
-- `balanced-frames` for column balancing. What it does not keep is any of the
-- hardcoding — geometry, the English word "Chapter", and the Gentium font are
-- all options here, because SRS CFG-002 requires them to come from settings.

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
   self._bcopts = {
      columns = tonumber(options and options.columns or 2) or 2,
      gutter = options and options.gutter or "3.5%pw",
      margintop = options and options.margintop or "9%ph",
      marginbottom = options and options.marginbottom or "12%ph",
      margininner = options and options.margininner or "11%pw",
      marginouter = options and options.marginouter or "8%pw",
      headsep = options and options.headsep or "4%ph",
      footsep = options and options.footsep or "3%ph",
   }
   self.defaultFrameset = self:_frameset(self._bcopts.margininner, self._bcopts.marginouter)
   self.firstContentFrame = "contentA"

   plain._init(self, options)
   self:loadPackage("masters")
   self:loadPackage("infonode")
   self:loadPackage("chapterverse")

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
      return { SU.contentToString(content) }
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
