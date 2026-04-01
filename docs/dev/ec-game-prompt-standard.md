# `nc-game` Prompt Standard

This document defines the standard command-line prompt grammar for the Rust
player TUI.

It exists to stop screen-by-screen prompt drift. Shared prompt rows, inline
entry flows, and command-line confirmations should follow this spec unless a
screen has a narrow, explicit exception.

Read it together with:

- [bbs_door_client_rust.md](bbs_door_client_rust.md)
- [tui_style_guide.md](tui_style_guide.md)
- [nc-game-table-standard.md](nc-game-table-standard.md)

## Core Rules

- The player TUI targets a fixed `80x25` playfield.
- Live command-line prompts leave a space after `-> ` before the cursor.
- Command rails use the universal frame `LABEL <- ... ->`.
- For non-table, non-main-menu command-line prompts:
  - angle brackets `<...>` mean available command key(s)
  - square brackets `[...]` mean the default value only
- The only allowed exception is classic yes/no shorthand:
  - `Y/[N]`
  - `[Y]/N`
- Primary command-center menu rails keep their menu identity:
  - `MAIN COMMAND`
  - `GENERAL COMMAND`
  - `FLEET COMMAND`
  - `STARBASE COMMAND`
  - `PLANET COMMAND`
  - `BUILD COMMAND`
  - `FIRST TIME COMMAND`
- Outside those primary command-center menu rails, the default live prompt label
  is `COMMAND`.
- Explicit startup/naming prompts may keep their classic labels such as:
  - `EMPIRE NAME`
  - `HOMEWORLD`
  - `WORLD NAME`
- On help-capable screens, `?` is the visible popup-help token.
- On classic primary menus, `H` is an implicit alias for the same popup help
  even when it is not repeated in the visible rail.
- Command rails never list `ENTER`.

## Standard Grammar

### Freeform input with a default

```text
LABEL <- Prompt text [default] <Q> ->
```

Examples:

```text
COMMAND <- Review Fleet # [2] <Q> ->
COMMAND <- Empire tax rate (0 - 100) [65] <Q> ->
COMMAND <- Planet coords [16,13] <Q> ->
```

### Freeform input with command choices and a default

```text
LABEL <- Prompt text <R>, <I>, <S> [R] <Q> ->
```

Examples:

```text
COMMAND <- Change <R>OE, <I>D, or <S>peed [R] <Q> ->
Sort by <C>urrent Prod, <L>ocation, <M>ax, or <Q>uit? [C] ->
Filter by <L>ocation, <R>ange, <E>mpire, <M>ax Prod, or <Q>uit? [L] ->
```

### Freeform input with command choices and no default

```text
LABEL <- Prompt text <BB,CA,DD,TT*,TT,SC,ET,C,X> <Q> ->
```

Examples:

```text
COMMAND <- Class <BB,CA,DD,TT*,TT,SC,ET,C,X> <Q> ->
COMMAND <- Class <BB,CA,DD,TT*,TT,SC,ET,C,X> <Q> ->
```

The repeated fleet examples are intentional: detach and transfer should follow
the same staged class-entry grammar.

### Yes / no confirmations

Keep the classic EC shorthand:

```text
LABEL <- Y/[N] ->
LABEL <- [Y]/N ->
```

Examples:

```text
SEND MESSAGE <- Y/[N] ->
GENERAL COMMAND <- Y/[N] ->
HOMEWORLD <- "Aurora Prime" <- Is this correct? Y/[N] ->
WORLD NAME <- "New Terra" <- Is this correct? [Y]/N ->
```

Do not rewrite these to `<Y>, <N> [N]`.

## Theming Rules

- Prompt row background uses the semantic `prompt` style.
- Prompt label uses the semantic `title` style.
- Angle punctuation `<` and `>` use `prompt_angle_delimiter`.
- Square punctuation `[` and `]` use `prompt_square_delimiter`.
- Inner hotkey/default text inside those delimiters uses `prompt_hotkey`.
- Live typed input also uses `prompt_hotkey`.
- The special phrase `slap a key` keeps its current accent behavior through
  `prompt_notice_action` for the prose and `prompt_hotkey` for `key`.

Implementation consequence:

- any shared renderer that accepts prompt text must parse prompt markup instead
  of writing the prompt body as plain prompt-colored text
- custom prompt renderers must match the same style split if they do not use
  the shared helper

## Normalization Rules

- Use `<Q>` consistently in command-line prompts when quit/cancel is available.
- In command rails, `Q` is always rendered as `<Q>`.
- In command rails, other tokens stay bare and use space separation.
- In command rails, `[default]` stays rightmost and `<Q>` sits immediately to
  its left when both are present.
- In command rails, `ENTER` remains implicit and should not be repeated in the
  rail or nearby subtitle text.
- When a prompt is rendered through `draw_command_line_default_input_at(...)`,
  do not include `Q` in the prompt-body command list; that helper already
  appends the canonical trailing `<Q> ->`.
- Do not use square brackets for normal command choices such as:
  - `[R]OE`
  - `[D]elete`
  - `[A]ll`
  unless the prompt is a yes/no confirmation exception.
- Keep prompt wording short and input-oriented.
- Do not expand simple prompts into wordy forms like `<Y>es, <N>o` unless a
  specific screen genuinely needs the extra text for clarity.
- If a prompt already has a command-center label, do not repeat that title in
  the prompt body.

## Representative Non-Table Prompts

Fleet:

```text
COMMAND <- Review Fleet # [2] <Q> ->
COMMAND <- Order Fleet # [2] <Q> ->
COMMAND <- Change <R>OE, <I>D, or <S>peed [R] <Q> ->
COMMAND <- New ROE [0] <Q> ->
COMMAND <- New Fleet ID [4] <Q> ->
COMMAND <- New Speed [5] <Q> ->
COMMAND <- ETA Fleet # [2] <Q> ->
COMMAND <- Merge Fleet # [4] <Q> ->
COMMAND <- Into Fleet # [2] <Q> ->
COMMAND <- Transfer From Fleet # [4] <Q> ->
COMMAND <- Transfer To Fleet # [2] <Q> ->
COMMAND <- Load Fleet # [3] <Q> ->
COMMAND <- How many armies to load? [2] <Q> ->
COMMAND <- Unload Fleet # [2] <Q> ->
COMMAND <- How many armies to unload? [2] <Q> ->
COMMAND <- Class <BB,CA,DD,TT*,TT,SC,ET,C,X> <Q> ->
```

Startup / naming:

```text
EMPIRE NAME <- Name your empire (20 chars or less) <Q> ->
EMPIRE NAME <- "Aurora" <- Is this correct? [Y]/N ->
HOMEWORLD <- Name this world (20 chars or less) <Q> ->
HOMEWORLD <- "Aurora Prime" <- Is this correct? Y/[N] ->
WORLD NAME <- Name this world (20 chars or less) <Q> ->
WORLD NAME <- "New Terra" <- Is this correct? [Y]/N ->
```

Messaging and planet-side prompts:

```text
SEND MESSAGE <- Y/[N] ->
GENERAL COMMAND <- Y/[N] ->
COMMAND <- Planet coords [16,13] <Q> ->
COMMAND <- Empire tax rate (0 - 100) [65] <Q> ->
COMMAND <- Qty for Destroyers [00] <Q> ->
COMMAND <- How many new destroyers to build [1] <Q> ->
COMMAND <- <ENTER> commissions the drafted fleet. <Q> ->
COMMAND <- <ENTER> commissions the highlighted starbase. <Q> ->
COMMAND <- Delete how many Destroyers? <A>ll or 1-2 <Q> ->
```

## Command Rail Examples

```text
MAIN COMMAND <- ? X V C G P F T I B D <Q> ->
GENERAL COMMAND <- ? X V I A S P M C R D O E <Q> ->
MAP COMMAND <- ? HJKL 1 2 3 4 6 7 8 9 <Q> ->
COMMANDS <- J K ^U ^D <Q> [03,03] ->
```

This document does not change the prompt-replacement table rows described in
[nc-game-table-standard.md](nc-game-table-standard.md). Those still use normal
prompt markup such as `COMMANDS <- Sort by <C>urrent Prod ... [C] ->`.
