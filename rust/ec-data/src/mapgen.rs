use crate::{CoreGameData, GameStateBuilder, GameStateMutationError, PlanetRecord};

const REROLL_CANDIDATES: usize = 64;
const HOMEWORLD_EDGE_MARGIN: f32 = 2.0;
const HOMEWORLD_MIN_DISTANCE_RATIO: f32 = 0.28;
const LOCAL_WORLD_COUNT_PER_PLAYER: usize = 2;
const EARLY_RADIUS: f32 = 5.5;
const CONTESTED_GAP_LIMIT: f32 = 2.75;
const NEUTRAL_MIN_SPACING: f32 = 1.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedWorld {
    pub coords: [u8; 2],
    pub potential_production: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapMetrics {
    pub score: f32,
    pub early_count_range: u8,
    pub early_value_range: u16,
    pub contested_worlds: u8,
    pub min_homeworld_spacing: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedMap {
    pub seed: u64,
    pub map_size: u8,
    pub homeworld_coords: Vec<[u8; 2]>,
    pub neutral_worlds: Vec<GeneratedWorld>,
    pub metrics: MapMetrics,
}

pub fn build_seeded_new_game(
    player_count: u8,
    year: u16,
    seed: u64,
) -> Result<CoreGameData, GameStateMutationError> {
    let generated = generate_map(player_count, seed);
    let mut data = GameStateBuilder::new()
        .with_player_count(player_count)
        .with_year(year)
        .with_homeworld_coords(generated.homeworld_coords.clone())
        .build_joinable_new_game_baseline()?;

    for (idx, world) in generated.neutral_worlds.iter().enumerate() {
        let record_index = player_count as usize + idx;
        if let Some(planet) = data.planets.records.get_mut(record_index) {
            seed_unowned_world(planet, *world, record_index + 1);
        }
    }

    Ok(data)
}

pub fn build_seeded_initialized_game(
    player_count: u8,
    year: u16,
    seed: u64,
) -> Result<CoreGameData, GameStateMutationError> {
    let generated = generate_map(player_count, seed);
    let mut data = GameStateBuilder::new()
        .with_player_count(player_count)
        .with_year(year)
        .with_homeworld_coords(generated.homeworld_coords.clone())
        .build_initialized_baseline()?;

    for (idx, world) in generated.neutral_worlds.iter().enumerate() {
        let record_index = player_count as usize + idx;
        if let Some(planet) = data.planets.records.get_mut(record_index) {
            seed_unowned_world(planet, *world, record_index + 1);
        }
    }

    Ok(data)
}

pub fn generate_map(player_count: u8, seed: u64) -> GeneratedMap {
    let map_size = map_size_for_player_count(player_count);
    let mut best_map = None;
    let mut best_score = f32::MIN;

    for reroll in 0..REROLL_CANDIDATES {
        let candidate_seed =
            seed ^ ((player_count as u64) << 32) ^ ((reroll as u64) << 48) ^ 0xEC15_1000_0000_0000;
        let mut rng = Lcg::new(candidate_seed);
        let homeworld_coords = generate_homeworlds(player_count, map_size, &mut rng);
        let neutral_worlds = generate_neutral_worlds(
            player_count,
            map_size,
            seed,
            reroll as u32,
            &homeworld_coords,
            &mut rng,
        );
        let metrics = score_map(map_size, &homeworld_coords, &neutral_worlds);
        if metrics.score > best_score {
            best_score = metrics.score;
            best_map = Some(GeneratedMap {
                seed,
                map_size,
                homeworld_coords,
                neutral_worlds,
                metrics,
            });
        }
    }

    let generated = best_map.expect("map generation should always produce a candidate");
    debug_assert!(all_systems_unique(
        &generated.homeworld_coords,
        &generated.neutral_worlds
    ));
    generated
}

pub fn map_size_for_player_count(player_count: u8) -> u8 {
    match player_count {
        1..=4 => 18,
        5..=9 => 27,
        10..=16 => 36,
        _ => 45,
    }
}

fn generate_homeworlds(player_count: u8, map_size: u8, rng: &mut Lcg) -> Vec<[u8; 2]> {
    let min_distance_sq = (map_size as f32 * HOMEWORLD_MIN_DISTANCE_RATIO).powi(2);
    let regions = homeworld_regions(player_count, map_size);
    let mut homeworlds = Vec::with_capacity(player_count as usize);

    for region in &regions {
        let mut best = None;
        let mut best_score = f32::MIN;
        for _ in 0..96 {
            let x = rng.range_u8(region.min[0], region.max[0]);
            let y = rng.range_u8(region.min[1], region.max[1]);
            let candidate = [x, y];
            if homeworlds.contains(&candidate) {
                continue;
            }

            let mut min_dist_sq = f32::MAX;
            for other in &homeworlds {
                let dist_sq = distance_sq(candidate, *other);
                min_dist_sq = min_dist_sq.min(dist_sq);
            }
            if !homeworlds.is_empty() && min_dist_sq < min_distance_sq {
                continue;
            }

            let edge_clearance = edge_clearance(candidate, map_size);
            let region_distance = distance(candidate, region.anchor);
            let score = edge_clearance * 1.5 + min_dist_sq.sqrt() * 1.8 - region_distance * 1.2;
            if score > best_score {
                best_score = score;
                best = Some(candidate);
            }
        }

        homeworlds.push(best.unwrap_or(region.anchor));
    }

    homeworlds
}

fn generate_neutral_worlds(
    player_count: u8,
    map_size: u8,
    seed: u64,
    reroll: u32,
    homeworlds: &[[u8; 2]],
    rng: &mut Lcg,
) -> Vec<GeneratedWorld> {
    let total_neutrals = player_count as usize * 4;
    let local_count = player_count as usize * LOCAL_WORLD_COUNT_PER_PLAYER;
    let frontier_count = total_neutrals.saturating_sub(local_count);
    let mut worlds = Vec::with_capacity(total_neutrals);
    let mut used = homeworlds.to_vec();

    for (home_idx, &home) in homeworlds.iter().enumerate() {
        for slot in 0..LOCAL_WORLD_COUNT_PER_PLAYER {
            let coords = choose_local_world(
                home_idx, home, map_size, seed, reroll, homeworlds, &used, rng,
            );
            let potential = local_potential(seed, reroll, home_idx, slot);
            used.push(coords);
            worlds.push(GeneratedWorld {
                coords,
                potential_production: potential,
            });
        }
    }

    let frontier_potentials = frontier_potentials(frontier_count, rng);
    for (idx, potential) in frontier_potentials.into_iter().enumerate() {
        let coords = choose_frontier_world(
            idx, potential, map_size, seed, reroll, homeworlds, &worlds, &used, rng,
        );
        used.push(coords);
        worlds.push(GeneratedWorld {
            coords,
            potential_production: potential,
        });
    }

    worlds
}

fn choose_local_world(
    home_idx: usize,
    home: [u8; 2],
    map_size: u8,
    seed: u64,
    reroll: u32,
    homeworlds: &[[u8; 2]],
    used: &[[u8; 2]],
    rng: &mut Lcg,
) -> [u8; 2] {
    let mut best = None;
    let mut best_score = f32::MIN;
    let base_angle = std::f32::consts::TAU * home_idx as f32 / homeworlds.len().max(1) as f32;

    for attempt in 0..96 {
        let orbit = 2.6 + rng.next_f32() * 2.4;
        let angle = base_angle + attempt as f32 * 0.31 + (rng.next_f32() - 0.5) * 1.0;
        let x = clamp_coord(home[0] as f32 + orbit * angle.cos(), map_size, 1.0);
        let y = clamp_coord(home[1] as f32 + orbit * angle.sin(), map_size, 1.0);
        let candidate = [x, y];
        if !candidate_allowed(candidate, used) {
            continue;
        }

        let nearest_idx = nearest_homeworld(candidate, homeworlds);
        if nearest_idx != home_idx {
            continue;
        }

        let nearest = distance(candidate, home);
        let spacing = nearest_used_distance(candidate, used);
        let noise = density_noise(candidate, map_size, seed, reroll);
        let score = 18.0 - (nearest - 3.8).abs() * 4.0 + spacing * 1.8 + noise * 3.0;
        if score > best_score {
            best_score = score;
            best = Some(candidate);
        }
    }

    best.unwrap_or_else(|| fallback_local_world(home, map_size, used))
}

fn choose_frontier_world(
    frontier_idx: usize,
    potential: u8,
    map_size: u8,
    seed: u64,
    reroll: u32,
    homeworlds: &[[u8; 2]],
    existing_worlds: &[GeneratedWorld],
    used: &[[u8; 2]],
    rng: &mut Lcg,
) -> [u8; 2] {
    let mut best = None;
    let mut best_score = f32::MIN;
    let center = (map_size as f32 - 1.0) / 2.0;

    for attempt in 0..192 {
        let x = rng.range_u8(1, map_size.saturating_sub(2));
        let y = rng.range_u8(1, map_size.saturating_sub(2));
        let candidate = [x, y];
        if !candidate_allowed(candidate, used) {
            continue;
        }

        let dists = sorted_homeworld_distances(candidate, homeworlds);
        let nearest = dists[0];
        let second = *dists.get(1).unwrap_or(&nearest);
        let spacing = nearest_used_distance(candidate, used);
        let gap = (nearest - second).abs();
        let contest = if gap < CONTESTED_GAP_LIMIT { 12.0 } else { 0.0 };
        let center_bias =
            6.0 / (1.0 + ((x as f32 - center).abs() + (y as f32 - center).abs()) * 0.5);
        let noise = density_noise(candidate, map_size, seed, reroll);
        let void_penalty = if nearest < 2.5 { 8.0 } else { 0.0 };
        let frontier_ring = if frontier_idx % 3 == 0 { 1.5 } else { 0.0 };
        let balance_bonus =
            frontier_balance_bonus(candidate, potential, homeworlds, existing_worlds);
        let score =
            spacing * 1.7 + contest + center_bias + noise * 6.0 + frontier_ring + balance_bonus
                - gap * 1.4
                - void_penalty;
        if score > best_score {
            best_score = score;
            best = Some(candidate);
        }

        if attempt > 64 && best_score > 18.0 {
            break;
        }
    }

    best.unwrap_or_else(|| fallback_frontier_world(map_size, used))
}

fn frontier_balance_bonus(
    candidate: [u8; 2],
    potential: u8,
    homeworlds: &[[u8; 2]],
    existing_worlds: &[GeneratedWorld],
) -> f32 {
    let mut counts = vec![0u8; homeworlds.len()];
    let mut values = vec![0u16; homeworlds.len()];

    for (home_idx, &home) in homeworlds.iter().enumerate() {
        for world in existing_worlds {
            if distance(world.coords, home) <= EARLY_RADIUS
                && nearest_homeworld(world.coords, homeworlds) == home_idx
            {
                counts[home_idx] += 1;
                values[home_idx] += world.potential_production as u16;
            }
        }
    }

    let before_count_range = range_u8(&counts);
    let before_value_range = range_u16(&values);
    let owner_idx = nearest_homeworld(candidate, homeworlds);
    if distance(candidate, homeworlds[owner_idx]) <= EARLY_RADIUS {
        counts[owner_idx] += 1;
        values[owner_idx] += potential as u16;
    }
    let after_count_range = range_u8(&counts);
    let after_value_range = range_u16(&values);

    (before_count_range as f32 - after_count_range as f32) * 18.0
        + (before_value_range as f32 - after_value_range as f32) * 0.22
}

fn local_potential(seed: u64, reroll: u32, home_idx: usize, slot: usize) -> u8 {
    let base = if slot == 0 { 68 } else { 86 };
    let wobble = hashed_range(seed, reroll, home_idx as u32 * 4 + slot as u32, 0, 8);
    (base + wobble).min(99)
}

fn frontier_potentials(count: usize, rng: &mut Lcg) -> Vec<u8> {
    let mut values = Vec::with_capacity(count);
    if count == 0 {
        return values;
    }

    values.push(130 + (rng.next_u8() % 21));
    if count > 1 {
        values.push(108 + (rng.next_u8() % 18));
    }
    while values.len() < count {
        let next = match values.len() % 5 {
            0 => 34 + (rng.next_u8() % 18),
            1 => 48 + (rng.next_u8() % 22),
            2 => 62 + (rng.next_u8() % 24),
            3 => 78 + (rng.next_u8() % 18),
            _ => 90 + (rng.next_u8() % 12),
        };
        values.push(next.min(150));
    }

    for idx in 0..values.len() {
        let swap = (rng.next_u32() as usize) % values.len();
        values.swap(idx, swap);
    }
    values
}

fn score_map(
    map_size: u8,
    homeworlds: &[[u8; 2]],
    neutral_worlds: &[GeneratedWorld],
) -> MapMetrics {
    let mut early_counts = Vec::with_capacity(homeworlds.len());
    let mut early_values = Vec::with_capacity(homeworlds.len());

    for (home_idx, &home) in homeworlds.iter().enumerate() {
        let mut count = 0u8;
        let mut value = 0u16;
        for world in neutral_worlds {
            if distance(world.coords, home) <= EARLY_RADIUS
                && nearest_homeworld(world.coords, homeworlds) == home_idx
            {
                count += 1;
                value += world.potential_production as u16;
            }
        }
        early_counts.push(count);
        early_values.push(value);
    }

    let early_count_range = range_u8(&early_counts);
    let early_value_range = range_u16(&early_values);
    let contested_worlds = neutral_worlds
        .iter()
        .filter(|world| {
            let dists = sorted_homeworld_distances(world.coords, homeworlds);
            let nearest = dists[0];
            let second = *dists.get(1).unwrap_or(&nearest);
            (nearest - second).abs() < CONTESTED_GAP_LIMIT
        })
        .count() as u8;
    let min_homeworld_spacing = minimum_pair_distance(homeworlds);
    let connected = neutral_worlds
        .iter()
        .all(|world| sorted_homeworld_distances(world.coords, homeworlds)[0] <= 9.0);

    let score = 120.0 - early_count_range as f32 * 42.0 - early_value_range as f32 * 0.70
        + contested_worlds as f32 * 2.5
        + min_homeworld_spacing * 3.2
        + if connected { 18.0 } else { -60.0 }
        - isolated_home_penalty(homeworlds, neutral_worlds) * 28.0
        - dominant_cluster_penalty(homeworlds, neutral_worlds) * 0.32
        + density_balance_bonus(map_size, neutral_worlds)
        - system_overlap_penalty(homeworlds, neutral_worlds) * 200.0;

    MapMetrics {
        score,
        early_count_range,
        early_value_range,
        contested_worlds,
        min_homeworld_spacing,
    }
}

fn isolated_home_penalty(homeworlds: &[[u8; 2]], neutral_worlds: &[GeneratedWorld]) -> f32 {
    homeworlds
        .iter()
        .map(|&home| {
            neutral_worlds
                .iter()
                .filter(|world| distance(world.coords, home) <= 6.5)
                .count()
        })
        .map(|count| if count >= 2 { 0.0 } else { (2 - count) as f32 })
        .sum()
}

fn dominant_cluster_penalty(homeworlds: &[[u8; 2]], neutral_worlds: &[GeneratedWorld]) -> f32 {
    let values = homeworlds
        .iter()
        .enumerate()
        .map(|(idx, &home)| {
            neutral_worlds
                .iter()
                .filter(|world| nearest_homeworld(world.coords, homeworlds) == idx)
                .map(|world| {
                    let dist = distance(world.coords, home).max(1.0);
                    (world.potential_production as f32) / dist
                })
                .sum::<f32>()
        })
        .collect::<Vec<_>>();

    let min = values.iter().copied().fold(f32::INFINITY, f32::min);
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    max - min
}

fn density_balance_bonus(map_size: u8, neutral_worlds: &[GeneratedWorld]) -> f32 {
    let center = (map_size as f32 - 1.0) / 2.0;
    neutral_worlds
        .iter()
        .map(|world| {
            let center_distance = ((world.coords[0] as f32 - center).abs()
                + (world.coords[1] as f32 - center).abs())
                / map_size as f32;
            let richness = world.potential_production as f32 / 150.0;
            (1.0 - center_distance) * richness * 2.5
        })
        .sum()
}

fn system_overlap_penalty(homeworlds: &[[u8; 2]], neutral_worlds: &[GeneratedWorld]) -> f32 {
    let mut seen = homeworlds.to_vec();
    let mut duplicates = 0u32;
    for world in neutral_worlds {
        if seen.contains(&world.coords) {
            duplicates += 1;
        } else {
            seen.push(world.coords);
        }
    }
    duplicates as f32
}

fn all_systems_unique(homeworlds: &[[u8; 2]], neutral_worlds: &[GeneratedWorld]) -> bool {
    let mut seen = homeworlds.to_vec();
    for world in neutral_worlds {
        if seen.contains(&world.coords) {
            return false;
        }
        seen.push(world.coords);
    }
    true
}

fn seed_unowned_world(
    planet: &mut PlanetRecord,
    world: GeneratedWorld,
    world_index_1_based: usize,
) {
    *planet = PlanetRecord::new_zeroed();
    planet.set_coords_raw(world.coords);
    planet.set_potential_production_raw([world.potential_production, 0]);
    planet.set_planet_tax_rate_raw(0);
    planet.set_planet_name(&format!("World {:02}", world_index_1_based));
}

fn candidate_allowed(candidate: [u8; 2], used: &[[u8; 2]]) -> bool {
    !used.contains(&candidate) && nearest_used_distance(candidate, used) >= NEUTRAL_MIN_SPACING
}

fn nearest_homeworld(candidate: [u8; 2], homeworlds: &[[u8; 2]]) -> usize {
    homeworlds
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            distance_sq(candidate, **a)
                .partial_cmp(&distance_sq(candidate, **b))
                .unwrap()
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn sorted_homeworld_distances(candidate: [u8; 2], homeworlds: &[[u8; 2]]) -> Vec<f32> {
    let mut distances = homeworlds
        .iter()
        .map(|coords| distance(candidate, *coords))
        .collect::<Vec<_>>();
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    distances
}

fn minimum_pair_distance(coords: &[[u8; 2]]) -> f32 {
    let mut min = f32::MAX;
    for left in 0..coords.len() {
        for right in left + 1..coords.len() {
            min = min.min(distance(coords[left], coords[right]));
        }
    }
    if min.is_finite() { min } else { 0.0 }
}

fn nearest_used_distance(candidate: [u8; 2], used: &[[u8; 2]]) -> f32 {
    used.iter()
        .map(|other| distance(candidate, *other))
        .fold(f32::MAX, f32::min)
}

fn edge_clearance(candidate: [u8; 2], map_size: u8) -> f32 {
    f32::min(
        f32::min(candidate[0] as f32, candidate[1] as f32),
        f32::min(
            (map_size - 1 - candidate[0]) as f32,
            (map_size - 1 - candidate[1]) as f32,
        ),
    )
}

fn distance(a: [u8; 2], b: [u8; 2]) -> f32 {
    distance_sq(a, b).sqrt()
}

fn distance_sq(a: [u8; 2], b: [u8; 2]) -> f32 {
    let dx = a[0] as f32 - b[0] as f32;
    let dy = a[1] as f32 - b[1] as f32;
    dx * dx + dy * dy
}

fn density_noise(candidate: [u8; 2], map_size: u8, seed: u64, reroll: u32) -> f32 {
    let x = candidate[0] as f32 / map_size.max(1) as f32;
    let y = candidate[1] as f32 / map_size.max(1) as f32;
    let phase_a = (seed as f32 / 97.0) + reroll as f32 * 0.19;
    let phase_b = (seed as f32 / 211.0) + reroll as f32 * 0.11;
    let coarse = ((x * 5.7 + phase_a).sin() + (y * 6.3 + phase_b).cos()) * 0.5;
    let fine = ((x * 13.0 + phase_b * 0.7).sin() * (y * 11.0 + phase_a * 0.4).cos()) * 0.5;
    (coarse * 0.65 + fine * 0.35).clamp(-1.0, 1.0)
}

fn hashed_range(seed: u64, reroll: u32, salt: u32, min: u8, max: u8) -> u8 {
    if min >= max {
        return min;
    }
    let mut value = seed ^ ((reroll as u64) << 16) ^ ((salt as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
    value ^= value >> 33;
    value = value.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    value ^= value >> 33;
    value = value.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    value ^= value >> 33;
    let span = max - min + 1;
    min + (value as u8 % span)
}

fn range_u8(values: &[u8]) -> u8 {
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    max.saturating_sub(min)
}

fn range_u16(values: &[u16]) -> u16 {
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    max.saturating_sub(min)
}

fn clamp_coord(value: f32, map_size: u8, margin: f32) -> u8 {
    value
        .round()
        .clamp(margin, (map_size as f32 - 1.0) - margin) as u8
}

fn fallback_local_world(home: [u8; 2], map_size: u8, used: &[[u8; 2]]) -> [u8; 2] {
    for dx in -4..=4 {
        for dy in -4..=4 {
            let x = (home[0] as i16 + dx).clamp(1, map_size as i16 - 2) as u8;
            let y = (home[1] as i16 + dy).clamp(1, map_size as i16 - 2) as u8;
            let candidate = [x, y];
            if candidate != home && candidate_allowed(candidate, used) {
                return candidate;
            }
        }
    }
    [1, 1]
}

fn fallback_frontier_world(map_size: u8, used: &[[u8; 2]]) -> [u8; 2] {
    let center = map_size / 2;
    for ring in 0..map_size {
        for dx in -(ring as i16)..=(ring as i16) {
            for dy in -(ring as i16)..=(ring as i16) {
                let x = (center as i16 + dx).clamp(1, map_size as i16 - 2) as u8;
                let y = (center as i16 + dy).clamp(1, map_size as i16 - 2) as u8;
                let candidate = [x, y];
                if candidate_allowed(candidate, used) {
                    return candidate;
                }
            }
        }
    }
    [center, center]
}

#[derive(Debug, Clone)]
struct Lcg {
    state: u64,
}

#[derive(Debug, Clone, Copy)]
struct HomeworldRegion {
    min: [u8; 2],
    max: [u8; 2],
    anchor: [u8; 2],
}

fn homeworld_regions(player_count: u8, map_size: u8) -> Vec<HomeworldRegion> {
    let low = HOMEWORLD_EDGE_MARGIN as u8;
    let high = map_size.saturating_sub(low + 1);
    let mid = map_size / 2;
    match player_count {
        1 => vec![HomeworldRegion {
            min: [mid.saturating_sub(2), mid.saturating_sub(2)],
            max: [mid + 2, mid + 2],
            anchor: [mid, mid],
        }],
        2 => vec![
            HomeworldRegion {
                min: [low, low + 1],
                max: [mid.saturating_sub(2), high.saturating_sub(1)],
                anchor: [mid.saturating_sub(4), mid],
            },
            HomeworldRegion {
                min: [mid + 1, low + 1],
                max: [high, high.saturating_sub(1)],
                anchor: [mid + 4, mid],
            },
        ],
        3 => vec![
            HomeworldRegion {
                min: [low, low],
                max: [mid.saturating_sub(2), mid.saturating_sub(2)],
                anchor: [mid.saturating_sub(4), mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [mid + 1, low],
                max: [high, mid.saturating_sub(2)],
                anchor: [mid + 4, mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [mid.saturating_sub(2), mid + 1],
                max: [mid + 2, high],
                anchor: [mid, mid + 4],
            },
        ],
        4 => vec![
            HomeworldRegion {
                min: [low, low],
                max: [mid.saturating_sub(2), mid.saturating_sub(2)],
                anchor: [mid.saturating_sub(4), mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [mid + 1, low],
                max: [high, mid.saturating_sub(2)],
                anchor: [mid + 4, mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [low, mid + 1],
                max: [mid.saturating_sub(2), high],
                anchor: [mid.saturating_sub(4), mid + 4],
            },
            HomeworldRegion {
                min: [mid + 1, mid + 1],
                max: [high, high],
                anchor: [mid + 4, mid + 4],
            },
        ],
        _ => grid_homeworld_regions(player_count as usize, low, high),
    }
}

fn grid_homeworld_regions(player_count: usize, low: u8, high: u8) -> Vec<HomeworldRegion> {
    let cols = (player_count as f32).sqrt().ceil() as usize;
    let rows = player_count.div_ceil(cols);
    let usable_width = (high.saturating_sub(low) as usize).max(cols);
    let usable_height = (high.saturating_sub(low) as usize).max(rows);
    let cell_width = (usable_width / cols).max(1);
    let cell_height = (usable_height / rows).max(1);
    let mut regions = Vec::with_capacity(player_count);

    for idx in 0..player_count {
        let col = idx % cols;
        let row = idx / cols;
        let min_x = low as usize + col * cell_width;
        let max_x = if col + 1 == cols {
            high as usize
        } else {
            low as usize + ((col + 1) * cell_width).saturating_sub(1)
        };
        let min_y = low as usize + row * cell_height;
        let max_y = if row + 1 == rows {
            high as usize
        } else {
            low as usize + ((row + 1) * cell_height).saturating_sub(1)
        };
        let anchor = [((min_x + max_x) / 2) as u8, ((min_y + max_y) / 2) as u8];
        regions.push(HomeworldRegion {
            min: [min_x as u8, min_y as u8],
            max: [max_x as u8, max_y as u8],
            anchor,
        });
    }

    regions
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(6364136223846793005).wrapping_add(1),
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u32() & 0xFF) as u8
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range_u8(&mut self, min: u8, max: u8) -> u8 {
        if min >= max {
            return min;
        }
        let span = max - min + 1;
        min + (self.next_u8() % span)
    }
}
