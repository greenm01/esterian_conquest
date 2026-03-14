use crate::{CoreGameData, GameStateBuilder, GameStateMutationError, PlanetRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedWorld {
    pub coords: [u8; 2],
    pub potential_production: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMap {
    pub seed: u64,
    pub map_size: u8,
    pub homeworld_coords: Vec<[u8; 2]>,
    pub neutral_worlds: Vec<GeneratedWorld>,
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
    let mut rng = Lcg::new(seed ^ ((player_count as u64) << 32) ^ 0xEC15_1000_0000_0000);
    let homeworld_coords = generate_homeworlds(player_count, map_size, &mut rng);
    let neutral_worlds =
        generate_neutral_worlds(player_count, map_size, &homeworld_coords, &mut rng);

    GeneratedMap {
        seed,
        map_size,
        homeworld_coords,
        neutral_worlds,
    }
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
    let center = (map_size as f32 - 1.0) / 2.0;
    let radius = map_size as f32 * 0.33;
    let angle_offset = rng.next_f32() * std::f32::consts::TAU;
    let margin = 2.0;
    let min_distance_sq = (map_size as f32 * 0.28).powi(2);
    let mut homeworlds = Vec::with_capacity(player_count as usize);

    for idx in 0..player_count as usize {
        let base_angle = angle_offset + std::f32::consts::TAU * idx as f32 / player_count as f32;
        let mut best = None;
        let mut best_score = f32::MIN;
        for _ in 0..64 {
            let angle_jitter = (rng.next_f32() - 0.5) * 0.55;
            let radial_jitter = (rng.next_f32() - 0.5) * (map_size as f32 * 0.08);
            let x = clamp_coord(
                center + (radius + radial_jitter) * (base_angle + angle_jitter).cos(),
                map_size,
                margin,
            );
            let y = clamp_coord(
                center + (radius + radial_jitter) * (base_angle + angle_jitter).sin(),
                map_size,
                margin,
            );
            let candidate = [x, y];
            if homeworlds.contains(&candidate) {
                continue;
            }

            let mut min_dist_sq = f32::MAX;
            for other in &homeworlds {
                let dx = candidate[0] as f32 - other[0] as f32;
                let dy = candidate[1] as f32 - other[1] as f32;
                min_dist_sq = min_dist_sq.min(dx * dx + dy * dy);
            }
            if !homeworlds.is_empty() && min_dist_sq < min_distance_sq {
                continue;
            }

            let edge_clearance = f32::min(
                f32::min(candidate[0] as f32, candidate[1] as f32),
                f32::min(
                    (map_size - 1 - candidate[0]) as f32,
                    (map_size - 1 - candidate[1]) as f32,
                ),
            );
            let score = edge_clearance * 2.0 + min_dist_sq.sqrt();
            if score > best_score {
                best = Some(candidate);
                best_score = score;
            }
        }
        homeworlds
            .push(best.unwrap_or_else(|| fallback_homeworld(idx, player_count as usize, map_size)));
    }

    homeworlds
}

fn generate_neutral_worlds(
    player_count: u8,
    map_size: u8,
    homeworlds: &[[u8; 2]],
    rng: &mut Lcg,
) -> Vec<GeneratedWorld> {
    let total_neutrals = (player_count as usize) * 4;
    let local_count = player_count as usize * 2;
    let frontier_count = total_neutrals.saturating_sub(local_count);
    let mut worlds = Vec::with_capacity(total_neutrals);
    let mut used = homeworlds.to_vec();
    let center = (map_size as f32 - 1.0) / 2.0;

    for (idx, &home) in homeworlds.iter().enumerate() {
        let local_values = [
            55 + ((idx as u8 * 7 + rng.next_u8() % 16) % 26),
            72 + ((idx as u8 * 11 + rng.next_u8() % 20) % 24),
        ];
        for (slot, potential) in local_values.into_iter().enumerate() {
            let angle =
                std::f32::consts::TAU * ((idx * 2 + slot) as f32) / (homeworlds.len() * 2) as f32;
            let mut placed = None;
            for _ in 0..64 {
                let distance = 3.0 + (rng.next_f32() * 2.5);
                let jitter = (rng.next_f32() - 0.5) * 0.9;
                let x = clamp_coord(
                    home[0] as f32 + distance * (angle + jitter).cos(),
                    map_size,
                    1.0,
                );
                let y = clamp_coord(
                    home[1] as f32 + distance * (angle + jitter).sin(),
                    map_size,
                    1.0,
                );
                let candidate = [x, y];
                if used.contains(&candidate) {
                    continue;
                }
                if nearest_homeworld(candidate, homeworlds) != idx {
                    continue;
                }
                placed = Some(candidate);
                break;
            }
            let coords = placed.unwrap_or_else(|| fallback_local_world(home, map_size, &used));
            used.push(coords);
            worlds.push(GeneratedWorld {
                coords,
                potential_production: potential,
            });
        }
    }

    let frontier_values = frontier_potentials(frontier_count, rng);
    for potential in frontier_values {
        let mut best = None;
        let mut best_score = f32::MIN;
        for _ in 0..128 {
            let x = rng.range_u8(1, map_size.saturating_sub(2));
            let y = rng.range_u8(1, map_size.saturating_sub(2));
            let candidate = [x, y];
            if used.contains(&candidate) {
                continue;
            }

            let dists = homeworld_distances(candidate, homeworlds);
            let nearest = dists[0];
            let second = dists[1];
            let center_bias = 1.0 / (1.0 + ((x as f32 - center).abs() + (y as f32 - center).abs()));
            let contest_score = 20.0 - (nearest - second).abs();
            let spacing_penalty = used
                .iter()
                .map(|other| distance_sq(candidate, *other).sqrt())
                .fold(f32::MAX, f32::min);
            let score = contest_score * 2.0 + spacing_penalty + center_bias * 8.0;
            if score > best_score {
                best = Some(candidate);
                best_score = score;
            }
        }
        let coords = best.unwrap_or_else(|| fallback_frontier_world(map_size, &used));
        used.push(coords);
        worlds.push(GeneratedWorld {
            coords,
            potential_production: potential,
        });
    }

    worlds
}

fn frontier_potentials(count: usize, rng: &mut Lcg) -> Vec<u8> {
    let mut values = Vec::with_capacity(count);
    for idx in 0..count {
        let value = match idx {
            0 => 130 + (rng.next_u8() % 21),
            1 => 100 + (rng.next_u8() % 21),
            n if n % 3 == 0 => 35 + (rng.next_u8() % 25),
            _ => 60 + (rng.next_u8() % 40),
        };
        values.push(value.min(150));
    }
    for idx in 0..count {
        let swap = (rng.next_u32() as usize) % count.max(1);
        values.swap(idx, swap);
    }
    values
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

fn homeworld_distances(candidate: [u8; 2], homeworlds: &[[u8; 2]]) -> [f32; 2] {
    let mut distances = homeworlds
        .iter()
        .map(|coords| distance_sq(candidate, *coords).sqrt())
        .collect::<Vec<_>>();
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    [distances[0], *distances.get(1).unwrap_or(&distances[0])]
}

fn distance_sq(a: [u8; 2], b: [u8; 2]) -> f32 {
    let dx = a[0] as f32 - b[0] as f32;
    let dy = a[1] as f32 - b[1] as f32;
    dx * dx + dy * dy
}

fn clamp_coord(value: f32, map_size: u8, margin: f32) -> u8 {
    value
        .round()
        .clamp(margin, (map_size as f32 - 1.0) - margin) as u8
}

fn fallback_homeworld(index: usize, player_count: usize, map_size: u8) -> [u8; 2] {
    let center = (map_size / 2) as i16;
    let radius = (map_size as i16 / 3).max(3);
    let angle = std::f32::consts::TAU * index as f32 / player_count.max(1) as f32;
    [
        ((center as f32 + radius as f32 * angle.cos()).round() as i16).clamp(2, map_size as i16 - 3)
            as u8,
        ((center as f32 + radius as f32 * angle.sin()).round() as i16).clamp(2, map_size as i16 - 3)
            as u8,
    ]
}

fn fallback_local_world(home: [u8; 2], map_size: u8, used: &[[u8; 2]]) -> [u8; 2] {
    for dx in -4..=4 {
        for dy in -4..=4 {
            let x = (home[0] as i16 + dx).clamp(1, map_size as i16 - 2) as u8;
            let y = (home[1] as i16 + dy).clamp(1, map_size as i16 - 2) as u8;
            let candidate = [x, y];
            if !used.contains(&candidate) && candidate != home {
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
                if !used.contains(&candidate) {
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
