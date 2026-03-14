use crate::{CoreGameData, DatabaseDat, map_size_for_player_count};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStarmapWorld {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_owner_empire_name: Option<String>,
    pub known_potential_production: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_ground_batteries: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStarmapProjection {
    pub map_width: u8,
    pub map_height: u8,
    pub year: u16,
    pub viewer_empire_id: u8,
    pub worlds: Vec<PlayerStarmapWorld>,
}

impl PlayerStarmapProjection {
    pub fn render_ascii_map(&self) -> String {
        let height = self.map_height as usize;
        let width = self.map_width as usize;
        let mut occupied = vec![vec![false; width + 1]; height + 1];
        for world in &self.worlds {
            let x = world.coords[0] as usize;
            let y = world.coords[1] as usize;
            if x <= width && y <= height {
                occupied[y][x] = true;
            }
        }

        let mut pages = Vec::new();
        for start_x in (1..=width).step_by(18) {
            let end_x = usize::min(start_x + 17, width);
            let mut rows = Vec::new();
            rows.push(render_header_row(start_x, end_x));
            rows.push(render_border_row(start_x, end_x, true));

            for y in (1..=height).rev() {
                let mut row = format!("{y:>2} |");
                for x in start_x..=end_x {
                    let ch = if occupied[y][x] { '*' } else { ' ' };
                    row.push_str(&format!(" {ch}  "));
                }
                row.push_str(&format!("| {y:>2}"));
                rows.push(row);
                rows.push(render_border_row(start_x, end_x, y == 1));
            }
            rows.push(render_header_row(start_x, end_x));
            pages.push(rows.join("\n"));
        }

        pages.join("\n\n\x0c\n")
    }

    pub fn render_ascii_export(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "ESTERIAN CONQUEST STARMAP  YEAR {}  EMPIRE {}\n\n",
            self.year, self.viewer_empire_id
        ));
        out.push_str(&self.render_ascii_map());
        let mut worlds = self.worlds.iter().collect::<Vec<_>>();
        worlds.sort_by_key(|world| (world.coords[1], world.coords[0]));
        let known_worlds = worlds
            .into_iter()
            .filter(|world| {
                world.known_name.is_some()
                    || world.known_owner_empire_name.is_some()
                    || world.known_potential_production.is_some()
                    || world.known_armies.is_some()
                    || world.known_ground_batteries.is_some()
            })
            .collect::<Vec<_>>();

        if !known_worlds.is_empty() {
            out.push_str("\n\nKNOWN WORLD DETAILS\n");
            out.push_str("-------------------\n");
        }

        for world in known_worlds {
            let known_name = world.known_name.as_deref().unwrap_or("UNKNOWN");
            let owner = world
                .known_owner_empire_name
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| "UNKNOWN".to_string());
            let prod = world
                .known_potential_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let armies = world
                .known_armies
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let batteries = world
                .known_ground_batteries
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());

            out.push_str(&format!(
                "({:>2},{:>2})  {:<14}  OWNER {:<20}  PROD {:>3}  ARM {:>3}  BAT {:>3}\n",
                world.coords[0], world.coords[1], known_name, owner, prod, armies, batteries
            ));
        }

        out
    }

    pub fn render_csv_export(&self) -> String {
        let width = self.map_width as usize;
        let height = self.map_height as usize;
        let mut occupied = vec![vec![false; width + 1]; height + 1];
        for world in &self.worlds {
            let x = world.coords[0] as usize;
            let y = world.coords[1] as usize;
            if x <= width && y <= height {
                occupied[y][x] = true;
            }
        }

        let mut out = String::new();
        for start_x in (1..=width).step_by(18) {
            let end_x = usize::min(start_x + 17, width);
            out.push(',');
            for x in start_x..=end_x {
                out.push_str(&x.to_string());
                if x != end_x {
                    out.push(',');
                }
            }
            out.push('\n');

            for y in (1..=height).rev() {
                out.push_str(&y.to_string());
                out.push(',');
                for x in start_x..=end_x {
                    let cell = if occupied[y][x] { "*" } else { "" };
                    out.push_str(cell);
                    if x != end_x {
                        out.push(',');
                    }
                }
                out.push('\n');
            }

            if end_x != width {
                out.push('\n');
            }
        }

        out
    }

    pub fn render_csv_details_export(&self) -> String {
        let mut out =
            String::from("x,y,known_name,known_owner_empire_id,known_owner_empire_name,known_potential_production,known_armies,known_ground_batteries\n");
        let mut worlds = self.worlds.iter().collect::<Vec<_>>();
        worlds.sort_by_key(|world| (world.coords[1], world.coords[0]));
        for world in worlds.into_iter().filter(|world| {
            world.known_name.is_some()
                || world.known_owner_empire_name.is_some()
                || world.known_potential_production.is_some()
                || world.known_armies.is_some()
                || world.known_ground_batteries.is_some()
        }) {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                world.coords[0],
                world.coords[1],
                csv_field(world.known_name.as_deref()),
                world
                    .known_owner_empire_id
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                csv_field(world.known_owner_empire_name.as_deref()),
                world
                    .known_potential_production
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                world.known_armies.map(|value| value.to_string()).unwrap_or_default(),
                world
                    .known_ground_batteries
                    .map(|value| value.to_string())
                    .unwrap_or_default()
            ));
        }
        out
    }
}

fn render_header_row(start_x: usize, end_x: usize) -> String {
    let mut row = String::from("   ");
    for x in start_x..=end_x {
        row.push_str(&format!("{x:>4}"));
    }
    row
}

fn render_border_row(start_x: usize, end_x: usize, final_row: bool) -> String {
    let mut row = String::from("  -|");
    for _x in start_x..=end_x {
        if final_row {
            row.push_str("---|");
        } else {
            row.push_str("- -|");
        }
    }
    row
}

pub fn build_player_starmap_projection(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    viewer_empire_id: u8,
) -> PlayerStarmapProjection {
    let map_size = map_size_for_player_count(game_data.conquest.player_count());
    let planet_count = game_data.planets.records.len();
    let viewer_index = viewer_empire_id.saturating_sub(1) as usize;
    let worlds = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .map(|(planet_index, planet)| {
            let db_record = database.record(planet_index, viewer_index, planet_count);
            let known_name = decode_known_name(db_record);
            let actual_owner_empire_id = planet.owner_empire_slot_raw();
            let known_owner_empire_id = if actual_owner_empire_id == viewer_empire_id {
                Some(viewer_empire_id)
            } else {
                decode_known_owner_empire_id(db_record, game_data)
            };
            let known_owner_empire_name = known_owner_empire_id.map(|empire_id| {
                game_data.player.records[empire_id as usize - 1].controlled_empire_name_summary()
            });

            PlayerStarmapWorld {
                planet_record_index_1_based: planet_index + 1,
                coords: planet.coords_raw(),
                known_name,
                known_owner_empire_id,
                known_owner_empire_name,
                known_potential_production: decode_known_u16(db_record.raw[0x1c]),
                known_armies: decode_known_u8(db_record.raw[0x23]),
                known_ground_batteries: decode_known_u8(db_record.raw[0x25]),
            }
        })
        .collect();

    PlayerStarmapProjection {
        map_width: map_size,
        map_height: map_size,
        year: game_data.conquest.game_year(),
        viewer_empire_id,
        worlds,
    }
}

fn decode_known_name(record: &crate::DatabaseRecord) -> Option<String> {
    let name = String::from_utf8_lossy(record.planet_name_bytes()).trim().to_string();
    if name.is_empty() || name.eq_ignore_ascii_case("unknown") {
        None
    } else {
        Some(name)
    }
}

fn decode_known_owner_empire_id(record: &crate::DatabaseRecord, game_data: &CoreGameData) -> Option<u8> {
    let raw = record.raw[0x15];
    if raw >= 1 && raw <= game_data.conquest.player_count() {
        Some(raw)
    } else {
        None
    }
}

fn decode_known_u16(raw: u8) -> Option<u16> {
    if raw == 0 || raw == 0xff {
        None
    } else {
        Some(raw as u16)
    }
}

fn decode_known_u8(raw: u8) -> Option<u8> {
    if raw == 0 || raw == 0xff {
        None
    } else {
        Some(raw)
    }
}

fn csv_field(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
