# Esterian Conquest History of Revisions

_Readable Markdown transcription of [`original/v1.5/WHATSNEW.DOC`](/home/mag/dev/esterian_conquest/original/v1.5/WHATSNEW.DOC)._

The original `.DOC` file remains the preserved source artifact. This
transcription is provided for easier reading, quoting, and linking.

Edited: `07/29/1992`

Current EC Version in the original file: `1.50`

The original revision notes describe Esterian Conquest as a multi-node
galactic battle game supporting seven major BBS door-file formats and tested
on setups using `NOVELL(tm)`, `Lantastic(tm)`, and `Desqview(tm)`.

## Original Summary for Version 1.50

The `1.50` notes highlight:

- two new fleet missions
- an intelligence database that remembers planet information
- faster order-entry features such as auto-commission and group orders
- enhanced fleet reports
- a registration key file for unlocking registered features in later `1.x`
  test-drive builds

The file also says `1.50` would be sent to registered users for a
`$4.00` shipping and handling fee.

## Version 1.50 (Released 07/29/92)

### User Interface (`ECGAME`)

1. Two new missions:
   - `RENDEZVOUS` — fleets move to a specified sector and merge there
   - `SALVAGE` — fleets move to a planet and are scrapped for about half
     their value in production points
2. Group orders for up to `500` fleets at once.
3. Intelligence database for reviewing planets you have viewed or scouted.
4. Auto-commission of fleets and starbases from all stardocks.
5. Fleet-view enhancements, including distance-based listing and better
   starting-value controls for ID and mission filtering.
6. Message and result enhancements for nonstop review and bulk deletion.
7. Better detach and transfer behavior that avoids aborting the original
   mission unless necessary.
8. Fleet speed can now be changed, in addition to ROE and ID.
9. Empire lists now show the previous year's planet count and production.

### Maintenance (`ECMAINT`)

1. Improved memory management.
2. New combat simulations, described in the original notes as more realistic
   for both fleet-vs-fleet and fleet-vs-planet combat.
3. `ECMAINT /C` now works to close files and force unclaimed empires to
   become rogues.

## Version 1.11 (Released 05/25/91)

### Known Bugs That Were Exterminated

1. Fleets occasionally refused to attack planets.
2. Joining fleets could rarely think their host had been destroyed when it
   had not.
3. Some planets with all armies destroyed could suddenly gain a huge number of
   armies.
4. Starbases could incorrectly keep listing fleets as escorts after those
   fleets left.
5. Planets were allowed to have blank names.
6. Snoop could not be turned off.

### New Features

1. Fleets can be listed by destination, sorted by ETA.
2. Brief fleet reports now show ROE and ETA.
3. The stardate now appears in the rankings file.
4. Sysops can assign IRQ numbers and hardware flow control to `COM1` through
   `COM4`.

### New `ECFIX` Utility

The original notes say `ECFIX` was added to restore game files if they became
corrupted. Messages and recent orders might still be lost, but restored games
were said to remain very close to the undamaged originals.

## Version 1.10 (Released 03/01/91)

1. Added support for these BBS packages and door-file formats:
   - `PCBOARD`
   - `GAP`
   - `RBBS`
   - `QBBS`
   - `RA`
   - `FoReM`
   - `WILDCAT`
   - `SPITFIRE`
   - `WWIV`
   - `PHOENIX`
   - plus any BBS that could generate one of:

```text
PCBOARD.SYS
DOOR.SYS
DORINFOx.DEF
SFDOORS.DAT
CALLINFO.BBS
INFO.BBS
CHAIN.TXT
```

2. `ECGAME` can be launched with either a specific door-file path or just a
   directory path and will search for a valid door file there.
3. More stable interrupt handling.
4. `Desqview(tm)` awareness.
5. A new first-time menu for users to browse a game before joining.

## Version 1.02 (Released 01/07/91)

1. Fixed a minor bug that could occasionally block users from entering the
   game at late hours.
2. User-side beep alarms no longer echoed on the sysop side.
3. Internal programming was streamlined and modularized, which the notes say
   made the files smaller and more efficient.

## Version 1.01 (Released 01/02/91)

1. Corrected the P.O. box address in earlier documentation.
