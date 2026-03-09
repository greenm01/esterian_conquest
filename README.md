# esterian_conquest

Preservation and reverse-engineering workspace for Esterian Conquest v1.5.

Current focus:
- documenting the original DOS game behavior and file formats
- preserving confirmed reverse-engineering findings
- building a Rust compatibility/preservation toolchain

Current top-level contents:
- `original/`: local snapshot of original EC 1.5 files used for preservation and testing
- `docs/`: stable project docs for approach, fixtures, and ECMAINT planning
- `RE_NOTES.md`: working reverse-engineering notes
- `rust/`: preservation-oriented Rust workspace
- `tools/`: unpacking and analysis helpers used during investigation

Docs:
- `docs/approach.md`: preservation and porting strategy
- `docs/fixtures.md`: fixture creation and usage workflow
- `docs/ecmaint-plan.md`: current plan for reverse engineering the maintenance engine
