# Esterian Conquest Quick Start Player's Guide

_Inspired by Esterian Conquest (c) 1992 Bentley C. Griffith.
A fan-built resurrection -- not affiliated with the original._

_Quick-start reference based on
[`original/v1.5/ECQSTART.DOC`](../../original/v1.5/ECQSTART.DOC)._

The original `.DOC` file remains the preserved source artifact. This edition
keeps the substance of the quick-start guide, but presents it as a cleaner
reference.

## How To Use This In EC Rust

Start here if you want the shortest path to understanding the game loop. When
you are ready for the full command and mechanics reference, move on to
[ec_player.md](ec_player.md).
If you want the untouched source, read
[ECQSTART.DOC](../../original/v1.5/ECQSTART.DOC).

## Start Here

Read this first, then read the full
[ec_player manual](ec_player.md).

## Object of the Game

Your objective is simple: become Emperor. In practice that means building
enough strength to dominate the remaining empires or destroying every serious
rival that can still contest the throne.

## The Play

You begin with one productive planet and four fleets. Two of those fleets carry
an `ETAC` and a cruiser, which gives you immediate colonization capability. The
other two are single-destroyer fleets, which are useful for early scouting and
light defense.

Set a tax rate, usually somewhere around `65%` on your starting world, and use
that revenue to build ships, armies, ground batteries, and starbases. As you
expand, ease the tax burden on young colonies when you want them to grow their
current production faster.

## Game Time: The Round

Each round represents one year. During that year, you enter and revise orders.
At maintenance time, the game resolves every empire's moves together.

Read report dates as `Stardate WK/YYYY`. The first number is the week of the
year, not the month. In other words, `05/3012` means week 5 of the 3012 turn
year, and `52/3012` means a late-year event in that same turn.

That is because the engine resolves one annual turn on an internal 52-week
timeline. You still give orders once per year, but the game uses those hidden
weeks to decide when scouting reports, encounters, battles, and other dated
events happen inside the turn.

## What You Can Do in a Round

In a normal round you will manage planets, assign fleet missions, review
intelligence, send messages, and decide whether diplomacy or force is the right
tool for the current year. If you expect to miss turns, you can also enable
`AUTOPILOT` and let the game fall back to defensive behavior.

## The Forces at Your Command

You command planets, ships, starbases, armies, and ground batteries. The ships
break down into six classes: destroyers, cruisers, and battleships for combat;
scouts for reconnaissance; troop transports for delivering armies; and `ETAC`s
for colonizing unowned worlds. Starbases improve both defense and development.
Armies decide who owns a planet. Ground batteries punish fleets that come too
close.

## Strategy Hints for Beginning the Game

Open by securing information and territory. Capture or export a starmap early,
send `ETAC` fleets toward nearby unowned worlds, and do not chase short-term
tax revenue so hard that you stunt growth. Once neighbors start locating your
worlds, invest in defenses, then in combat fleets, then in starbases on the
planets that must survive.

When you begin attacking, choose the assault mode deliberately. `BLITZ` lands
armies fast and does less planetary damage, but it exposes those armies to more
risk. `INVADE` is slower and rougher on the target, but it gives you a better
chance to break through a defended world. `BOMBARD` is the denial option: use
it when damaging the planet matters more than taking it intact.

## Reference Charts

### Assets You Can Build

| Item | Build Cost | Size | Max Speed | AS | DS | Purpose |
| --- | ---: | :---: | :---: | ---: | ---: | --- |
| Destroyer | 5 | S | 6 | 1 | 1 | Combat / Defense |
| Cruiser | 15 | M | 5 | 3 | 3 | Combat / Defense |
| Battleship | 45 | L | 4 | 9 | 10 | Combat / Defense |
| Scout | 15 | S | 6 | 0 | 1 | Spy on Planet / Sector |
| Troop Transport | 5 | M | 5 | 0 | 1 | Land armies on a planet |
| ETAC | 20 | L | 3 | 0 | 2 | Colonize a raw planet |
| Ground Battery | 20 | L | n/a | 9 | 2 | Defend planet |
| Army | 2 | S | n/a | 1 | 1 | Defend planet surface |
| Starbase | 50 | L | 1 | 10 | 12 | Enhance / Defend |

### Fleet Missions by Mission Number

| No. | Mission | Requirements |
| ---: | --- | --- |
| 0 | None (hold position) | None. All ships can do this. |
| 1 | Move Fleet | None. All ships can do this. |
| 2 | Seek Home | None. All ships can do this. |
| 3 | Patrol a Sector | None. All ships can do this. |
| 4 | Guard a Starbase | Combat ship(s). |
| 5 | Guard/Blockade a World | Combat ship(s). |
| 6 | Bombard a World | Combat ship(s). |
| 7 | Invade a World | Combat ships and loaded troop transports. |
| 8 | Blitz a World | Loaded troop transports. |
| 9 | View a World | None. All ships can do this. |
| 10 | Scout a Sector | At least one scout ship. |
| 11 | Scout a Solar System | At least one scout ship. |
| 12 | Colonize a World | At least one ETAC. |
| 13 | Join another fleet | None. All ships can do this. |
| 14 | Rendezvous at Sector | None. All ships can do this. |
| 15 | Salvage | None. All ships can do this. |
