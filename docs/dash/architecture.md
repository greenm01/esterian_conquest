# nc-dash — Full-Screen Dashboard TUI

## Overview

`nc-dash` is a modern full-screen terminal dashboard for Nostrian Conquest.
It replaces the legacy 80×25 BBS-style interface with a three-column layout
built for SSH and local play on modern terminals.

The legacy TUI (`nc-game`) remains unchanged for BBS door mode and players
who prefer the classic interface. Both crates share the same game data model
(`nc-data`), engine (`nc-engine`), and rendering primitives (`nc-ui`).

## Crate Boundaries

```
nc-game   ← legacy 80×25 BBS/door TUI (unchanged)
nc-dash   ← full-screen dashboard (this crate)
nc-ui     ← shared: PlayfieldBuffer, crossterm, themes, diff renderer
nc-data   ← shared: game state, records, economy formulas
nc-engine ← shared: maintenance engine, combat, reports
```

`nc-dash` produces its own binary. The sysop deploys `nc-game` for BBS
doors and retro terminals, `nc-dash` for modern SSH or local play.

`nc-dash` must not call into `nc-game`. The legacy crate is the UX and
workflow reference only. Shared neutral rendering primitives belong in
`nc-ui`; dashboard-specific overlays, prompts, and tables are implemented
inside `nc-dash`.

## Rendering Model

`nc-dash` uses the same `PlayfieldBuffer` + crossterm pipeline as `nc-game`.
No new rendering dependencies are needed.

- `PlayfieldBuffer` already supports arbitrary dimensions.
- The diff-based renderer (row fingerprinting) is size-agnostic.
- crossterm handles raw mode, alternate screen, color, and terminal size.
- No widget framework — direct cell-by-cell rendering for full control.

The dashboard creates a `PlayfieldBuffer` at the actual terminal size
(detected at startup and on resize) rather than the fixed 80×25 grid.

## Layout

Three-column layout with header and footer bars:

```
┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ NOSTRIAN CONQUEST              Foo1           Y3012                  Autopilot:OFF  Tax:40% │
├────────────────────┬──────────────────────────────────────────────────────────┬────────────────────┤
│ ECONOMY            │    01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 │ KNOWN GALAXY       │
│  Treasury:     820 │ 18  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  My       ■    5   │
│  Prod:  980/1200   │ 17  ·  ·  ·  ·  ·  ○  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Neutral  ○    3   │
│  Revenue:      210 │ 16  ·  ·  ■  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Enemy    ●    8   │
│  PP Gen:       +12 │ 15  ·  ·  ·  ·  ·  ·  ·  ●  ·  |  ·  ·  ◊  ·  ·  ·  ·  · │  ICD      ◊    2   │
├────────────────────┤ 14  ·  ·  ·  ·  ■  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Unch     ·   22   │
│ MY PLANETS         │ 13  ·  ·  ·  ·  ·  ·  ·  ·  ·  ○  ·  ·  ·  ·  ·  ·  ·  · ├────────────────────┤
│  Total Worlds:  12 │ 12  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ●  ·  ·  ·  ·  ·  · │ DIPLOMACY          │
│  Active Docks:   3 │ 11  ·  ·  ■  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo2    Neutral   │
│  Starbases:      2 │ 10  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo3    Enemy     │
│  Total Armies:  34 │ 09 ─·──·──·──·──·──·──·──·──·──O──·──·──·──·──·──·──·──· │  Foo4    Enemy     │
│  Grnd Batteries:12 │ 08  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ●  ·  ·  ·  · │  Foo5    Neutral   │
├────────────────────┤ 07  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo6    Neutral   │
│ MY FLEETS          │ 06  ·  ·  ·  ·  ·  ·  ○  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo7    Enemy     │
│  Total Fleets:   8 │ 05  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo8    Neutral   │
│  Total Ships:   84 │ 04  ·  ·  ·  ■  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · ├────────────────────┤
│  In Transit:     3 │ 03  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │ REPORTS  (5R, 2M)  │
│  Hostile:        2 │ 02  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  3rd Flt intercept │
│  Defensive:      2 │ 01  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  7th Flt bombarded │
│  Idle:           1 │                                                          │  SB 1 lost         │
├────────────────────┴──────────────────────────────────────────────────────────┴────────────────────┤
│ COMMAND <- ? P F I R D A S <Q> [10,09] ->                                                        │
└────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Classic-style 3-char-wide × 1-row-tall cells with crosshair and axis
labels. The center widget is currently sized to the rendered map exactly,
with no active interior padding. The padding constants still exist in the
layout code as future knobs, but the current compare state is tight. Row
numbers on the left flow
directly into the grid with no separator bar — just `09 ─·──·──` for the
crosshair row. Column numbers sit on their own row inside the map area,
centered over each 3-char cell. The crosshair highlights row 09 with dashes.
Sector intel lives in the right-side `SECTOR DETAIL` widget rather than on a
duplicate map status line.

The rendered 18x18 map block is 54 grid columns + 3 for row labels = ~57
columns center.
Side panels get the full 20+ rows of vertical space alongside the grid —
room for all economy, planet, fleet, diplomacy, and report data without
scrolling in most games.

The left and right side columns use internal horizontal separators between
their stacked sections, and all panel text must clip before crossing a
divider or the outer frame border.

### Header Bar

- **Left:** "NOSTRIAN CONQUEST" branding.
- **Center:** Empire name.
- **Right:** Game year, autopilot status (`AP:ON` / `AP:OFF` — bright/dim), tax rate.

All status fields are spelled out — no abbreviations. Right-justified
up to the border. Autopilot indicator rendered in a bright warning color
when on so the player never forgets he's on autopilot.

### Footer Bar

The dashboard footer is a real command line rendered with the shared prompt
grammar:

`COMMAND <- ? P F I R D A S <Q> [XX,YY] ->`

`[XX,YY]` always reflects the current crosshair position. The player may type
coordinates directly into that footer to jump the crosshair, using the same
punctuation-insensitive coordinate parser behavior as table typed-jump.

## Overlay Windows

Planet List, Fleet List, the Total Planet Database, the inbox, diplomacy,
help, and settings open as centered modal work windows inside the fullscreen
dashboard canvas:

- centered with visible outer padding
- boxed with proper borders and titles
- command rails and prompts rendered inside the popup box
- all table, preview, and prompt content clipped to the popup body

Dense work overlays use a calm full-screen backdrop so the dashboard does not
show through behind busy tables:

- `Planet List`
- `Fleet List`
- `Total Planet Database`
- `Inbox`

Lighter overlays such as diplomacy, help, and settings keep the live
dashboard visible behind the popup.

Helper modals are dismissal-only. They should use a plain dismissal footer
like `(slap a key)` instead of a `COMMAND <- ... ->` rail. More generally,
command-line chrome is reserved for modals that are actually collecting or
advertising actionable commands. Dismissal footers must render entirely inside
the modal footer span and must never overwrite the modal borders. In dash,
`(slap a key)` means exactly that: any key dismisses the modal.

Helper popup bodies should stay flat and command-focused:

- one command or action per row
- no combined rows such as `Q/Esc` or `B / A / C`
- no internal category headings or grouped sections inside the popup body
- aligned `Key : Description` columns throughout the helper

Within any dash modal or widget body that uses colon-separated label/value
rows, the `:` column should align consistently across that block, matching the
`INFO ABOUT A PLANET:` detail layout. Ad hoc `Label:value` spacing is not
allowed for dashboard widgets or modal body text.

For the three stacked left-column dashboard widgets (`ECONOMY`, `MY PLANETS`,
and `MY FLEETS`), treat the whole column as one shared label/value grid:
their `:` columns should align vertically across the overall dashboard, not
independently per widget.

These overlays should match `nc-game` behavior and command semantics where
applicable, but they should use the extra dashboard real estate rather than
the legacy 80×25 geometry.

### Left Column (adaptive width)

Three stacked sections, all visible simultaneously:

- **Economy:** Treasury (total stored production across all planets),
  Prod (current production), Pot Prod (potential production),
  Revenue (this turn's tax income), PP Gen (raw yearly production points
  generated by growth), and % Growth (that same generated growth expressed
  against current production). The player sees spending power, capacity gap,
  and how quickly the empire is still developing.
- **My Planets:** Summary of planetary assets:
  `Tot Worlds`, `Act Docks`, `Starbases`, `Tot Armies`, `GBs`. A
  `Vulnerable` count appears in red if any owned worlds have 0 armies and 0
  batteries.
- **My Fleets:** Summary of space assets and their current postures:
  `Tot Fleets`, `Tot Ships`, `Docked`, `In Transit`, `Hostile`, `Defensive`,
  `Idle`.

Both lists use a matching `key : value` format, with the left column sharing
one common `:` column across all three stacked widgets. Labels may be
abbreviated where useful, but the left column itself now sizes from the
widest rendered left-widget row in the current game state rather than staying
fixed-width.



### Center (fills remaining dashboard space)

Full starmap rendered as a projected grid. The galaxy remains logically
`18×18`, `25×25`, or `36×36`, but the on-screen sector rectangles expand to
fill the entire center widget exactly. Axes are pinned to the widget's
top-left corner. The right-side `SECTOR DETAIL` widget owns the selected-sector
intel display.

The center pane consumes all remaining dashboard width and height after the
content-sized side widgets are measured.

### Right Column (adaptive width)

Three stacked sections:

- **Known Galaxy:** Summary counts of known worlds by category — My Worlds,
  Neutral, Enemy, and Uncharted (total map sectors minus known). Gives the
  player a quick sense of exploration progress and threat landscape.
- **Diplomacy:** List of other empires, color-coded by diplomatic status
  (green = neutral, red = enemy). Up to 11 others in a 12-player game.
  Compact one-line-per-empire format.
- **Sector Detail:** Passive detail for the sector under the map crosshair.
  Shows the selected world's most useful condensed information: planet name,
  owner, `E`conomy (`Potential|Current|Points`), `D`efenses
  (`Armies|Ground Batteries|Starbases`), plus state/intel/build/docked
  summaries that fit comfortably in the panel. Empty sectors show
  `empty sector`. The full inbox overlay remains available on `R`.

### Startup Flow

`nc-dash` preserves the classic nc-game intro sequence before the
dashboard appears:

1. Splash screen (branding, version)
2. Nostr npub authentication (automatic)
3. Reports narrative — scrolling one-by-one with delete Y/N prompts
4. Messages — presented in order
5. **Dashboard** — replaces the classic 80×25 main menu

The dashboard is what the player sees after the intro flow completes.
The intro screens render fullscreen using the same `PlayfieldBuffer`
but with their own layout, not the three-column dashboard.


### Minimum Terminal Size

- The dashboard now measures its required frame from the current game state.
  It refuses to start if the current terminal is smaller than the measured
  frame.
- The dashboard shell now consumes the full terminal canvas.
- Startup still checks a measured minimum:
  - left column widest rendered widget row
  - plus right column widest rendered widget row
  - plus enough center width for 2-digit top-axis labels
  - plus enough center height for top axis + one row per sector
  - plus shell/dividers

## Starmap

### Viewport

By default the starmap uses a **Readable** view mode. The whole dashboard
shrinks back to its measured content before being centered in the terminal.
In this mode the map uses the same exact-fill projection and zoom behavior as
fill mode, but inside the smaller readable dashboard frame instead of the
full terminal canvas.

An alternate **Fill** view mode projects the map into the full center widget
using exact-fill rasterization, so sector widths/heights may vary by at most
1 character/row while still consuming the widget exactly.

When zoomed in, either mode shows a cursor-following viewport subset of the
galaxy. The logical crosshair remains in world coordinates and the viewport
recenters around it when possible. `Z` resets the current mode back to its
default full-map fit for that mode's frame.

### Display

- Empty sectors: dim dot (`·`) centered in the projected sector.
- Owned planets: bright color matching empire, centered in the projected sector.
- Enemy planets: hostile marker centered in the projected sector.
- ICD (Civil Disorder) planets: dimmed or distinct marker (`◊`). These
  are prime expansion targets — visible at a glance on the map.
- Neutral/unowned planets: gray marker centered in the sector.
- Crosshair: uses the same exact-fill row/column highlight treatment in both
  readable and fill modes. Readable mode differs only by using the smaller
  centered dashboard frame.
- Selected sector: shown in the `SECTOR DETAIL` widget.

### Terminal Resize

If the player resizes the terminal mid-session:
- Re-detect terminal dimensions on the next render cycle.
- Recompute panel geometry and re-render the full frame.
- If the terminal drops below the newly measured frame requirement for the
  current game state, show a warning overlay asking the player to resize or
  press Q to quit.

## Input Model

All interaction is keyboard-driven. The dashboard footer is a real command
line showing the active map command rail and live coordinate default.

### Focus Model

The dashboard has a focus system. One panel is active at a time. The
focused panel gets a highlighted border/title. Unfocused panels show
static summaries (top N items that fit).

- **Tab / Shift+Tab:** Cycle focus: Map → Economy → Planets → Fleets →
  Galaxy → Diplomacy → Map.
- **Esc:** Unfocus current panel, return to map. Dismiss any open popup.

### Panel List Navigation (when a side panel is focused)

- **Arrow up/down, j/k:** Scroll one item.
- **PgUp / PgDn:** Scroll one page.
- **Home / End:** Jump to top / bottom of list.
- **Enter:** Open detail popup for the selected item when that panel supports
  it.

When a panel is focused, the other panels freeze at their current scroll
position. The map remains visible.

### Detail Popups

**Enter** on the map opens a detail popup for the world under the crosshair.
List/overlay views may also use **Enter** for row review or detail. The
detail popup overlays the map area (center column) and leaves the side panels
visible for context.

- **Fleet detail popup:** Fleet number, full ship composition, location,
  order, target, speed, max speed, ROE, ETA, loaded armies. Action
  hotkeys at the bottom: O:Order, C:Speed, E:ROE, M:Merge, T:Transfer.
- **Planet detail popup:** Mirrors `nc-game` planet info: coordinates,
  planet, owner, state, economy, defenses, orbit/docked/build status, and
  intel freshness where applicable.
- **Report detail popup:** Full report text, word-wrapped to fit the
  overlay width. Scrollable if the report is long.
- **Diplomacy detail popup:** Empire name, diplomatic status, known
  planets, known fleet count, production score. Action: D:Toggle status.

**Esc** dismisses the popup and returns to the focused panel list.

### Overlay Screens

All overlay screens (**P**, **F**, **I**, **R**, **D**, **S**, **?**)
render as centered popup boxes with padding around the border — not
fullscreen edge-to-edge. The dashboard remains dimly visible behind
the overlay, preserving spatial context.

- Box borders use Unicode box-drawing characters.
- Tables and borders dynamically size to fit their content, centered
  on screen. A small fleet list gets a compact box; a large planet
  table gets a wider one. Minimum padding of ~4 chars on each side.
- Command prompt sits inside the box at the bottom, following the
  nc-game prompt standard (`LABEL <- ... <Q> ->`). See
  [ec-game-prompt-standard.md](../dev/ec-game-prompt-standard.md).
  The main map-screen dashboard also follows this standard, using its own
  dashboard command line instead of a literal legend bar.
- **Enter** on a row opens a detail popup where that overlay supports it.
- **?** from within any overlay opens a context-sensitive help popup
  (boxed, dynamically sized, centered, padded — same style as nc-game's
  help boxes). Shows the available commands for that specific overlay.
  **Esc** or **?** dismisses it back to the overlay.
- **Esc** closes the overlay and returns to the dashboard.

This keeps the interaction model consistent with nc-game — players who
know the legacy TUI already know the overlays. The centered box framing
makes them feel like focused work windows rather than full mode switches.

All popup modals, including center-pane detail popups over the starmap,
must follow the same padded modal treatment. A popup border should never
sit flush against its parent pane when there is room for visible outer
padding.

### Map Navigation (when map is focused)

- **Arrow keys, h/j/k/l:** Move crosshair.
- **Type `XX,YY`:** Jump crosshair directly to map coordinates.
- **[ / ]:** Jump to the previous or next planet in wrapped screen order.
  These keys wrap across map edges; ordinary crosshair movement does not.
- **+ / =:** Zoom in.
- **-:** Zoom out.
- **Z:** Reset the map zoom for the current view mode.
- **V:** Toggle readable and fill map view.
- **Enter:** Open planet detail for the world under the crosshair.

### Global Hotkeys (always active, any focus)

- **Tab / Shift+Tab:** Cycle focus between panels.
- **P:** Planet list overlay (manage your planets).
- **F:** Fleet list overlay (manage your fleets).
- **I:** Planet database overlay (all known planets / intel).
- **R:** Inbox overlay (reports + messages + compose).
- **D:** Diplomacy overlay (leaderboard + declare enemies).
- **A:** Toggle autopilot on/off (reflected in header).
- **?:** Help overlay.
- **S:** Settings (theme picker, mouse toggle).
- **Q:** Quit.

### Mouse (Optional)

crossterm supports mouse events. If enabled:
- Click on map sector to move crosshair.
- Click on item in a side panel list to select.
- Scroll wheel to scroll focused list panel.

Mouse support is optional — the dashboard is fully keyboard-navigable.

### Footer Command Line

The dashboard footer stays on one shared map-screen command line:

`COMMAND <- ? P F I R D A S <Q> [XX,YY] ->`

`[XX,YY]` is the live default taken from the current crosshair position.
When the player types coordinate characters, the shared coordinate parser
reuses the same punctuation-insensitive matching behavior as table typed-jump.
Partial matches move the crosshair immediately and keep the typed input
visible; a terminal exact match clears the live input automatically.

## Inbox (R Overlay)

The inbox is a centered, padded popup overlay combining reports and
messages in a unified view. Boxed and dynamically sized like all other
overlays. It reuses the same data models from nc-game (`ReportBlockRow`
and `QueuedPlayerMail`) stored in the campaign SQLite database.

### Current nc-game Implementation (to reuse)

The existing inbox in nc-game already provides:
- Unified view of reports and messages sorted by year/week
- Type filter (M:Messages, R:Reports, A:All)
- Year filter (Y to enter 4-digit year)
- Split-pane layout: item list left, preview right
- Auto-classified report subjects (Combat, Bombard, Scout, etc.)
- Soft-delete with D key (marks `recipient_deleted`, does not purge)
- 2-digit quick-jump to item by display ID
- Message composer with recipient selection, subject (60 char limit),
  body (1,000 char limit, word-wrapped), send/discard confirmations
- Outbox view for queued unsent messages
- 3 messages per opponent per turn limit enforced

### nc-dash Improvements

The fullscreen overlay has much more room than the 80×25 version. Take
advantage of it:

**Layout:** Split-pane with command line. Boxed, centered, padded like
all overlays. Tab switches focus between item list and preview pane.

```
       ┌─────────────────────────────────────────────────────────────────────┐
       │ INBOX                                        Filter: All  Year: All │
       ├───────────────────┬─────────────────────────────────────────────────┤
       │ ID  Type  Date    │                                                 │
       │► 01  R  03/3012   │ From your 3rd Fleet, located in                 │
       │  02  R  03/3012   │ System(9,13):           Stardate: 03/3012       │
       │  03  R  02/3012   │                                                 │
       │  04  M  02/3012   │ Bombardment mission report: We have just        │
       │  05  R  01/3012   │ concluded a bombing run against planet          │
       │  06  R  01/3012   │ "Colony B". The target world was defended by    │
       │  07  M  01/3012   │ 4 ground battery(ies) and 10 army(ies). We      │
       │  08  R  52/3011   │ suffered no ship losses. Planetary batteries    │
       │  09  R  52/3011   │ absorbed our bombardment. The world's           │
       │  10  R  51/3011   │ infrastructure remains shielded. We are         │
       │  ..  ..  .......  │ maintaining bombardment position and will       │
       │                   │ continue next turn.                             │
       │                   │ <end of transmission>                           │
       ├───────────────────┴─────────────────────────────────────────────────┤
       │ COMMAND <- ? Tab M R A Y D C <Q> ->                                 │
       └─────────────────────────────────────────────────────────────────────┘
```

**Improvements over 80×25 version:**
- Centered popup box with padding — dashboard visible behind.
- Preview pane is much wider — full report text without wrapping.
- Item list shows more rows (15+ vs 8 in classic).
- Tab switches focus between item list and preview pane.
- Standard command line at bottom with ? help.
- The dashboard summary panel stays compact while `R` handles full reading.

### Navigation

- **↑↓ / j/k:** Scroll item list.
- **PgUp / PgDn:** Page through items.
- **Home / End:** Jump to newest / oldest.
- **0-9:** Quick-jump by 2-digit display ID.
- **Enter:** Select item (focuses preview pane for scrolling).
- **Tab:** Toggle focus between item list and preview pane.
- **M / R / A:** Filter by Messages / Reports / All.
- **Y:** Toggle year filter (enter 4-digit year or clear).
- **D:** Delete selected item (soft-delete with Y/N confirm).

### Compose (C from Inbox)

Opens the message composer as a sub-overlay:

1. **Recipient:** List of empires, select with ↑↓ + Enter.
2. **Subject:** Single-line input, 60 char limit.
3. **Body:** Multi-line editor, 1,000 char limit, word-wrapped.
   Arrow keys, Home/End, Enter for newlines, Backspace/Delete.
   Ctrl+E to send, Ctrl+X to discard. Character count shown.
4. **Confirm:** "Send message? Y/N" prompt.
5. **Outbox:** View/delete queued messages before they're delivered.

The composer reuses the nc-game composition logic and data model
(`QueuedPlayerMail`). The 3-message-per-opponent-per-turn limit is
enforced and shown in the status line.

### Inbox Tracking

nc-game has no separate read/unread tracking, and nc-dash currently mirrors
that runtime model:
- Reports and messages stay visible until the recipient explicitly deletes them.
- There is no separate unread state on the dashboard; inbox detail lives in
  the `R` overlay.
- The inbox overlay (`R`) is where the player reads, filters, previews, and
  deletes items.
- Soft-delete with `D` marks `recipient_deleted`; it does not purge history
  from the runtime snapshot immediately.

## Planet List (P Overlay)

Fullscreen overlay for managing owned planets. Reuses nc-game's planet
list table infrastructure (`TableColumn`, `write_table_window_*`) and
command-line interaction model.

### Layout

The existing nc-game planet list is row-centric with a command prompt
at the bottom. nc-dash renders this at fullscreen width with the same
column layout, expanded to fill available space.

### Columns

Reuse from nc-game planet list:
- Planet name, coordinates, present/potential production, stored points,
  armies, batteries, stardock status, build queue summary.

### Command Rail

```
COMMAND <- ? B A C L U X S I T <Q> ->
```

Same actions as nc-game planet list: B:Build, A:Auto-commission,
C:Commission, L/U:Load/Unload, X:Scorch, S:Sort, I:Info, T:Tax.

## Fleet List (F Overlay)

Fullscreen overlay for managing fleets. Reuses nc-game's fleet list
table infrastructure and command-line interaction model.

### Columns

Reuse from nc-game fleet list, with starbases included as rows:
- Fleet number (or `SB N` for starbases), location, order, target,
  speed, ETA, ROE, armies, ship composition.
- Starbases appear in the same table — no separate Starbase Command.

### Command Rail

```
COMMAND <- ? O C M T <Q> ->
```

Same actions as nc-game fleet command: O:Order, C:ROE, M:Merge (disabled
for starbases — they cannot merge), T:Transfer.

## Planet Database (I Overlay)

Fullscreen overlay for the intel database — all known planets from
scouting, viewing, combat, and colonization attempts. This feeds the
starmap markers.

### Columns

Reuse from nc-game planet database:
- Planet name, coordinates, owner, max production, current production,
  stored points, armies, batteries, starbases, year last scouted.

### Filters

nc-game database supports filters. Reuse:
- All, Range, Empire, Max Production
- Sort by: Location, Range, Empire, Max Production

### Command Rail

```
COMMANDS <- ? S <Q> ->
```

S:Sort cycles through sort modes. Selecting a planet and pressing Q
highlights that planet's sector on the starmap.

## Diplomacy (D Overlay)

Fullscreen overlay combining nc-game's Rankings and Enemies screens into
a single table. Reuses nc-game's table infrastructure and command-line
interaction model.

### Columns

Single table merging leaderboard and diplomatic status:
- Rank, Empire Name, ID, Planets, Production, Campaign State (Stable /
  Civil Disorder / etc.), Diplomatic Status (Neutral / Enemy).
- Player's own empire highlighted.
- Diplomatic status column color-coded (green = neutral, red = enemy).
- Sortable by production (default), planets, name.

### Command Rail

```
COMMAND <- ? D S <Q> ->
```

D:Declare enemy/neutral, S:Sort.

## Theme and Settings

nc-dash supports the same color theme system as nc-game. The theme
engine lives in `nc-ui` and `nc-game/src/theme.rs`. nc-dash reuses
these.

### Theme Picker

Accessible from the Settings overlay (**S**). Reuses nc-game's
`ThemePickerScreen` table with available themes:
- `tokyo_night` (bundled)
- `mag16` (classic ANSI16)
- Other bundled palettes
- Theme selection persisted per empire per campaign

### Settings Overlay

Accessible via **S** from the dashboard. Provides:
- **Theme:** Select color theme (opens theme picker)
- **Mouse:** Toggle mouse support on/off
- **Sound:** Toggle notification sounds (future)

Settings are minimal — the game is keyboard-first and most config is
empire-level (tax rate, autopilot) rather than client-level.

### Color Theming

The entire dashboard — header, footer, panels, starmap, popups, overlays
— renders through the theme system. All colors are theme-defined, never
hardcoded RGB values. The themes from nc-game work unchanged in nc-dash
because both use `GameColor` from `nc-ui`.

Empire colors on the starmap and diplomacy screen use a stable 12-slot,
theme-defined palette keyed to player number. Custom themes may omit that
palette and fall back to the built-in default slot colors.

## Help Overlay (?)

**?** from any screen opens a help overlay covering the center column.
Shows context-sensitive commands for the current mode plus a compact
dashboard reference.

```
┌────────────────────────────────────┐
│ HELP                               │
│                                    │
│ GLOBAL                             │
│  P  Planet List    F  Fleet List   │
│  I  Intel Database R  Inbox        │
│  D  Diplomacy      A  Autopilot    │
│  X  Tax Rate       S  Settings     │
│  ?  This Help      Q  Quit         │
│                                    │
│ DASHBOARD                          │
│  Tab        Cycle focus            │
│  Esc        Back / unfocus         │
│                                    │
│           Press ? or Esc to close  │
└────────────────────────────────────┘
```

The help overlay is command-focused. It should mirror the visible command
rails for each overlay and only add unobvious non-rail actions such as typed
jump, `Enter` review/open behavior, or `Tab` focus switching. It should not
repeat generic movement, paging, or erase keys.

## Data Flow

`nc-dash` reads the same `CoreGameData` from the campaign store that
`nc-game` uses. The game state is loaded at startup, mutated by player
orders, and saved back. The maintenance engine (`nc-engine`) runs
identically regardless of which frontend is active.

Turn submission, report reading, and all game mechanics are shared code
in `nc-data` and `nc-engine`. The dashboard is purely a presentation layer.

## Module Layout

Per AGENTS.md: no giant monolithic source files. Each feature area gets its
own focused module. Split early, split often.

```
nc-dash/
  src/
    main.rs              ← entry point, CLI args, terminal setup
    app/
      mod.rs             ← App struct, main loop, action dispatch
      state.rs           ← dashboard state (focus, selection, scroll positions)
      input.rs           ← key/mouse event → Action mapping
      render.rs          ← top-level render dispatch to panels and overlays
    layout/
      mod.rs             ← three-column frame, border drawing, resize
      geometry.rs        ← panel dimensions from terminal size, max width cap
    panels/
      mod.rs             ← panel trait, focus management
      economy.rs         ← left: treasury, production, revenue, growth
      planets.rs         ← left: owned planet summary stats
      fleets.rs          ← left: active fleet summary stats
      starmap.rs         ← center: sector grid, crosshair, axis labels
      known_galaxy.rs    ← right: world counts by category
      diplomacy.rs       ← right: empire list, color-coded status
      sector_detail.rs   ← right: selected sector / planet summary
    overlays/
      mod.rs             ← overlay trait, Esc:Back handling
      planet_list.rs     ← P: fullscreen planet management table
      fleet_list.rs      ← F: fullscreen fleet + starbase table
      intel_database.rs  ← I: fullscreen planet database with filters
      inbox.rs           ← R: unified reports + messages + compose
      diplomacy.rs       ← D: merged leaderboard + relations
      settings.rs        ← S: theme picker, mouse toggle
      help.rs            ← ?: keyboard reference overlay
    popups/
      mod.rs             ← popup rendering over map area
      fleet_detail.rs    ← fleet info + action hotkeys
      planet_detail.rs   ← planet info + action hotkeys
      report_detail.rs   ← full report text, scrollable
    startup/
      mod.rs             ← classic intro flow (splash, auth, reports, messages)
    theme/
      mod.rs             ← reuse nc-game theme system, empire color palette
```

Guidelines:

- Each panel is a self-contained module that takes data and returns a
  rendered region (sub-buffer or direct cell writes into a buffer slice).
- No panel module should exceed ~300 lines. If it does, split the rendering
  helpers, data extraction, and interaction logic into sub-modules.
- Shared layout primitives (box drawing, text truncation, scrollbar) belong
  in `layout/` or `nc-ui`, not duplicated across panels.
- The `app/` module handles the event loop and delegates to panels. It should
  not contain rendering logic for any specific panel.
- State for each panel (scroll offset, selected index) lives in `app/state.rs`
  as flat fields, not nested structs — keep it simple until complexity demands
  otherwise.

## Implementation Phases

### Phase 1: Scaffold

- Create `nc-dash` crate with `Cargo.toml` depending on `nc-ui`, `nc-data`.
- Implement terminal size detection and measured minimum size check.
- Max rendering area with centering and dark padding.
- Render static three-column frame with header, footer, panel borders.
- Verify crossterm rendering at full terminal size.
- Theme system integration from nc-game.

### Phase 2: Starmap

- Render full map (18×18 through 36×36) with 3-char-wide × 1-row cells.
- Axis labels: row numbers left, column numbers top.
- Crosshair: red dashed horizontal and vertical lines.
- No duplicate status line below grid; sector intel lives in `SECTOR DETAIL`.
- Arrow/hjkl navigation, direct `XX,YY` coordinate jump, and `[` / `]` planet jump.
- Planet markers: ■ owned, ● enemy, ○ neutral, · empty.
- Fleet markers overlaid on sectors.

### Phase 3: Side Panels

- Left: Economy (treasury, production, revenue, growth), My Planets
  (summary stats), Active Fleets (summary stats).
- Right: Known Galaxy (world counts), Diplomacy (empire list, color-coded),
  Sector Detail (selected world summary).
- Tab focus cycling, scrolling within focused panel.

### Phase 4: Classic Intro Flow

- Splash screen, nostr auth, report narrative, messages.
- Reuse nc-game startup sequence rendering at fullscreen.
- Transition to dashboard after intro completes.

### Phase 5: Fullscreen Overlays

- P: Planet List (reuse nc-game table + command line).
- F: Fleet List with starbases (reuse nc-game table + command line).
- I: Planet Database (reuse nc-game intel table + filters).
- R: Inbox (reuse nc-game inbox with wider layout, unread tracking).
- D: Diplomacy (merged leaderboard + relations).
- S: Settings (theme picker, mouse toggle).
- ?: Help overlay.

### Phase 6: Detail Popups

- Fleet detail popup with action hotkeys.
- Planet detail popup with action hotkeys.
- Report detail popup (scrollable full text).
- Render over map area, side panels stay visible.

### Phase 7: Polish

- Mouse support (optional).
- Terminal resize handling (re-detect size, re-render frame).
- Color refinement across all panels and overlays.
- Keyboard conflict audit (no hotkey collisions between modes).

## Reference

- Mockup: [docs/assets/ideas/nc_hud_mockup.png](../assets/ideas/nc_hud_mockup.png)
- Legacy TUI: `nc-game` crate
- Rendering primitives: `nc-ui` crate
