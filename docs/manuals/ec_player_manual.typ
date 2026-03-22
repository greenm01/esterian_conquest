// Esterian Conquest — Player Manual
// Typst source — generates US Letter PDF with proper table layout

#set document(
  title: "Esterian Conquest — Player Manual",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 3, day: 22),
)

#set page(
  paper: "us-letter",
  margin: (x: 1in, y: 1in),
)

#set text(
  font: "New Computer Modern",
  size: 11pt,
)

#show raw: set text(font: "0xProto Nerd Font Mono")

#set par(
  justify: true,
  leading: 0.65em,
)

#set heading(numbering: "1.")

// Bold header row styling for all tables
#show table.cell.where(y: 0): strong

// Admonition helper
#let admonition(kind, body) = {
  let icon = if kind == "NOTE" { "i" }
    else if kind == "WARNING" { "!" }
    else if kind == "IMPORTANT" { "!!" }
    else { "?" }
  block(
    width: 100%,
    inset: 10pt,
    stroke: 0.5pt + luma(160),
    radius: 3pt,
    fill: luma(245),
  )[
    #text(weight: "bold")[#kind:] #body
  ]
}

// ─── Title Page ───────────────────────────────────────────────────────────

#align(center + horizon)[
  #block(width: 100%)[
    #set text(size: 8pt, font: "0xProto Nerd Font Mono")
    ```
  o     #######   ###### ########  #######  ######    ##    #####    ###  ##
    .  ##       ##         ##     ##       ##   ##   ##   ##   ##   #### ##
      ####      #####     ##     ####     ######    ##   #######   ## ####   .
     ##            ##    ##     ##       ## ##     ##   ##   ##   ##  ###
 .  #######  ######     ##     #######  ##   ##   ##   ##   ##   ##   ##

   *   ######   #####    ###  ##   #####   ##   ##  #######   ###### ########
     ##       ##   ##   #### ##  ##   ##  ##   ##  ##       ##         ##  .
  . ##       ##   ##   ## ####  ##   ##  ##   ##  ####      #####     ##
   ##       ##   ##   ##  ###  ## # ##  ##   ##  ##            ##    ##      .
   ######   #####    ##   ##   ######   #####   #######  ######     ##
    ```
  ]

  #v(2em)
  #text(size: 24pt, weight: "bold")[Esterian Conquest]
  #linebreak()
  #text(size: 16pt)[Player Manual]
  #v(1em)
  #text(size: 10pt, style: "italic")[
    Inspired by Bentley C. Griffith (1992). Rust port by Mason A. Green (2026)
  ]
]

#pagebreak()

// ─── Table of Contents ───────────────────────────────────────────────────

#outline(
  title: "Contents",
  indent: 1.5em,
)

#pagebreak()

// ─── 1. Introduction ────────────────────────────────────────────────────

= Introduction

Beyond the mapped frontiers of the old Esterian dominion lies a small galaxy of contested solar systems. The old masters are gone. Their stations are silent, their patrols vanished, and their subjects left with fleets, factories, and enough knowledge to build empires.

You rise as one of the new Star Masters. From a single world and a few small fleets, you must tax, build, scout, bargain, threaten, and strike before rival powers can do the same. Some systems will join your banner willingly. Others will require persuasion from orbit.

Each maintenance marks the passage of a year. In that span, fleets cross the dark between stars, colonies grow or starve, alliances turn cold, and wars are decided by distance, industry, mathematics, and will.

In profound respect and admiration to Bentley C. Griffith and his fellow pioneers, who between 1990 and 1992 forged the enduring legend of Esterian Conquest, and to the ancient dreamers, strategists, and storytellers whose timeless visions of galactic dominion still light the way among these stars.

Rust port by Mason A. Green, 2026.

#pagebreak()

// ─── 2. Quick Start ─────────────────────────────────────────────────────

= Quick Start

Your objective is simple: become Emperor by dominating rivals or eliminating every serious threat.

You begin with one planet at 100 production and four fleets --- two carrying an ETAC and cruiser for colonization, and two single-destroyer scouts. Set a tax rate around 50--65% on your homeworld, and use the revenue to build ships, armies, batteries, and starbases. Keep taxes low on new colonies so they develop quickly.

Each round represents one year. You submit orders during the year, and maintenance resolves all empires simultaneously on an internal 52-week timeline. Reports are dated as `Stardate WK/YYYY` (week/year).

=== Assets You Can Build

The table below lists everything your planets can produce. *AS* is Attack Strength (firepower) and *DS* is Defense Strength (hull/shields).

#figure(
  table(
    columns: (auto, auto, auto, auto, auto, auto, 1fr),
    align: (left, right, center, center, right, right, left),
    inset: 6pt,
    table.header(
      [Item], [Cost], [Size], [Speed], [AS], [DS], [Purpose],
    ),
    [Destroyer],       [05], [S], [6], [01], [01], [Combat, scouting, defense],
    [Cruiser],         [15], [M], [5], [03], [03], [Balanced combat],
    [Battleship],      [45], [L], [4], [09], [10], [Heavy combat],
    [Scout],           [15], [S], [6], [00], [01], [Reconnaissance],
    [Troop Transport], [05], [M], [5], [00], [01], [Deliver armies],
    [ETAC],            [20], [L], [3], [00], [02], [Colonize raw planets (reusable)],
    [Ground Battery],  [20], [L], [--], [09], [02], [Planetary defense],
    [Army],            [02], [S], [--], [01], [01], [Surface defense and capture],
    [Starbase],        [50], [L], [1], [10], [12], [Defense, production boost],
  ),
)

=== Fleet Missions (Summary)

Fleet missions fall into three categories. _One-shot_ missions cause the fleet to travel, perform an action, and then revert to Hold Position --- you must issue new orders afterward. _Persistent_ missions remain active until you replace them or game rules invalidate them. _Hostile_ missions send the fleet to a target world where it waits; the assault executes on the _next_ maintenance tick after arrival, not immediately.

#figure(
  table(
    columns: (auto, auto, auto, 1.2fr, 1.8fr),
    align: (right, left, left, left, left),
    inset: 6pt,
    table.header(
      [No], [Mission], [Type], [Requirements], [On Arrival / Completion],
    ),
    [00], [Hold position],         [Persistent], [Any ships],                   [Idle at current position],
    [01], [Move to sector],        [One-shot],   [Any],                         [Reverts to Hold on arrival],
    [02], [Seek Home],             [One-shot],   [Any],                         [Reverts to Hold; retargets if destination captured],
    [03], [Patrol],                [Persistent], [Any],                         [Patrols sector continuously],
    [04], [Guard starbase],        [Persistent], [Combat ships],                [Escorts starbase continuously],
    [05], [Guard/blockade world],  [Persistent], [Combat ships],                [Blockades world continuously],
    [06], [Bombard world],         [Hostile],    [Combat ships],                [Bombards when in orbit at tick start],
    [07], [Invade world],          [Hostile],    [Combat + loaded transports],  [Invades when in orbit at tick start],
    [08], [Blitz world],           [Hostile],    [Loaded transports (combat recommended)], [Blitzes when in orbit at tick start],
    [09], [View],                  [One-shot],   [Any],                         [Scans from system edge, reverts to Hold],
    [10], [Scout sector],          [One-shot],   [At least one scout],          [Reports and reverts to Hold],
    [11], [Scout system],          [One-shot],   [At least one scout],          [Reports and reverts to Hold],
    [12], [Colonize world],        [One-shot],   [At least one ETAC],           [Colonizes if unowned; ETAC survives for reuse],
    [13], [Join Fleet],            [Persistent], [Any],                         [Chases host until merge; abandons if host lost],
    [14], [Rendezvous],            [Persistent], [Any],                         [Waits at sector; lowest fleet ID becomes host],
    [15], [Salvage],               [One-shot],   [Any],                         [Scraps ships for \~50% value, reverts to Hold],
  ),
)

#pagebreak()

// ─── 3. Forces at Your Command ──────────────────────────────────────────

= Forces at Your Command

You control five major force types: planets, ships, starbases, armies, and ground batteries.

=== Planets

Planets are the foundation of your empire. Each one has a *Potential Production* --- the ceiling it can reach at full efficiency, ranging from 10 to 150 across the galaxy --- and a *Present Production*, which is what the planet can actually deliver right now. Present Production grows toward Potential over time, and the speed of that growth depends heavily on your tax rate.

Taxes convert Present Production into spendable build points each year. See @economy for the full tax rate, growth, and starbase economics model.

=== Ships

Ships operate in fleets. A fleet always moves at the speed of its slowest member and can hold anywhere from one to *3,000 ships* of mixed types.

#figure(
  table(
    columns: (auto, auto, auto, auto, auto, auto, 1fr),
    align: (left, right, center, center, right, right, left),
    inset: 6pt,
    table.header(
      [Ship Type], [Cost], [Speed], [Size], [AS], [DS], [Tactical Role],
    ),
    [*Destroyer*],  [05], [6], [S], [01], [01], [Fast, cheap screen. Escapes heavy fleets. Good for scouting.],
    [*Cruiser*],    [15], [5], [M], [03], [03], [Balanced fighter. About 3x the power of a destroyer.],
    [*Battleship*], [45], [4], [L], [09], [10], [Heavy firepower anchor. About 3x the power of a cruiser. Slow but durable.],
    [*Scout*],      [15], [6], [S], [00], [01], [Stealthy spy. A lone scout is hardest to detect.],
    [*Transport*],  [05], [5], [M], [00], [01], [Unarmed. Carries one army. Essential for conquest.],
    [*ETAC*],       [20], [3], [L], [00], [02], [Colony ship. Reusable --- survives colonization.],
  ),
)

=== Starbases

Starbases are large space fortresses (Cost: 50, AS: 10, DS: 12) that serve dual roles. In orbit around a planet, they provide a defensive combat bonus and significant economic benefits --- a planet with a starbase tolerates higher taxes without stalling growth and can spend up to *5x* its current production on a single build when points have been accumulated (see @economy for details). In deep space, they function as surveillance platforms with slightly more firepower than a battleship, though they move very slowly at just 1 sector per year.

Unlike ships, starbases are not assigned to fleets. They are commissioned individually from stardock and moved independently through the *Starbase Command* submenu. You can order combat fleets to escort a starbase using Mission 4 (Guard Starbase), but the starbase itself remains a separate unit.

#admonition("NOTE")[Starbases must be commissioned from stardock before they can be moved or provide orbital benefits. An uncommissioned starbase sitting in stardock has no effect.]

=== Ground Forces

*Armies* (Cost: 2, AS: 1, DS: 1) defend your planets from invasion and are the only way to capture enemy worlds. Each troop transport carries one army, and a successful invasion requires landing enough armies to overwhelm the defending garrison.

*Ground Batteries* (Cost: 20, AS: 9, DS: 2) are immobile land-based cannons that offer massive firepower per cost --- roughly equal to a battleship at less than half the price. During an invasion, all batteries must be destroyed before transports can land. During a blitz, surviving batteries fire directly on descending transports, inflicting heavy losses. If a planet is captured via blitz, any surviving batteries transfer intact to the new owner.

#pagebreak()

// ─── 4. Economy and Taxes ───────────────────────────────────────────────

= Economy and Taxes <economy>

Your empire runs on production. Every owned planet generates tax revenue each maintenance cycle, and that revenue pays for ships, defenses, and expansion. Managing the tension between short-term revenue and long-term growth is one of the deepest strategic challenges in the game.

=== Key Terms

Every planet has a *Potential Production* --- the maximum productive capacity it can ever reach --- and a *Present Production*, which is what it can deliver right now. Present Production grows toward Potential over time. Your empire's *Total Available Points* is the sum of tax revenue across all your planets, and any revenue you do not spend accumulates on each planet as *Stored Production Points*, available for future builds.

=== Tax Revenue

The tax rate is empire-wide: you set one rate for all your planets. Revenue per planet per year is calculated as:

#align(center)[
  `revenue = floor(present_production * tax_rate / 100)`
]

Your empire's Total Available Points is the sum of this across all owned planets.

=== Growth Toward Potential

Each maintenance turn, every owned planet grows its Present Production toward its Potential. Lower taxes produce faster growth --- a planet at 30% tax develops far faster than one at 60%. Growth also slows naturally as a planet approaches its ceiling. Even at punishingly high tax rates, a planet below its Potential always grows by at least 1 point per year.

=== The 65% Tax Threshold

#admonition("WARNING")[Setting taxes above *65%* can directly _reduce_ Present Production on your planets, not just slow growth.]

Below 65%, growth is always positive --- lower is faster. Above 65%, a penalty kicks in that actively damages Present Production. A planet with a commissioned starbase in orbit can tolerate rates up to approximately *70%*, but there is no hard guarantee. Pushing past that threshold still risks damage.

The recommended early-game rate is around 50--65%. Drop taxes on new colonies to accelerate their development.

=== Starbase Economic Effects

A commissioned starbase in orbit provides two major benefits. First, the planet's yearly production growth is boosted by *+50%* over the base rate, which means underdeveloped planets with starbases catch up significantly faster. Second, the planet gains a *build capacity multiplier* --- it can spend up to *5x* its Present Production on builds in a single turn, drawing from Stored Production Points. Without a starbase, a planet can only spend up to 1x its Present Production per turn.

#admonition("NOTE")[These bonuses require an active, commissioned starbase in orbit --- not an uncommissioned starbase sitting in stardock.]

=== Stored Production Points

Tax revenue that you do not spend accumulates as Stored Production Points on each planet. This reserve is what allows starbase worlds to execute large builds --- up to 5x Present Production --- in a single turn.

=== Newly Colonized Planets

A freshly colonized planet starts with Present Production far below its Potential. Growth depends heavily on the tax rate you set, so keep taxes low on new colonies to develop them quickly.

=== Conquered Planets

When you capture an enemy planet by invasion or blitz, the factories need approximately *two turns* before they are fully converted and begin producing tax revenue for your empire. Plan your logistics around this delay.

#pagebreak()

// ─── 5. Combat Mechanics ────────────────────────────────────────────────

= Combat Mechanics

While battle reports are simple summaries, the engine uses a sophisticated system behind the curtain. The combat model is deterministic and simultaneous rather than relying on opaque random number generation or arbitrary ship-vs-ship duels.

=== The Rules of Battle

There is no "first strike" advantage. Each round, the total *Attack Strength (AS)* of each fleet is calculated and inflicted upon the enemy at the same instant. Rounds repeat until one side is destroyed, disengages, or only one hostile force remains.

Damage reduces ships from "Nominal" to "Crippled" status before destroying them. All nominal ships must be crippled before any crippled ship is destroyed, and surviving crippled ships are repaired automatically after battle. Hits always target the combat line first --- destroyers, cruisers, battleships, and starbases. Scouts, transports, and ETACs are protected as long as any combat-line ships remain, which means your non-combat vessels survive as long as your warships hold the line.

Mixed fleets containing destroyers, cruisers, _and_ battleships receive a *combined arms bonus* that improves their tactical effectiveness compared to single-type fleets. Always mix your composition when possible. A defending starbase at its own world provides an additional combat bonus to the defender. In a draw, the defender wins.

=== Fleet Limits

A fleet can contain as few as one ship and as many as *3,000 ships* of mixed types. A fleet always moves at the speed of its slowest member.

=== Rules of Engagement (ROE)

You assign an ROE level (0--10) to control when your fleet voluntarily engages hostile forces. ROE is a deterministic commitment rule based on force ratios, not random chance.

#figure(
  table(
    columns: (auto, auto, 1fr),
    align: (right, left, left),
    inset: 6pt,
    table.header(
      [ROE], [Force Requirement], [Behavior],
    ),
    [00], [Never engage],            [*Pacifist*: Flee from all hostile fleets.],
    [01], [Enemy defenseless],       [*Opportunist*: Engage only if the enemy has no combat capability.],
    [02], [4:1 or better],           [*Very Cautious*: Engage only with overwhelming advantage.],
    [03], [3:1 or better],           [*Cautious*: Engage only with strong advantage.],
    [04], [2:1 or better],           [*Favorable*: Engage with clear superiority.],
    [05], [3:2 or better],           [*Confident*: Engage with moderate advantage.],
    [06], [1:1 or better],           [*Balanced*: Engage equal or inferior forces.],
    [07], [Even if outgunned 3:2],   [*Bold*: Accept moderate disadvantage.],
    [08], [Even if outgunned 2:1],   [*Aggressive*: Accept significant disadvantage.],
    [09], [Even if outgunned 3:1],   [*Reckless*: Accept severe disadvantage.],
    [10], [Always],                  [*Suicidal*: Attack regardless of the odds.],
  ),
)

Non-combat fleets (scouts, transports, ETACs only) are treated as ROE 0 automatically.

=== Withdrawal and Retreat

#admonition("IMPORTANT")[Low ROE does not guarantee safety.]

A fleet that refuses engagement or breaks off mid-battle does not escape cleanly. It suffers a *withdrawal exchange* --- the enemy fires on your retreating fleet, and your fleet fires back at reduced effectiveness. Only after absorbing that exchange does the fleet actually retreat and abort its current mission. After each round of combat, surviving fleets re-check their ROE. If the post-loss ratio no longer meets their threshold, they attempt to disengage and suffer the withdrawal exchange.

=== Planetary Combat

When fleets attack planets through bombardment, invasion, or blitz, different rules apply. Ground batteries are the primary anti-orbital weapon, and armies also contribute partially to return fire. Bombardment follows a strict targeting priority: stardock contents first, then ground batteries, then armies, then stored goods, and finally factories and development. Only combat ships --- destroyers, cruisers, and battleships --- contribute bombardment firepower. Scouts, transports, and ETACs do not.

See @missions for the detailed mechanics of bombardment, invasion, and blitz missions.

#pagebreak()

// ─── 6. Missions and Orders ─────────────────────────────────────────────

= Missions and Orders <missions>

A fleet always has exactly one standing order. If you issue a new order before maintenance, it replaces whatever the fleet was doing. Missions fall into three categories. _One-shot_ missions cause the fleet to travel, perform an action, and revert to Hold Position --- you must issue new orders afterward. _Persistent_ missions remain active until you replace them or game rules invalidate them (for example, a Join mission is abandoned if the host fleet is destroyed). _Hostile_ missions send the fleet to a target world where it waits; the assault executes on the _next_ maintenance tick after arrival, not immediately. Plan accordingly.

#figure(
  table(
    columns: (auto, auto, auto, 1fr),
    align: (right, left, left, left),
    inset: 6pt,
    table.header(
      [No], [Mission], [Type], [Description],
    ),
    [00], [None],            [Persistent], [Hold position at current location.],
    [01], [Move Fleet],      [One-shot],   [Travel to a specified sector, then revert to Hold.],
    [02], [Seek Home],       [One-shot],   [Return to nearest owned planet; retargets if that planet is captured en route.],
    [03], [Patrol],          [Persistent], [Move within sector deep space to intercept enemies.],
    [04], [Guard Starbase],  [Persistent], [Escort a starbase; fight alongside it.],
    [05], [Blockade],        [Persistent], [Prevent access to a planet. Stops enemy launches and landings.],
    [06], [Bombard],         [Hostile],    [Damage factories, batteries, armies, stardock contents, and production.],
    [07], [Invade],          [Hostile],    [Suppress batteries, bombard, then land armies. Requires loaded transports.],
    [08], [Blitz],           [Hostile],    [Drop armies immediately, dodging batteries. High army risk.],
    [09], [View],            [One-shot],   [Long-range scan of owner and production from system edge.],
    [10], [Scout Sector],    [One-shot],   [Passive stealth surveillance of sector deep space (requires Scout).],
    [11], [Scout System],    [One-shot],   [Active spy mission into a solar system (requires Scout).],
    [12], [Colonize],        [One-shot],   [Terraform and claim unowned planet (requires ETAC). ETAC survives for reuse.],
    [13], [Join Fleet],      [Persistent], [Chase and merge with target fleet. Abandons if host is destroyed.],
    [14], [Rendezvous],      [Persistent], [Meet other fleets at sector. Lowest fleet ID becomes host.],
    [15], [Salvage],         [One-shot],   [Scrap ships at planet for \~50% of build cost.],
  ),
)

=== Mission Details

==== One-Shot Missions

*Mission 1: Move to Sector.* A simple transit order. The fleet travels to the destination sector at the speed of its slowest ship, then stops and reverts to Hold Position. You must issue new orders if you want it to do anything else.

*Mission 2: Seek Home.* The fleet travels to the nearest planet you own. If that planet is captured while the fleet is en route, it automatically redirects to the next nearest friendly planet. On arrival, it reverts to Hold Position.

*Mission 9: View a World.* A safe, long-range scan. The fleet approaches the edge of the target system, scans for owner and production data, and immediately backs off into deep space. It reverts to Hold Position after reporting.

*Missions 10 and 11: Scout Sector / Scout System.* Both require at least one Scout ship, and a fleet consisting of a single Scout is the least likely to be detected. Scout Sector is a passive, stealthy patrol --- unlike Mission 3, the fleet will not engage enemies but instead relies on stealth to observe traffic without being seen. Scout System is an active spy run where the Scout penetrates the system to report on ground batteries, armies, current production, stardock contents, and orbiting fleets. Both revert to Hold Position after reporting.

*Mission 12: Colonize a World.* Requires at least one ETAC (Environmental Transformation And Colonization ship). If the planet is unowned, it is terraformed and claimed. The new colony starts with one garrison army and very low current production, which grows faster under low taxes. The ETAC is not consumed --- it survives and can colonize additional planets. If the planet is already owned, the ETAC aborts, reports the owner's identity and production potential, and waits for new orders.

*Mission 15: Salvage.* The fleet travels to the specified planet and scraps its ships for approximately *50%* of the original build cost, returned as stored production points on that planet. It reverts to Hold Position after scrapping.

==== Persistent Standing Missions

*Mission 0: Hold Position.* The default idle state. The fleet stays at its current location and takes defensive action based on ROE if hostile fleets approach.

*Mission 3: Patrol a Sector.* The fleet moves within deep space to intercept enemies passing through. If the patrol spots anything, it sends a report, and it engages based on your ROE settings. The mission remains active until you assign a new one.

*Mission 4: Guard Starbase.* The fleet escorts a starbase and fights alongside it. Be aware that if your fleet has a high ROE, it may break formation to chase enemy fleets entering the system, leaving the starbase temporarily vulnerable.

*Mission 5: Guard/Blockade a World.* A strategy of denial. The blockade stops enemy fleets from using the planet, intercepts ships launching from stardock, and paralyzes the enemy's ability to deploy forces from that world. It remains active until you assign a new mission.

*Missions 13 and 14: Fleet Coordination.* Mission 13 (Join Fleet) causes the fleet to chase a specific host fleet and merge with it when they meet. If the host is destroyed before they rendezvous, the joining fleet abandons the mission. Mission 14 (Rendezvous) sends multiple fleets to a sector where the fleet with the lowest Fleet ID becomes the host of the combined force. The rendezvous point remains active so additional fleets can keep merging there.

==== Hostile (Delayed-Resolution) Missions

#admonition("IMPORTANT")[Hostile missions require the fleet to be *in orbit at the start of maintenance* to execute. A fleet that arrives at the target world this turn will carry out its assault *next turn*. This one-turn delay is a critical tactical consideration --- defend accordingly.]

*Mission 6: Bombard a World.* Only destroyers, cruisers, and battleships contribute bombardment firepower --- scouts, transports, and ETACs do not. Bombardment hits stardock contents first, then ground batteries, then armies, then stored goods, and finally factories and development. Ground batteries and armies fire back at the bombarding fleet. Use bombardment to soften up a world before invasion, or to deny resources to an enemy by destroying production and stardock contents.

*Mission 7: Invade a World.* A three-stage deliberate assault. First, combat ships exchange fire with ground batteries in orbital suppression --- transports cannot land until all batteries are destroyed, though even a failed suppression still damages the planet. Once batteries are gone, surviving combat ships inflict bombardment-style damage on armies and industry. Finally, transports land their armies to fight the surviving defenders in simultaneous ground combat where the defender wins ties. Capture requires destroying all defending armies, after which your surviving armies become the new garrison. The invasion inflicts significant factory damage, and conquered planets need approximately two turns before they are fully converted to your production.

*Mission 8: Blitz a World.* Transports drop armies immediately in a fast assault that bypasses the full orbital suppression sequence. Escorting combat ships provide brief cover fire, but surviving batteries fire directly on descending transports, causing heavy losses. Landed armies fight defenders immediately, and the defender receives a defensive bonus. If you take the planet, surviving ground batteries transfer intact to your control. The blitz preserves factories but carries high risk to your armies and transports --- a 2:1 army advantage or better is recommended. Choose blitz when the planet has few or no batteries and you want to preserve its industry, or when speed matters more than casualties.

#pagebreak()

// ─── 7. Interface and Commands ──────────────────────────────────────────

= Interface and Commands

The game is organized around four primary menus. From the *Main Menu*, you access General Command (*G*) for autopilot, diplomacy, and reports; Planet Command (*P*) for economy and production; Fleet Command (*F*) for ship movement and missions; Information Database (*I*) to review known planet data; and View Starmap (*V*) for a graphic map of the galaxy.

=== General Command

General Command handles empire-wide administration. Autopilot (*A*) lets the computer manage your defenses if you miss turns. Diplomacy (*E*) lets you declare other players as Neutral or Enemy --- neutral fleets will not attack unless provoked, while enemy fleets will attack on sight based on ROE. Messages (*C*) lets you send messages to other empires.

=== Planet Command

Planet Command controls your economy and ground operations. Tax (*T*) sets the empire-wide tax rate. Scorch Earth (*S*) destroys your own factories to deny them to an invader. Build (*B*) spends production points on ships, defenses, or starbases. Commission (*C*) assigns newly built ships from stardock into active fleets. Load and Unload (*L* / *U*) move armies between the planet surface and troop transports.

=== Fleet Command

Fleet Command controls your ships in space. Mission (*O*) assigns missions 0--15. ROE (*C*) changes a fleet's rules of engagement. Merge (*M*) combines fleets that are in the same sector. Transfer (*T*) moves individual ships between fleets.

=== Building and Commissioning

Each planet has a *10-slot build queue*, and all builds complete in a single maintenance turn. Ships and starbases, upon completion, move to *Stardock* --- a holding area on the planet where they sit idle and vulnerable until commissioned. Ships are commissioned into numbered fleets, while starbases are commissioned individually and managed through their own Starbase Command submenu. Armies and ground batteries, by contrast, deploy directly to the planet surface and do not pass through stardock.

A planet without a starbase can spend up to its Present Production in a single turn. A planet with an orbiting starbase can spend up to *5x* its Present Production, drawing from Stored Production Points (see @economy).

#admonition("WARNING")[Troop transports are built empty. You must manually load armies onto them before sending them into battle.]

#admonition("WARNING")[Stardock contents are a prime target for enemy bombardment. Commission your ships promptly or risk losing them before they ever see combat.]

#pagebreak()

// ─── 8. Strategy ────────────────────────────────────────────────────────

= Strategy

=== Early Game: The Land Grab

The opening turns are a race for territory. ETACs are your most valuable early asset --- grab every raw planet you can find before your rivals do. Send single destroyers ahead as scouts to locate neighbors before they locate you. Keep your tax rate at or below 65% to avoid damaging production, and drop taxes even lower on new colonies so they develop quickly. The temptation to tax at 100% for immediate cash is strong, but it cripples long-term growth.

=== Mid-Game: Consolidation

Once the easy colonies are claimed, the game shifts to fortification and intelligence. Build starbases on your best worlds to boost both defense and production capacity. Never attack a planet blindly --- use Scout ships on Mission 11 to count enemy batteries and garrison strength before committing forces. This is the "get tough" phase: when raw planets are gone, the only way to grow is war.

=== Late Game: Total War

In the endgame, fleet composition and denial matter more than raw numbers. Mix destroyers, cruisers, and battleships in every fleet to trigger the combined arms bonus and distribute damage across hull types --- pure battleship fleets are expensive and miss the bonus. If you cannot hold a planet, scorch it. If you cannot take a planet, blockade or bombard it into uselessness. And in a 25-player galaxy, diplomacy is not optional: you cannot fight everyone at once. Form alliances, even temporary ones, and break them only when the timing is right.

#pagebreak()

// ─── 9. Historical Context ─────────────────────────────────────────────

= Historical Context

*The BBS Era (1990--1992)* \
Esterian Conquest was created by Bentley C. Griffith between 1990 and 1992 as a "door game" for Bulletin Board Systems. In an era before the World Wide Web, players dialed into servers over phone lines to play strategy games against dozens of strangers. Everything ran through rigid 80x25 text terminals --- every menu, every star map, every battle report squeezed into that tiny viewport. EC stood alongside classics like _TradeWars 2002_ and _Solar Realms Elite_, but was distinguished by its depth, its hands-off design, and the sheer scale of its campaigns.

*What Made It Special* \
Most multiplayer games of the era demanded constant attention. Esterian Conquest was different. You checked in once a day, submitted your orders, and went about your life. Overnight, the engine processed every empire simultaneously --- fleets moved, economies grew, battles resolved, and alliances were tested. When you logged in the next day, a stack of reports was waiting. Campaigns ran for months, and the stories they produced --- surprise invasions, desperate blockades, betrayals at the worst possible moment --- were the kind that stuck with players for years.

*The Rust Port (2026)* \
This version is a faithful preservation of the original game, rebuilt from the ground up in Rust. Every rule, every formula, every mechanic has been recovered from the original binaries and validated for compatibility. If you played EC on a BBS in the 1990s, this is the same game. If you are discovering it for the first time, you are playing the real thing --- not a clone, not an approximation.

The immediate goal is a drop-in replacement that runs natively on modern systems without emulation, while preserving the classic experience for BBS sysops and Telnet players.

*Nostrian Conquest* \
Looking further ahead, the project aims to free the game from centralized hosts entirely. Under the working title *Nostrian Conquest*, the next evolution will use the Nostr protocol as a transport layer --- players submit encrypted turn orders using cryptographic keys, and results are broadcast back through relays. No central server to shut down. No single point of failure. A serverless galaxy where the fog of war is enforced by cryptography. Freed from the shackles of the 80x25 Telnet screen, Nostrian Conquest will feature a full-screen modern ANSI/UTF-8 terminal interface worthy of the game's strategic depth.

#pagebreak()

// ─── 10. Original Sources ───────────────────────────────────────────────

= Original Sources

The preserved originals in `original/v1.5/` remain the definitive reference:

- `ECQSTART.DOC` --- Quick-start guide
- `ECPLAYER.DOC` --- Detailed player manual
- `ECREADME.DOC` --- Release and package information
- `ECGAME.EXE` --- The original 1992 player client

This manual combines and polishes those foundations into a single cohesive guide.
