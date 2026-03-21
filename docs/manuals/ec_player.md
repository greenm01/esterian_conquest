# Esterian Conquest Player's Guide

_v1.6 player reference based on
[`original/v1.5/ECPLAYER.DOC`](/home/niltempus/dev/esterian_conquest/original/v1.5/ECPLAYER.DOC)._

The original `.DOC` file remains the preserved source artifact. This
edition is being rewritten into a cleaner v1.6 reference while preserving the
same underlying rules.

## How To Use This In v1.6

Treat this as the main gameplay and command reference. If the Rust client
presents a workflow differently, prefer the underlying rule described here
unless a Rust-specific doc says otherwise. For the untouched source, read
[ECPLAYER.DOC](/home/niltempus/dev/esterian_conquest/original/v1.5/ECPLAYER.DOC).

Version 1.5.

Fourth Edition, July 1992.

Copyright (c) 1990-1992 by Bentley C. Griffith. Player's Guide written by Joel B. Cohen, Ph.D.

Published by Griffith International, P.O. Box 530703, Birmingham, AL 35253.

## Program Features

Esterian Conquest is an asynchronous turn-based empire game. Players submit
orders during the turn year, and maintenance resolves the turn. You manage
planets, direct fleets, gather intelligence, negotiate with rivals, and try to
become Emperor before another claimant does. The game supports up to `25`
players, offers menu-driven order entry with expert shortcuts, and gives you
messaging, intelligence, and autopilot tools on top of the core military and
economic systems.

## Table of Contents

- Program Features
- Introduction
- Background
- The Game
- The Play
- Basic Features
- Reference
- Conclusions
- Appendix A. Quick Reference Sheet
- Appendix B. ROE (Rules of Engagement) Settings
- Appendix C. Esterian Conquest(tm) Menus
- Appendix D. Feedback / Suggestion Sheet
- Appendix E. Registration of Esterian Conquest(tm)

## Introduction

Read this manual as both an overview and a reference. The rules are not hard to
learn, but winning consistently requires planning, patience, and a feel for
timing.

**Note:** For those wishing only quick-start information, read the
[ec_qstart guide](/home/niltempus/dev/esterian_conquest/docs/manuals/ec_qstart.md).

## Background

The year is `3000`. The Esterians vanished after centuries of rule, and their
old empire splintered into rival one-planet kingdoms. Each kingdom wants the
imperial crown. You command one of them.

## The Game

### Number Of Players

The game supports `4`, `9`, `16`, or `25` players.

### Object Of The Game

Become Emperor by dominating the remaining empires or by eliminating every
serious rival.

### How To Achieve Your Goal

Expand through colonization and conquest. Break enemy fleets. Take or protect
the planets that support your war effort.

### The Playing Field (Galaxy)

The galaxy is a square sector grid. Grid size depends on player count:
`18 x 18` for `4` players, `27 x 27` for `9`, `36 x 36` for `16`, and
`45 x 45` for `25`. Each sector is either empty or contains one solar system
with one colonizable planet. Total solar systems = `5 x player count`.

Planets are your economic base. Their potential production ranges from `10` to
`150`.

## The Play

### Starting The Game

You begin with one planet at current production `100` and four fleets. Two
fleets carry an `ETAC` and a cruiser. Two are single-destroyer fleets.

Set taxes, build units, and expand. On younger colonies, reduce tax pressure if
you want faster growth.

### Game Time - The Round

Each round equals one year. Enter or revise orders during that year. At
maintenance time, the game resolves every empire's orders together. There is no
advantage to entering moves early as long as they are in before maintenance.

Read report dates as `Stardate WK/YYYY`. The first number is the week of the
year rather than the month. A report dated `12/3002` happened in week 12 of
the 3002 turn year.

This works because the engine does not treat a turn as a single instantaneous
year-end rollup. Instead, it resolves the year on an internal 52-week
timeline. You still submit yearly orders, but the maintenance pass uses those
hidden weeks to schedule scouting results, contacts, combat, and other dated
events within the turn.

### What You Can Do In A Round

In a normal round you manage planets, assign fleet missions, check the database,
handle diplomacy, and decide whether autopilot should cover any neglected
assets.

### Winning

You win by becoming the only power that still matters. Sometimes that means
destroying rival empires outright. Sometimes it means reducing them to
irrelevance while your empire controls the balance of power.

### Losing

You lose when your planets, fleets, and armies are all gone. If you lose every
planet but still have fleets, you are not dead yet, but time is against you:
those fleets can begin to defect if you fail to recover a world.

### Marginal Existence

Planets are the foundation of everything. Lose them and recovery becomes a race.
Keep them and you can rebuild almost any military setback. That is why
planetary defense matters so much.

## Basic Features

### The Forces At Your Command

You control five major force types: planets, ships, starbases, armies, and
ground batteries.

Planets generate the production that pays for fleets, defenses, and expansion.
Two values matter constantly. `Maximum Production` is the ceiling a planet can
reach at full efficiency. `Current Production` is what the planet can deliver
right now before taxes are applied.

Taxes convert that current production into spendable build points. If a planet
has current production `50` and your empire tax rate is `60%`, it contributes
`30` build points next round.

Ships operate in fleets. A fleet can be tiny or enormous, but it always moves
at the speed of its slowest member. Mix a fast destroyer group with one slow
`ETAC` and the whole fleet slows down to the `ETAC`'s speed.

There are 6 types of starships -- 3 combat types and 3 specialized types.  Smaller ships are faster as a rule. Below is a list of the different ship types, their size (S,M,L), maximum speed (sectors/round), cost to build (in production points), and suggested uses.

**Destroyer:** A small, fast (speed=6), inexpensive (5 points) combat ship.  It is best for attacks on unarmed ships where it can overtake them.  Its speed also gives it a chance to escape from more heavily armed fleets.

**Cruiser:** A medium size combat ship.  About 3 times more powerful than a destroyer, they costs more (15 points) and are a bit slower (speed=5).  The cruiser is a good compromise between speed and power.  It's powerful enough to hit what it can't outrun (destroyers) and fast enough to outrun what it cannot outgun (battleships).

**Battleship:** This large combat ship is the anchor of any serious star fleet.  Three times more powerful than a cruiser, it is expensive (45 points) and slow (speed=4).

**Scout:** A good spy ship.  It is small and fast (speed=6) with excellent sensors and stealth capabilities.  It is expensive (15 points), but useful for reconnaissance and difficult to detect by enemy fleets.

**Troop Transport:** This ship is required for capturing planets.  A medium size, intermediate speed ship (speed=5).  It is inexpensive (5 points) and used to move an army from one planet to another.  It holds one army and is not armed.  This ship will drop an army on a planet during an INVADE or BLITZ.

ETAC (Environmental Transformation And Colonization): A large, slow (speed=3), expensive (20 points) ship used to terraform and colonize uninhabited planets.

STARBASES are large space fortresses that play special roles. Within a solar system, starbases protect and enhance the planet.  A planet with a starbase orbiting can withstand tax burdens better.  The starbase also enables a planet to spend up to 5 times its current production level building units (if you have that many production points stored on the planet).  A starbase helps a planet increase its current production level faster if the planet is not yet producing at its maximum.

Outside the solar system, a starbase gives surveillance reports on any alien fleets it finds.  It cannot attack since it moves very slowly (1 sector per round), but it can defend itself.  A starbase has slightly more fire power than a battleship and can withstand more hits. Starbases cost 50 production points to build.

ARMIES have two duties: (1) defend your planets from enemy invasions and blitzes and (2) invade or blitz enemy planets.  The only way to capture a planet owned by another player is to land armies on its surface and defeat his armies with yours.  Armies cost 2 production points to build.

GROUND BATTERIES are land-based cannons that defend planets against hostile starships.  Even though they cost 20 production points, they offer more "firepower per dollar" than any starship.  Their firepower is roughly equal to that of a battleship at less than half the cost.  Their major disadvantage is that they cannot move and therefore are easier to hit.

### Combat

A fleet with one or more combat ships (destroyer, cruiser, or battleship) can engage in battle.  You specify the aggressiveness of your fleet by assigning its ROE (Rules of Engagement).  An ROE of zero means the fleet will avoid everything at all cost.  An ROE of ten means the fleet will attack anything, even if it is greatly outnumbered. Appendix B lists all of the ROE levels.  Usually, the stronger fleet wins, but sometimes a weaker fleet may do more damage to the stronger fleet than it takes.

Your fleets may attack if they encounter the fleet of a player you've declared to be an enemy.  However, your fleets will always fight back if attacked.  Your fleets will also attack if another player's fleets enter one of your solar systems, or if they try to enter or leave a planet that your fleet is blockading.

### The Possible Fleet Missions

You can order a fleet on one of 15 missions.  Some missions require certain types of vessels.  After each mission name, the required vessels follow in parentheses.

**NONE (All ship types):** Hold position and do nothing.

**MOVE FLEET (All ship types):** Move to the specified sector.

**SEEK HOME (All ship types):** Move to the nearest planet you own.  Should that planet be taken over by an enemy, the fleet will move to the next closest planet you own.

**PATROL A SECTOR (All ship types):** Move around a sector (but not within a solar system even if the sector has a one) and try to intercept any ENEMIES traveling through.  If your fleet sees anything, it sends a report to you.

**GUARD A STARBASE (combat ships):** Move to and escort a specified starbase.  Help the starbase in a fight.  This is useful if the starbase is guarding a planet so that the combat ships work in coordination with the base.  A fleet guarding a starbase may leave it if they have a high ROE and an enemy fleet comes close to their solar system.

**GUARD/BLOCKADE A WORLD (combat ships):** Prevent alien contact with the planet you guard, including its owners if the planet does not belong to you.  This mission is primarily used to guard your planets from attack by other fleets. However, blockading a planet that belongs to another player is a way to intercept that player's ships which are launching from or approaching his/her planet.

**BOMBARD A WORLD (combat ships):** Pound the world, destroy its production and anything orbiting the world, including recently built ships stored in stardock.

**INVADE A WORLD (combat ships and troop transports with armies):** This is a three stage battle: (1) destroy all the world's ground batteries (cannons), (2) pound the population centers a little to soften resistance and take out enemy armies, then (3) send in troop transports to drop off your armies, but ONLY AFTER all ground batteries have been destroyed.  This damages the planet and gives them time to sabotage their industry before you take over, but gives your armies a better chance of taking the planet.  You succeed if you destroy all ground batteries and then your armies defeat their armies.

**BLITZ A WORLD (troop transports with armies -- combat ships recommended):** Try to infiltrate your armies onto the planet by dodging their ground batteries or distracting them.  You succeed if your armies defeat their armies. Since this form of attack is so quick, there is less damage to the planet since the enemy does not have time to sabotage their factories and since your combat ships do not pound the surface very much.  However, you put your armies at greater risk so you need many (twice as many as the enemy or better) to insure success.

**VIEW A WORLD (all ship types):** Go to the edge of a solar system and do a long-range scan of the planet to find out its potential production and who owns it.  Then back off into the deep space area of the sector.

**SCOUT A SECTOR (requires at least one scout ship):** Go into deep space of a sector (and not inside any solar systems) and look for alien fleets.  It will not engage enemy fleets.  This is less effective than the PATROL SECTOR order, but the scout ship is less likely to be noticed due to its stealth, especially if you use a fleet made of only one scout ship.

**SCOUT A SOLAR SYSTEM (requires at least one scout ship):** Use the scout's stealth equipment to get close to a world and: (1) count ground batteries and armies, (2) Estimate planet's current production and stardocks for currently built ships, and (3) Scan the solar system for fleets.  A fleet made of only one scout ship is least likely to be detected because of the scout's stealth capability.

**COLONIZE A WORLD (requires at least one ETAC):** Terraform and colonize a raw (unowned) planet.  If the world is already owned when you get there, report its owner and potential production and then leave the solar system to await new orders.

**JOIN ANOTHER FLEET (all ship types):** Seek out the specified host fleet and merge with it when they meet.  If the host fleet is destroyed, all joining fleets will abandon their mission.

**RENDEZVOUS (all ship types):** Move to the specified sector and merge with any other fleets ordered to rendezvous there.  The fleet with the lowest ID Number becomes the host fleet.  This order is useful for assembling large fleets near enemy planets for later attack.

**SALVAGE (all ship types):** Go to the specified planet and scrap the ships there for approximately half the purchase price in production points.

The fleet missions are summarized and arranged by mission number in Appendix A of this Player's Guide.

### Some Strategy Hints For The Beginning Of The Game

- **Print Out A Starmap.** From the GENERAL MENU, the game will display the entire galaxy map on the screen.  You can use your modem program to capture the display and then print the map.  Your Information Database remembers all you learn about each planet and you have the option to print your map with planets' Maximum Production and Owners on them.  This greatly aids your ability to plan your actions and plot the actions of the other players.  Since your information database keeps learning more as the game progresses, you might want to print a new starmap somewhat regularly.

- **Build ETACs.** Everyone starts with only one planet.  Since there are 5 times as many planets in the galaxy as there are players, there are many uncolonized planets.  Early moves should focus on building the Colonizing ships (ETACs) which terraform uninhabited planets and build factories on the new planets for later production.  Your ETACs will also tell you if they discover that a planet has already been colonized by another empire.  The information database records that information on your map for later attacks.

SET THE TAX RATE FOR YOUR EMPIRE.  You build equipment by spending the tax revenue production points you get on your planets.  All your planets are assigned the same tax rate (to avoid jealousies in your unstable empire).  You can set a tax rate from 0 to 100%, but we don't recommended setting it higher than 65% as some of your planets' production may suffer.  If your planet has a starbase, it may endure a tax rate of 67% to 70%, but there is no guarantee.

There is a trade-off in setting your tax rate.  High taxes give you more points to spend on equipment.  However, newly colonized planets start with CURRENT PRODUCTION much lower than the planet's MAXIMUM PRODUCTION.  These current production levels grow much faster when taxes are low.  Thus, you have to choose between getting immediate revenue by setting taxes high, or greater long term production by keeping tax rates low.

BUILD PLANETARY DEFENSES (Ground Batteries, combat ships, Starbases and armies).  Spend a good number of production points on defending your planets.  After the first few rounds, your neighbors may discover your planets.  Later, they may send their fleets to attack you.

- **Build Combat Fleets.** Use some resources to build fleets containing combat ships.  These fleets can initially guard your planets and later conquer your opponents' planets.

- **Build Starbases.** Starbases give your planets the ability to spend production points beyond their current production levels, increases their defense and accelerates their growth rates.

- **Build Exploration Fleets.** Fleets made up of a single destroyer are fast and relatively inexpensive (5 points). Ordering these fleets to view neighboring planets will build up your Information Database and give you the data to pick which planets you want to take.

### Some Middle To Endgame Strategy Hints

- **Get Tough.** Now the game takes an unfriendly turn.  There are no raw planets available for colonization.  The only way to get more planets is to take them away from other players.  Naturally, they are looking to do the same to you.  Develop both offensive and defensive forces.  Some diplomacy can be very helpful in neutralizing threats, but beware of possible betrayals.

- **Develop Planetary Defenses.** Defending your planets becomes much more important now.  Keep a number of armies on your planet or your opponents will BLITZ and overrun your armies.  GROUND BATTERIES (cannons) will fire on invading ships.  The ground batteries also shoot at enemy TROOP TRANSPORTs when they try to land armies on your planet.

- **Build Offensive Fleets.** Use these fleets to take planets away from your opponents.  Remember to build many troop transport ships loaded with armies to capture planets.

If you are using the BLITZ attack, your armies should outnumber the enemy's armies on the planet (a 2 to 1 ratio is recommended).  See the section on capturing a planet for full instructions.

BUILD SCOUT SHIPS TO SPY ON PLANETS.  The scout ship observes enemy forces while hidden.  It sends a detailed report on the planet's strength so you can know in advance whether your invading fleet is strong enough to win against the enemy (This is better than finding out the hard way!).

## Reference

### The Program Menus

You issue commands by selecting options from menus.  To pick an option, type the option letter and press [ENTER]. A list of all menus and options appears in Appendix C.

**Options Common To All Menus:** Five options available to all menus are: QUIT; HELP; XPERT MODE ON/OFF; VIEW PARTIAL STARMAP; and INFORMATION DATABASE.

QUIT ("Q"): usually returns you to a previous menu.  From the main menu, the quit option leaves the game and returns you back to the BBS.

HELP ("H"): gives a brief description of what each of the options do for the current menu.

XPERT ON/OFF ("X"): Toggles expert mode on and off. Normally, menus present a list of options and then prompt you to pick the letter of an option.  Expert mode saves you time by skipping the option list and displaying only the prompt for a letter.  If you forget the options while in expert mode, typing "X" and [ENTER] will turn expert mode off and show you the full menus again.

VIEW PARTIAL STARMAP ("V"): Displays a 17 x 17 portion of the Galaxy map centered around your specified coordinates.  Information Database knowledge of planetary Ownership and Maximum Production are also shown on the map.  You can slide the map by pressing a number key plus [ENTER] as displayed on the screen. Pressing the "5" plus [ENTER] will toggle the display of maximum production.

INFORMATION DATABASE ("I"): will display whatever your fleets have discovered about any planet.

**Menu Structure:** Esterian Conquest has four central menus. These are: MAIN MENU, GENERAL COMMAND, PLANET COMMAND, and FLEET COMMAND.

**Main Menu:** The main menu transfers you to one of the other three central menus ("G" for GENERAL COMMAND; "P" for PLANET COMMAND; or "F" for FLEET COMMAND).  You can toggle the ANSI graphics mode using the "A" command.  ANSI graphics look better and can make information easier to read.

Two summaries are available of your empire's production, fleets, planets, and starbases: "D" gives a Detailed Summary and "B" gives Brief Listings.

The "T" (Total Database) command gives a comprehensive list of your Planetary Database.  It first asks you which planets you want to view (All, Enemy owned, Neutral Owned,

Specific Empire, Unowned, or Unexplored Worlds).  Then it offers options on how to sort the information (Location, Range from a specified coordinate, Empire, or Maximum Production).

**General Command:** This menu offers four types of functions:

1. Turn autopilot mode on or off ("A").  If you will be away or unable to submit orders, autopilot mode causes the computer to play your empire for you (mostly building your planetary defenses).  You can turn off autopilot when you return.

2. Offer summary information: list of empires ("O"), a profile of your empire ("P"), a review of undeleted fleet reports or messages from other players ("R") and the option to delete all messages or reports ("D").

3. Declare the other players to be neutral or enemies ("E").  If you declare a player to be an enemy, your fleets will automatically attack his/her fleets when they are encountered.

4. Go to the MESSAGE COMMAND CENTER ("C").

**Message Command:** Esterian Conquest has a message editor for communicating with other players within the game. To start a new message, type "N" (for Number of Empire).  Enter the number and begin your message. Finish your message by entering a blank line.  Send your message with the "S" command.

The editor lets you to change, insert, delete, or move lines.  It also allows you to send copies of your message to multiple players.

**Planet Command:** list planets you own ("P" for brief, "F" for Full); set the planetary tax rate for your empire ("T"); order planets to scorch their surface ("S") to prevent an opponent from taking your factories; and loads ("L") or unloads ("U") armies to or from troop transports.  Planet Command also has two important sub-menus: ("B") BUILD ON PLANET and ("C") COMMISSION SHIPS.  With the "A" (AUTO- COMMISSION) command, you commission all newly built ships as fleets for all planets.

**Build On Planet:** specify what to build with your revenue production points to be ready next round.  You work on one planet at a time.  Type "R" to review information about the current planet.  The "S" (Specify builds) option gives you summary of how many tax revenue points you can spend; a list of objects you can build (see appendix A) and their cost.  When you are done, choose the "N" (Next Planet) option to go to the next of your planet with points to spend, or choose "C" (Change planet) if you want to work with a specific planet.

**Commission A Fleet:** assigns a fleet number to any newly built ships and lifts them out of stardock.  Starbases are also commissioned.  You commission from one planet at a time and can select the NEXT PLANET option to work with the another planet that has vessels in stardock.

**Fleet Command:** gives you lists of fleets ("B" for brief, "F" for full list); review ("R") an individual fleet; calculate Estimated Time of Arrival (ETA) for a fleet to any sector ("E"); change their aggressiveness, ID, or speed ("C"); detach ("D") or transfer ("T") ships from a fleet; order a fleet on a mission ("O"); give a group of fleets a common mission ("G"); merge two fleets ("M"); or load ("L") / unload ("U") armies onto troop transports. Fleet command has a submenu called STARBASE COMMAND, which allows you to review or move a starbase.

**Starbase Command:** lets you see a list of starbases ("S"); review a particular starbase ("R"); or Move/Halt a Starbase ("M").

**Entering Multiple Commands:** As you become more familiar with the menus, you will know in advance what information needs to be entered.  Esterian Conquest accepts multiple commands in one line if they are separated by a space, semi-colon, or comma.  For example, from the main menu, entering: "P;B" + ENTER will move through the PLANET COMMAND menu and then down to the BUILD ON PLANET menu.

### Planning Your Moves Off-Line.

Planning your moves when not connected to your BBS can save on-line time.  This is especially important if you have limited time on your BBS.  The best tool available for your strategy is a printout of the galaxy map (available from the GENERAL COMMAND MENU).  It enables you to locate nearby stars and mark the planets which belong to your opponents.  To get the galaxy map, capture the map through your telecommunications program as it displays on the screen and print the captured map.

Obtain an empire summary from the GENERAL COMMAND MENU. Activate the capture mode option of your modem program and save the empire summary that appears on the screen.  It provides a thorough report of the status of your planets and fleets.  That way, you can get a complete summary in a few minutes and then log off to plan your moves.

### How To Colonize A Raw (Unoccupied) Planet.

**Requirements:** You need a fleet that contains at least one ETAC (Environmental Transformation And Colonization ship).

**Procedure:** From the Fleet Menu, order the fleet on mission number 12 (colonize a planet).  Specify the X and Y coordinates of the planet you want to colonize and the fleet will proceed.  ETACs can be used unlimited times to colonize planets since they get their raw materials from the planets themselves.  When the ETAC fleet reaches the planet, it will send you a report.  It will terraform an unowned planet.  If the planet is owned, ETACs will report the owner and the planet's potential production.

### How To Defend Your Planet From Attack.

There are many ways you can defend your planet: (1) build armies to prevent it from being invaded; (2) construct ground batteries (land cannons) to shoot at any invading ships or troop transports, (3) build fleets to guard and blockade other ships in space, (4) construct starbases to reinforce and defend the planet (if you have a fleet and a starbase around a planet, order the fleet to guard the starbase), and (5) eliminate your enemies before they get to you (the best defense is a good offense).

Each method has its own strengths and weaknesses.  Using fleets and starbases to guard your planet from space keeps enemy ships away and, if successful, prevents enemy ships from firing on the planet and damaging production.  Some enemy fleets (especially those with fast ships) might slip past your defenses and gain access to the planet.

Ground defenses (armies and ground batteries) protect the planet directly from attack.  However, they are your last lines of defense.  Enemy combat ships kill your armies and ground batteries when they fire on the planet.  When enemy armies land on your planet, they fight your armies.  The ownership of the planet goes to the victorious army.

Obviously, it is best to combine land-based and space defenses.  Be certain to build armies on your planet.  If an enemy uses the BLITZ attack, the enemy armies will dodge your combat defenses and land to oppose your armies.

When you know an enemy is approaching your planet with an unstoppable fleet, you can use the SCORCH EARTH option of the Planet Command Menu.  This causes your planet to destroy its factories and make them useless to your enemy.

### Building And Commissioning Fleets.

Use taxed revenue production points from your planets to build ships.  From the PLANET COMMAND menu, select the BUILD COMMAND menu, specify build orders, and spend the production points necessary (see Appendix A) for the type of ship.  All equipment takes one turn to build.

Newly built ships are stored in your planet's stardock, and are ready for use.  Activate them by COMMISSIONING them, which means you assign them to fleets.  From the PLANET COMMAND menu, you can commission all ships from all planets at once with the AUTO-COMMISSION option, or tailor your fleet commissions by selecting the COMMISSION COMMAND option.

**Important Note:** Troop transports are empty when you commission them.  You must load armies onto them from your planets.  Use the "L" option from either the PLANET COMMAND or FLEET COMMAND menus.  Troop transports must be loaded with armies in order to capture planets.  Build extra armies on your planets for loading onto troop transports.

Commissioned fleets are assigned a number.  Occasionally, you will want to add newly constructed ships to existing fleets.  If the merging fleets are in the same sector, issue the MERGE command from the FLEET COMMAND menu. Otherwise, order the newly formed fleet to join the other fleet.

When you commission fleets containing combat ships, you control how aggressive they are by assigning them a Rules Of Engagement value (ROE -- see Appendix B for the list of possible ROE values).  As the ROE gets higher, your fleet will be more aggressive and fight with enemy fleets of greater strength relative to your own.  With low

```text
            ROE scores, your fleets will attempt to flee more easily.
            Setting low ROE values does not guarantee safety for your
            fleet, since the enemy has an opportunity to fire on them
            while they flee.
```

Match the types of ships built with the mission you want your fleets to accomplish.  Simple missions might require one type of ship such as colonizing a planet (requires at least one ETAC), scouting a planet (requires at least one scout ship), or blockading a planet (requires combat ships with firepower).  Some missions require combinations of ships.  For example, invading a planet requires combat ships and troop transports loaded with armies.

Some players prefer to build general purpose fleets which can accomplish many types of missions.  An advantage of mixed fleets is that combat ships can protect non-combat ships.  In addition, a mixture of combat ship types (destroyers, cruisers, and battleships) do better in battle since they can employ tactics not available to fleets of one ship type.

Disadvantages of larger mixed fleets are that the speed of the fleet is limited to that of its slowest member.  Also, with fewer large fleets, you cannot cover as much ground as you can with many moderate sized fleets.  Many players seem to prefer having one or two huge fleets which act on the information of smaller scouting or defensive fleets.

### Spying On Your Opponents.

Once all the uninhabited planets have been colonized, you must capture inhabited planets by force.  Gathering intelligence is a necessity.  Any ship can view a planet and report on its owner and production potential.  Through this intelligence, you can map the empires of other players and decide which planets are worth capturing.

Scout ships give more detailed reports, including the planetary defenses (ground batteries and armies), contents of the planet's stardock, and information on any fleets or starbases which might be orbiting the planet.

### Conquering And Attacking An Enemy'S Planet.

There are three methods of assaulting a planet: the BLITZ mission, the INVADE mission, and the BOMBARD mission.  The blitz and invade missions attempt to capture the planet, while the bombard mission pummels planets in order to make them useless to the owners.  Once you conquer a planet, remember to guard it from recapture.  Planets need two turns before factories are fully converted and start producing tax revenue points for you.

**The Blitz:** (Requires Loaded troop transports in much greater number than the armies on the planet.  Combat ships are recommended for escort and cover fire).  Your fleets will attempt to distract the planet's ground batteries and deposit your armies.  Then your armies will battle the armies of the planet.  The winner earns ownership of the planet.  The planet will often not be very damaged because the speed of your attack prevents attempts at sabotage and because your combat ships do not fire very much on the planet.  For the blitz, you need an overwhelming number of armies compared to those of the planet (they have an advantage of familiarity with their terrain).

**The Invade:** (Requires Sufficient combat ships to disable the ground batteries and then enough loaded troop transports to defeat the remaining armies on the planet surface). Your combat ships fight the planet's ground batteries. After all the ground batteries are destroyed, you land the armies with your transport to fight the remaining surface armies.  The winner takes control.  Invaded planets take damage because of your bombardment of the planet and time in which the planet's forces can sabotage factories.  The advantage of invading is that your bombardments also reduce the number of armies on the surface and give your armies better odds of winning the planet.

THE BOMBARD attack: (requires sufficient combat ships to damage a planet's resources).  The goal of the bombard mission is to reduce a planet's production and make it unusable to its owner.  Use this mission if you do not have enough armies to capture a planet, or wish to cut down your enemy's production.

## Conclusions

This manual is a basic guide to get you started with some basic strategies to consider at the beginning of the game. As in chess, you can learn how to move the pieces in a short time, but learning to master the game may take a lifetime. After reading this guide, you are now familiar with the capabilities and possible actions of your forces.  The task of coordinating your forces remains up to you.  Very elaborate campaigns, bluffs, and betrayals are possible.

**A Final Note:** We want your feedback and suggestions.  This game is the result of consultations with many experienced gamers and sysops.  We are always looking for new ways to enrich the play or add a wrinkle to Esterian Conquest. Appendix D is a feedback sheet set up for this purpose.

NOW STOP READING AND PLAY!  THERE'S A GALAXY TO CONQUER.

## Appendix A.  Quick Reference Sheet

### Machinery You Can Build

```text
                            BUILD        MAX.
```

### Item               Cost   Size  Speed   Purpose

```text
         DESTROYER            5      S     6     Combat / Defense
         CRUISER             15      M     5     Combat / Defense
         BATTLESHIP          45      L     4     Combat / Defense
```

```text
         SCOUT               15      S     6     Spy on Planet/Sector
         TROOP TRANSPORT      5      M     5     Land armies on Planet
         ETAC                20      L     3     Colonize a Raw Planet
```

```text
         GROUND BATTERY      20      L    n/a    Defend Planet
         ARMY                 2      S    n/a    Defend Planet Surface
```

### Starbase            50      L     1     Enhance / Defend

### Possible Missions For Fleets Listed By Their Mission Numbers

### No.  Mission                    Requirements (If Any)

```text
           0   None (hold position)       None. All ships can do this.
           1   Move Fleet (only)          None. All ships can do this.
           2   Seek Home                  None. All ships can do this.
           3   Patrol a Sector            None. All ships can do this.
           4   Guard a Starbase           Combat ship(s).
           5   Guard/Blockade a World     Combat ship(s).
           6   Bombard a World            Combat ship(s).
           7   Invade a World             Combat ship(s) & Loaded
                                            Troop Transports.
           8   Blitz a World              Loaded Troop Transports.
           9   View a World               None. All ships can do this.
          10   Scout a Sector             At least one scout ship.
          11   Scout a Solar System       At least one scout ship.
          12   Colonize a World           At least one ETAC.
          13   Join another fleet         None. All ships can do this.
          14   Rendezvous at Sector       None. All ships can do this.
```

### 15   Salvage                    None. All Ships Can Do This.

## Appendix B.  ROE (Rules Of Engagement) Settings

```text
            You only assign ROE for Fleets that contain Combat ships.
         Any fleet with only non-combat ships automatically gets an
         ROE of zero to avoid being destroyed by armed enemy fleets.
```

### Possible ROE (Rules Of Engagement)

### ROE   Conditions To Engage Hostile Fleets In Battle

```text
           0    Avoid all hostile fleets.  (Non-combat Fleets)
           1    Engage fleets only if they are defenseless.
           2    Engage fleets only if your advantage is 4:1 or better.
           3    Engage fleets only if your advantage is 3:1 or better.
           4    Engage fleets only if your advantage is 2:1 or better.
           5    Engage fleets only if your advantage is 3:2 or better.
           6    Engage hostile fleets of equal or inferior strength.
           7    Engage hostile fleets even if outgunned 3:2.
           8    Engage hostile fleets even if outgunned 2:1.
           9    Engage hostile fleets even if outgunned 3:1.
```

### 10    Engage Hostile Fleets Regardless Of Their Size.

## Appendix C. Esterian Conquest(tm) Menus

### Options Available To All Menus

```text
           H>elp with commands         V>iew Partial Starmap
           Q>uit to previous Menu      I>nfo about a Planet
```

### X>Pert Mode On/Off

### Main Menu Commands

```text
           A>nsi color ON/OFF          T>otal Planet Database
           G>ENERAL COMMAND MENU...    I>nfo about a Planet
           P>LANET COMMAND MENU...     B>rief Empire Report
```

### F>Leet Command Menu...      D>Etailed Empire Report

### General Command Center

```text
           A>utopilot ON/OFF            R>eview messages/results
           S>tatus, your                D>elete ALL messages/results
           P>rofile of your empire      O>ther empires (rankings)
           M>ap of the galaxy           E>nemies, declare or list
```

### C>Ommunicate (Send Message)

### Message Command Center

```text
           L>ist message                    I>nsert a line
           C>ontinue message                D>elete a line
           S>end (transmit) message         M>ove a line
           N>ew addressee for message       E>dit a line
```

### R>Emove Message From Memory

### Planet Command Center

```text
           C>OMMISSION MENU            T>ax rate: Empire
           A>UTO-COMMISSION            S>corch planets
           B>UILD MENU...              L>oad TTs w/Armies
           D>etail Planet List         U>nload TT Armies
```

### P>Lanet: Brief List

```text
         (Continued)
```

```text
              APPENDIX C. ESTERIAN CONQUEST(tm) MENUS  (Continued)
```

### Build On Current Planet: "Planet Name" In System (15,32)

```text
           P>lanets, List your         S>pecify Build Orders
           R>eview current planet      A>bort planet's builds
           C>hange current planet      L>ist builds
```

### N>Ext Planet

### Commission From Planet: "Planet Name" In System (15,32)

```text
           P>lanets, List your         B>ases, commission
           R>eview current planet      F>leet, commission
           C>hange current planet      L>ist stardock contents
```

### N>Ext Planet                I>Nfo About A Planet

### Planet Command Center

```text
           S>TARBASE MENU...           D>etach Ships
           B>rief Fleet List           T>ransfer Ships
           F>ull Fleet List            O>rder a Fleet
           R>eview a Fleet             G>ROUP FLEET ORDER
           E>TA Calculation            M>erge a Fleet
           C>hange ROE,ID,Speed        L>oad TTs w/Armies
```

### U>Nload Tt Armies

### Starbase Command

```text
           S>tarbases-List
           R>eview a Starbase
```

### M>Ove/Halt Starbase

## Appendix D.  Feedback/Suggestion Sheet

```text
         Please Mail Suggestions or Comments to:
```

```text
         ATTN: ESTERIAN CONQUEST(tm)
         Griffith International
         P.O. Box 530703
         Birmingham, AL 35253
```

```text
         YOUR NAME: _________________________________________________
```

```text
         STREET...: _________________________________________________
```

```text
         CITY, STATE  ZIP: __________________________________________
```

```text
         FEEDBACK/SUGGESTIONS ON ESTERIAN CONQUEST
```

```text
         VERSION NUMBER (Appears when you log in): __________________
```

```text
         BBS YOU PLAY ON: ___________________________________________
```

```text
         BBS PHONE NUMBER: (______) ______ - ____________
```

```text
         COMMENTS:
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

```text
         ____________________________________________________________
```

## Appendix E. Registration Of Esterian Conquest(tm)

```text
         You can register your version of ESTERIAN CONQUEST(tm) for
         your BBS.  Currently ESTERIAN CONQUEST(tm) runs on any IBM
         PC,XT,AT,PS/2 or compatible having the following:
```

```text
            MS-DOS version 2.11 or later.
```

```text
            512K of internal memory (640K recommended).
```

```text
            A hard disk with at least 1 MB free disk space.
```

```text
            BBS software that can generate any of the following door
            file formats:
```

```text
               PCBOARD.SYS  . . . . (PCBoard v14+)
               DOOR.SYS . . . . . . (as suggested by GAP software)
               DORINFOx.DEF . . . . (QBBS, RBBS, RA, FoReM, etc.)
               CALLINFO.BBS . . . . (Wildcat!)
               SFDOORS.DAT. . . . . (Spitfire)
               CHAIN.TXT  . . . . . (WWIV)
               INFO.BBS . . . . . . (Phoenix software)
```

### Advantages Of Registering Your Esterian Conquest(tm) Program

```text
            A campaign will run for unlimited time (the Shareware
            version stops each game after 30 rounds).
```

```text
            Discounts for minor revisions and upgrades:
```

```text
               All minor revisions are free.
```

```text
               Upgrades (major revisions) are available for a
               substantial discount.
```

```text
            Your name and BBS name will be encrypted into the program
            and will appear to all who use the game.  Registration
            messages appearing at the beginning of the game are
            removed.
```

```text
            A professionally printed manual containing both the
            SYSOP's guide and the PLAYER's guide.
```

```text
            BBS support for questions.
```

```text
            VOICE support for bug information.  We correct program
            bugs and send you the corrected version absolutely free.
```

```text
    Make checks or money                 Send your order to:
    orders payable in                      ATTN: ESTERIAN CONQUEST(tm)
    U.S. Currency to:                      Griffith International
      GRIFFITH INTERNATIONAL               P.O. Box 530703
                                           Birmingham, AL 35253
```

```text
    PAYMENT INFORMATION:
       Description                                    Each      Total
       --------------------------------------------   ------    -----------
```

```text
       ESTERIAN CONQUEST(tm) DOOR GAME             US $35.00    US $35.00
       Check One:  [] 5.25" Disk    [] 3.5" Disk
```

```text
       SHIPPING AND HANDLING:        Inside U.S.   US  $4.00    ___________
       (Required on all orders)
                                     Canada        US  $5.00    ___________
```

```text
                         Outside U.S. and Canada   US $10.00    ___________
```

```text
                                           TOTAL ENCLOSED:   US $__________
```

```text
    ENTER NAMES AS YOU WANT THEM TO APPEAR ON THE GAME SCREEN:
```

```text
       Sysop Name(s): ____________________________________________________
```

```text
       BBS Name: _________________________________________________________
```

```text
    SHIP TO:
       Name: _____________________________________________________________
```

```text
       Address: __________________________________________________________
```

```text
                __________________________________________________________
```

```text
    PHONE:
       Voice: (______)_________________    BBS: (______)__________________
```

```text
    TYPE OF BBS SOFTWARE: ________________________   VERSION: ____________
```

```text
    HOW DID YOU HEAR OF THE GAME? (Check Main Sources):
       [] Download from Support BBS (Salt Air, WildCat, etc.): ___________
       [] Download from a local BBS
       [] User(s) Requested/Uploaded Game to BBS
       [] Played on another BBS
       [] ECHO Conference(s): ____________________________________________
       [] Sysop or friend told you about Esterian Conquest
       [] Other: _________________________________________________________
```

```text
    DID YOUR USERS PAY FOR THIS REGISTRATION?:   [] All   [] Part   [] None
```

```text
    __________________________________                            +-------+
                                                                  | Place |
    __________________________________                            | Stamp |
                                                                  | Here  |
    __________________________________                            +-------+
```

```text
                             ATTN: ESTERIAN CONQUEST(tm)
                             Griffith International
                             P.O. Box 530703
                             Birmingham, AL 35253
```

```text
    Place Feedback Form or Order Form inside this page.  Fold into thirds.
    Insert any checks or money orders.  Tape on bottom and sides.
```
