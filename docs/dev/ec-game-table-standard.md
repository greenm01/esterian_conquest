# `ec-game` Table Standard

This document defines the standard table format for the Rust player TUI.

It is intended to stop screen-by-screen table drift. Shared table rendering,
column budgeting, selection behavior, and the bottom command bar should follow
this spec unless a screen has a narrow, explicit exception.

Read it together with:

- [bbs_door_client_rust.md](bbs_door_client_rust.md)
- [tui_style_guide.md](tui_style_guide.md)
- [rust-architecture.md](rust-architecture.md)

## Core Rules

- The player TUI targets a fixed `80x25` playfield.
- Shared tables use single-line ANSI/CP437-style joined borders.
- Table borders and divider lines use the same subdued chrome color.
- Shared tables use a restrained Tokyo Night-inspired palette on a single dark
  background.
- Shared table colors come from the semantic ANSI-16 theme described in
  [tui_style_guide.md](tui_style_guide.md).
- The rightmost terminal column, `79`, is reserved for the scroll gutter when
  a scroll indicator is present.
- A full-width table therefore ends its right border at column `78`.
- Table headers are monochrome and restrained:
  - no grouped color bands
  - no stacked/grouped top header row as the normal standard
- The command row sits directly under the rendered table.
- For short tables, the command row may appear above screen row `24`.
- For max-height tables, the command row lands on row `24`.
- Table command bars use the generic label `COMMANDS`, not a repeated screen
  title.

## Standard Shape

Normal shared tables use this vertical structure:

1. Title row
2. Top border immediately below the title row
3. One header row inside the border
4. Joined divider row
5. Body rows
6. Bottom border
7. Command row directly below the bottom border

There is no blank spacer row between the title and the top border.

## Coordinate Convention

Coordinate-based tables use:

- header label: `(X,Y)`
- body values: zero-padded `(00,00)`

The padding is required for clean visual alignment inside bordered tables.

Command-bar default coordinate input uses square-bracketed defaults:

- `[03,03]`

That mirrors the client’s other prompt-default conventions while keeping the
table cells themselves visually distinct from the borders.

For multi-select tables, the identity column still stays first. Marker columns
such as `Sel` come second.

## Owned-Planet List Standard

The owned-planet list is the normative reference table.

Its columns are:

- `(X,Y)`
- `Name`
- `Curr`
- `Max`
- `Points`
- `Rev`
- `Grow`
- `Docked`
- `SB`
- `ARs`
- `GBs`

Additional rules:

- `Docked` is the number of ships currently sitting in the planet’s stardock.
- `SB` uses `Y` / `N`.
- The first column is always the identity column for navigable tables.
- Only the selected identity cell is highlighted; the rest of the row stays
  normal.
- On first open, the selected row is the top row.

## Normative Example

This example is the reference mockup for the owned-planet table.

```text
PLANET COMMAND:
┌────────┬──────────────────────┬────┬────┬──────┬────┬─────┬──────┬──┬───┬───┐
│(X,Y)   │Name                  │Curr│ Max│Points│ Rev│ Grow│Docked│SB│ARs│GBs│
├────────┼──────────────────────┼────┼────┼──────┼────┼─────┼──────┼──┼───┼───┤
│(03,03) │Player 1 HW           │ 100│ 100│   165│  55│   +0│     0│ N│ 10│  4│
│(05,13) │Aster Vale            │  24│  75│    12│  13│   +4│     3│ N│  2│  1│
│(08,08) │New Carthage          │  61│  90│    40│  33│   +2│     6│ Y│  5│  2│
│(10,12) │Haven                 │  18│  60│     4│   9│   +5│     0│ N│  1│  0│
└────────┴──────────────────────┴────┴────┴──────┴────┴─────┴──────┴──┴───┴───┘
COMMANDS <ARROWS J K S Q> [03,03] ->
```

## Command Bar Grammar

### Browse mode

Standard browse bar:

```text
COMMANDS <HOTKEYS> ->
```

Coordinate-aware browse bar:

```text
COMMANDS <HOTKEYS> [03,03] ->
```

Rules:

- the default editable coordinate mirrors the currently selected row
- on first open, that means the top selected row
- the same identity-column rule applies to fleet and other ID-based tables
- pressing `ENTER` with an empty input opens the currently selected row’s
  detail view
- entering a valid coordinate opens that target’s detail view
- quitting from that detail view returns to the originating table

### Prompt-replacement mode

Sort and filter prompts replace the same bottom row instead of opening a
second prompt line.

Examples:

```text
COMMANDS <- Sort by <C>urrent Prod, <L>ocation, <M>ax, or <Q>uit? [C] ->
```

```text
COMMANDS <- Filter by <L>ocation, <R>ange, <E>mpire, <M>ax Prod, or <Q>uit? [L] ->
```

Rules:

- `S` is the standard sort hotkey for sortable tables
- `F` is the standard filter hotkey for filterable tables
- `Q` is quit/back
- sort prompts use the first letter of the column they actually sort
- hotkey letters inside both `<...>` and `[C]` / `[L]` render in yellow
- outer `< >` and `[ ]` stay in the neutral prompt color
- `ENTER` remains implicit and is not listed in the hotkey rail

## Total Planet Database

The total planet database follows the same border and command-bar standard.

It is not a special-case table style. The main difference is the data it
shows, not the chrome:

- the same one-row bordered header standard applies
- `(X,Y)` and `(00,00)` formatting still apply
- command-bar behavior follows the same `COMMANDS ... ->` grammar

## Split Tables

Split tables are allowed when two synchronized halves genuinely make the
screen clearer, such as the build-specify chooser.

Even there, the halves should still use the same border language:

- top border
- one header row
- joined divider row
- body rows
- bottom border

The split layout is the exception. The standard list format remains a single
bordered table.
