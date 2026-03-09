# Fixture Workflow

Fixtures are the main bridge between reverse engineering and implementation.

They let us:

- preserve known-good states
- diff specific transitions
- turn observed behavior into tests

## Existing Fixture Families

- `original/v1.5/`
  - shipped sample state and bundled assets
- `fixtures/ecutil-init/v1.5/`
  - known-good initialized game state produced by `ECUTIL`
- `fixtures/ecmaint-post/v1.5/`
  - known-good post-maintenance state produced by `ECMAINT`
- `fixtures/ecutil-f3-owner/v1.5/`
  - targeted ownership-change snapshot for `ECUTIL F3`

## Rules For New Fixtures

Only create a new fixture set when it captures a meaningful state transition.

Good examples:

- one `ECUTIL` option change
- one ownership change
- one maintenance run after one known order
- one battle-producing scenario

Avoid:

- redundant copies with no documented purpose
- mixed scenario states where too many variables changed at once

## Recommended Workflow

1. Start from a known baseline

- usually `fixtures/ecutil-init/v1.5/` or `fixtures/ecmaint-post/v1.5/`

2. Make exactly one controlled change

- one menu option
- one fleet order
- one build order
- one ownership edit

3. Save and preserve the result

- copy the changed files into a clearly named fixture directory
- note exactly what was changed and why the fixture exists

4. Diff against the baseline

- identify changed files
- identify changed offsets or records
- turn confirmed fields into Rust accessors/setters/tests

5. Add tests

- parser tests for new fields
- CLI/TUI tests when behavior is exposed there

## Naming Guidance

Use descriptive fixture names tied to one observed operation.

Examples:

- `fixtures/ecutil-f4-snoop-off/v1.5/`
- `fixtures/ecutil-f5-flow-off/v1.5/`
- `fixtures/ecmaint-bombard-scenario/v1.5/`

## Key Idea

Fixtures are not just test data.

They are the durable record of observed original behavior, and they should be treated as first-class preservation assets.
