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
│  Growth:       +12 │ 15  ·  ·  ·  ·  ·  ·  ·  ●  ·  |  ·  ·  ◊  ·  ·  ·  ·  · │  ICD      ◊    2   │
├────────────────────┤ 14  ·  ·  ·  ·  ■  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Unch     ·   22   │
│ MY PLANETS         │ 13  ·  ·  ·  ·  ·  ·  ·  ·  ·  ○  ·  ·  ·  ·  ·  ·  ·  · ├────────────────────┤
│  Total Worlds:  12 │ 12  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ●  ·  ·  ·  ·  ·  · │ DIPLOMACY          │
│  Active Docks:   3 │ 11  ·  ·  ■  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo2    Neutral   │
│  Starbases:      2 │ 10  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo3    Enemy     │
│  Total Armies:  34 │ 09 ─·──·──·──·──·──·──·──·──·──O──·──·──·──·──·──·──·──· │  Foo4    Enemy     │
│  Grnd Batteries:12 │ 08  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ●  ·  ·  ·  · │  Foo5    Neutral   │
├────────────────────┤ 07  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo6    Neutral   │
│ ACTIVE FLEETS      │ 06  ·  ·  ·  ·  ·  ·  ○  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo7    Enemy     │
│  Total Fleets:   8 │ 05  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  Foo8    Neutral   │
│  Total Ships:   84 │ 04  ·  ·  ·  ■  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · ├────────────────────┤
│  In Transit:     3 │ 03  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │ REPORTS  (5R, 2M)  │
│  Hostile:        2 │ 02  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  3rd Flt intercept │
│  Defensive:      2 │ 01  ·  ·  ·  ·  ·  ·  ·  ·  ·  |  ·  ·  ·  ·  ·  ·  ·  · │  7th Flt bombarded │
│  Idle:           1 │    Sector (10,09) ■ Colony A — 45 prod, 10 AR, 4 RB      │  SB 1 lost         │
├────────────────────┴──────────────────────────────────────────────────────────┴────────────────────┤
│ P:Planets  F:Fleets  I:Intel  R:Inbox  D:Diplomacy  A:Autopilot  X:Tax  S:Settings  Q:Quit  ?      │
└────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Classic-style 3-char-wide × 1-row-tall cells with crosshair and axis
labels. Row numbers on the left flow directly into the grid with no
separator bar — just `09 ─·──·──` for the crosshair row. Column numbers
sit on their own row inside the map area, centered over each 3-char cell.
The crosshair highlights row 09 with dashes. A status line below the grid
shows the sector under the crosshair with planet/fleet intel.

The map area is 54 grid columns + 3 for row labels = ~57 columns center.
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

Context-sensitive hotkey legend — shows available actions for the currently
focused panel. No command-line input. All interaction is through keyboard
shortcuts and panel navigation. The footer updates when focus changes
between panels.

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
the modal footer span and must never overwrite the modal borders.

Within any dash modal or widget body that uses colon-separated label/value
rows, the `:` column should align consistently across that block, matching the
`INFO ABOUT A PLANET:` detail layout. Ad hoc `Label:value` spacing is not
allowed for dashboard widgets or modal body text.

These overlays should match `nc-game` behavior and command semantics where
applicable, but they should use the extra dashboard real estate rather than
the legacy 80×25 geometry.

### Left Column (20 chars)

Three stacked sections, all visible simultaneously:

- **Economy:** Treasury (total stored production across all planets),
  Prod (current/potential — shows growth headroom at a glance),
  Revenue (this turn's tax income), Growth (net production change
  per turn). The player sees his spending power, capacity gap, and
  whether the empire is growing or stagnating.
- **My Planets:** Summary of planetary assets:
  Total Worlds, Active Docks (planets with items in stardock), Starbases
  (active owned starbases), Total Armies, Grnd Batteries. A "Vulnerable"
  count appears in red if any owned worlds have 0 armies and 0 batteries.
- **Active Fleets:** Summary of space assets and their current postures:
  Total Fleets, Total Ships, In Transit (moving or seeking), Hostile
  (bombarding or invading), Defensive (patrolling or guarding), Idle (holding).

Both lists use a matching `key: value` format for visual consistency and
maximum density.



### Center (variable width)

Full starmap. Classic-style 3-char-wide × 1-row-tall cells for all map
sizes (18×18 through 36×36). The entire map is always visible — no
panning. Row numbers descend on the left. Column numbers across the top.
Red dashed crosshair highlights the cursor row and column. A status line
below the grid shows intel for the sector under the crosshair.

Grid dimensions scale with map size:
- 18×18 map: 54 cols × 18 rows + axis/status
- 25×25 map: 75 cols × 25 rows + axis/status
- 36×36 map: 108 cols × 36 rows + axis/status

Side panels get whatever terminal width remains after the grid.

### Right Column (20 chars)

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
  `No world in sector`. The full inbox overlay remains available on `R`.

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

- **160 columns × 40 rows** minimum. Assumes 1920×1200 display at
  reasonable scaling. Fits all map sizes (up to 36×36 = 111 grid cols)
  with room for side panels.
- If the terminal is smaller, the dashboard refuses to start and suggests
  using `nc-game` instead.
- **Max rendering width: ~155 columns** (20 left + 111 grid + 20 right +
  borders). Centered on screen with dark padding if terminal is wider.
- Max rendering height capped similarly; centered vertically with padding.
- Side panels are 20 chars wide each — enough for ★ indicators, coords,
  and production numbers with clean spacing.
- Taller terminals extend the side panels above and below the map for more
  list entries and longer report text.

## Starmap

### Viewport

The full map is always rendered — no viewport panning. Each sector is
3-char-wide × 1-row-tall, matching the classic View Starmap. Row numbers
on the left, column numbers on top. All map sizes from 18×18 to 36×36
fit at 3-char-wide on a 1920×1200 minimum display.

Grid width = map_size × 3 + 3 (row labels). Grid height = map_size + 2
(column labels + status line).

### Display

- Empty sectors: dim dot (`·`) on dark background.
- Owned planets: bright color matching empire (`■`).
- Enemy planets: red/hostile color (`●`).
- ICD (Civil Disorder) planets: dimmed or distinct marker (`◊`). These
  are prime expansion targets — visible at a glance on the map.
- Neutral/unowned planets: gray (`○`).
- Fleets: triangle or arrow marker (`▸`) in the sector. When a fleet and
  planet share a sector, the planet marker takes priority and the fleet
  presence is indicated by a brighter color or underline. The status line
  shows both when the crosshair is over such a sector.
- Crosshair: red dashed line through the cursor row and column.
- Selected sector: shown in status line below grid with intel.

### Terminal Resize

If the player resizes the terminal mid-session:
- Re-detect terminal dimensions on the next render cycle.
- Recompute panel geometry and re-render the full frame.
- If the terminal drops below minimum (160×40), show a warning overlay
  asking the player to resize or press Q to quit.

## Input Model

All interaction through keyboard shortcuts. No command-line prompt.

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
  The only screen that does NOT follow this standard is the main
  map-screen dashboard, which uses its own hotkey footer bar.
- **Enter** on a row opens a detail popup where that overlay supports it.
- **?** from within any overlay opens a context-sensitive help popup
  (boxed, dynamically sized, centered, padded — same style as nc-game's
  help boxes). Shows the available commands for that specific overlay.
  **Esc** or **?** dismisses it back to the overlay.
- **Esc** closes the overlay and returns to the dashboard.

This keeps the interaction model consistent with nc-game — players who
know the legacy TUI already know the overlays. The centered box framing
makes them feel like focused work windows rather than full mode switches.

### Map Navigation (when map is focused)

- **Arrow keys, h/j/k/l:** Move crosshair.
- **[ / ]:** Jump to the previous or next planet in wrapped screen order.
  These keys wrap across map edges; ordinary crosshair movement does not.
- **Enter:** Open planet detail for the world under the crosshair.
- **G:** Go-to — enter coordinates to jump crosshair.
- **Home:** Center crosshair on homeworld.
- **1-9:** Jump crosshair to fleet by number.

### Global Hotkeys (always active, any focus)

- **Tab / Shift+Tab:** Cycle focus between panels.
- **P:** Planet list overlay (manage your planets).
- **F:** Fleet list overlay (manage your fleets).
- **I:** Planet database overlay (all known planets / intel).
- **R:** Inbox overlay (reports + messages + compose).
- **D:** Diplomacy overlay (leaderboard + declare enemies).
- **A:** Toggle autopilot on/off (reflected in header).
- **X:** Change empire tax rate (inline prompt).
- **?:** Help overlay.
- **S:** Settings (theme picker, mouse toggle).
- **Q:** Quit.

### Mouse (Optional)

crossterm supports mouse events. If enabled:
- Click on map sector to move crosshair.
- Click on item in a side panel list to select.
- Scroll wheel to scroll focused list panel.

Mouse support is optional — the dashboard is fully keyboard-navigable.

### Footer Hotkey Legend

The footer bar shows context-sensitive hotkeys. Left side shows panel/mode
actions; right side always shows the global overlay keys.

The footer is the same global bar shown in the mockup. The overlay keys
(P, F, I, R, D, A, X, S, Q, ?) are always visible. When inside an
overlay or popup, the overlay's own footer replaces the global bar with
context-specific actions and an Esc:Back hint.

The footer updates whenever focus or mode changes.

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
- Implement terminal size detection and minimum size check (160×40).
- Max rendering area with centering and dark padding.
- Render static three-column frame with header, footer, panel borders.
- Verify crossterm rendering at full terminal size.
- Theme system integration from nc-game.

### Phase 2: Starmap

- Render full map (18×18 through 36×36) with 3-char-wide × 1-row cells.
- Axis labels: row numbers left, column numbers top.
- Crosshair: red dashed horizontal and vertical lines.
- Status line below grid with sector intel.
- Arrow/hjkl navigation, G:Goto, Home:Homeworld, 1-9:Fleet jump.
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
