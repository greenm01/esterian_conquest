## Accomplished
- Completed the reverse engineering of the token gate mechanism and the `16A4` integrity bypass flag.
- Exhaustively proved that `DS:16A4` is never set to 1 due to a likely developer typo (command line `/B` sets `16A2`, but the integrity check tests `16A4`).
- Discovered the true reason `.TOK` files "bypass" the crash: the presence of `Move.Tok` triggers an automatic restore of `.SAV` backups over the `.DAT` files prior to the integrity check, causing the repaired files to pass naturally.
- Documented findings in `token-investigation.md`.

## Next Steps
- Mission complete regarding the token bypass investigation. The user can dictate the next feature to reverse or modify.
