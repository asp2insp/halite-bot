#![allow(warnings)]

mod hlt;
use hlt::{ networking, types };
use std::collections::HashMap;

struct MoveFeatures {
    loc: types::Location,
    d: u8,
    owner_them: u8,
    distance: i32,
    friendly: bool,
    strength_us: i32,
    strength_them: i32,
    adjacent_strength_us: i32,
    assigned_strength: i32,
    production_them: i32,
    production_us: i32,
}

fn get_total_adjacent_strength(loc: types::Location, map: &types::GameMap, my_id: u8) -> i32 {
    // Check to see if we can use multiple moves to capture
    let total_adjacent_strength: i32 = types::CARDINALS.iter()
        .map(|d| {
            let site = map.get_site_ref(loc, *d);
            if site.owner == my_id { site.strength as i32 } else { 0 }
        })
        .sum();
    total_adjacent_strength
}

fn get_best_move_simple(
    loc: types::Location,
    map: &types::GameMap,
    my_id: u8,
    current_moves: &HashMap<types::Location, i32>) -> Vec<(types::Location, u8)>  {
    let mut moves = vec![];
    for d in &types::CARDINALS {
        let proposed_loc = map.get_location(loc, *d);
        let proposed = map.get_site_ref(loc, *d);
        let current = map.get_site_ref(loc, types::STILL);
        let already_assigned_strength: i32 = *current_moves.get(&proposed_loc)
            .unwrap_or(&0i32);
        moves.push(MoveFeatures {
            loc: loc,
            d: *d,
            distance: distance_to_border(loc, *d, &map, my_id),
            owner_them: proposed.owner,
            friendly: proposed.owner == my_id,
            strength_us: current.strength as i32,
            strength_them: proposed.strength as i32,
            adjacent_strength_us: get_total_adjacent_strength(proposed_loc, &map, my_id),
            assigned_strength: already_assigned_strength,
            production_us: current.production as i32,
            production_them: proposed.production as i32,
        });
    }
    // Sort by distance
    moves.sort_by(|a, b| a.distance.cmp(&b.distance));
    // Remove all but the shortest distances and filter losing battles and strength losses
    let shortest = moves[0].distance;
    let mut moves: Vec<MoveFeatures> = moves.into_iter()
        // Don't move weak pieces
        .filter(|a| a.strength_us > a.production_us * 5)
        // Only move towards the closest border
        .filter(|a| a.distance == shortest)
        // Don't allow losing battles
        .filter(|a| a.adjacent_strength_us > (a.strength_them + a.production_them))
        // Allow a small loss so full strength don't get stuck
        .filter(|a| !a.friendly || a.strength_us + a.strength_them <= 260)
        // Don't allow too many troops to move into the same space
        .filter(|a| a.strength_us + a.assigned_strength <= 260)
        .collect();
    match moves.len() {
        0 => return vec![(loc, types::STILL)],
        _ => {},
    };
    // If there's still more than one move available, go for production
    moves.sort_by(|a, b| a.production_them.cmp(&b.production_them));
    // Prefer losing less strength
    moves.sort_by(|a, b| b.strength_them.cmp(&a.strength_them));

    let m = moves.pop().unwrap();
    if m.strength_us > m.strength_them {
        vec![(m.loc, m.d)]
    } else {
        // We need to issue additional orders for other troups
        let mut r = vec![(m.loc, m.d)];
        let proposed = map.get_location(m.loc, m.d);
        for adj in &types::CARDINALS {
            let adj_loc = map.get_location(proposed, *adj);
            r.push((adj_loc, reverse(adj)));
        }
        r
    }
}

fn distance_to_border(loc: types::Location, dir: u8, map: &types::GameMap, my_id: u8) -> i32 {
    let mut l = loc.clone();
    let mut counter = 0;
    loop {
        if map.get_site_ref(l, types::STILL).owner != my_id {
            return counter
        }
        l = map.get_location(l, dir);
        if l == loc {
            return map.width as i32
        }
        counter += 1;
    }
}

fn get_units_of_player(id: u8, map: &types::GameMap) -> Vec<types::Location> {
    let mut result = Vec::new();
    for a in 0..map.height {
        for b in 0..map.width {
            let l = types::Location { x: b, y: a };
            let site = map.get_site_ref(l, types::STILL);
            if site.owner == id {
                result.push(l);
            }
        }
    }
    result
}

fn local_best_strategy(my_id: u8, game_map: &types::GameMap) -> HashMap<types::Location, u8> {
    let mut moves = HashMap::new();
    let mut planned_moves: HashMap<types::Location, i32> = HashMap::new();
    for l in get_units_of_player(my_id, game_map) {
        if moves.contains_key(&l) {
            continue;
        }

        let next = get_best_move_simple(l, game_map, my_id, &planned_moves);
        for (loc, d) in next {
            let site = game_map.get_site_ref(loc, types::STILL);
            let next_loc = game_map.get_location(loc, d);

            // Record the planned move
            let strength = site.strength as i32;
            *planned_moves.entry(loc).or_insert(strength) -= strength;
            *planned_moves.entry(next_loc).or_insert(0) += strength;

            moves.insert(loc, d);
        }
    }
    moves
}

fn max_capture_strategy(my_id: u8, game_map: &types::GameMap) -> HashMap<types::Location, u8> {
    let my_units = get_units_of_player(my_id, &game_map);
    let mut possibilities = my_units
        .iter()
        .flat_map(|l| {
            types::CARDINALS.iter()
                .cloned()
                .map(move |d| (*l, d))
                .collect::<Vec<_>>()
        })
        .map(|(l, d)| {
            let proposed = game_map.get_site_ref(l, d);
            let current = game_map.get_site_ref(l, types::STILL);
            MoveFeatures {
                loc: l,
                d: d,
                distance: 1,
                owner_them: proposed.owner,
                friendly: proposed.owner == my_id,
                strength_us: current.strength as i32,
                strength_them: proposed.strength as i32,
                adjacent_strength_us: get_total_adjacent_strength(l, &game_map, my_id),
                assigned_strength: 0,
                production_us: current.production as i32,
                production_them: proposed.production as i32,
            }
        })
        // Don't move weak pieces
        .filter(|a| a.strength_us > a.production_us * 3)
        // Only consider moves that move us toward victory!
        .filter(|a| !a.friendly)
        .filter(|a| a.adjacent_strength_us + a.strength_us > a.strength_them)
        .collect::<Vec<_>>();
    possibilities.sort_by(|a, b| b.strength_them.cmp(&a.strength_them));
    possibilities.sort_by(|a, b| a.strength_us.cmp(&b.strength_us));
    possibilities.sort_by(|a, b| a.production_them.cmp(&b.production_them));

    let mut moves = HashMap::new();
    while possibilities.len() > 0 {
        let m = possibilities.pop().unwrap();
        if moves.contains_key(&m.loc) {
            continue
        }
        if m.strength_us > m.strength_them {
            // If we can capture, do so
            moves.insert(m.loc, m.d);
        } else {
            // Otherwise, move everything towards that point
            for adj in &types::CARDINALS {
                let adj_loc = game_map.get_location(m.loc, *adj);
                if !moves.contains_key(&adj_loc) {
                    moves.insert(adj_loc, reverse(adj));
                }
            }
            moves.insert(m.loc, types::STILL);
        }
    }
    for remaining in my_units {
        if !moves.contains_key(&remaining) {
            moves.insert(remaining, types::STILL);
        }
    }
    moves
}

fn reverse(d: &u8) -> u8 {
    match d {
        &types::NORTH => types::SOUTH,
        &types::SOUTH => types::NORTH,
        &types::WEST => types::EAST,
        &types::EAST => types::WEST,
        _ => panic!("unknown direction {}", d),
    }
}

fn find_poi(map: &types::GameMap, my_id: u8) -> Vec<types::Location> {
    let mut result = Vec::new();
    for a in 0..map.height {
        for b in 0..map.width {
            let l = types::Location { x: b, y: a };
            let site = map.get_site_ref(l, types::STILL);
            if site.production >= 5 && site.owner != my_id {
                result.push(l);
            }
        }
    }
    result
}

fn find_closest_poi(l: types::Location, map: &types::GameMap, poi: &Vec<types::Location>) -> types::Location {
    poi.iter()
    .min_by_key(|p| map.get_distance(l, **p)).unwrap_or(&types::Location{x: 0, y: 0}).clone()
}

fn poi_strategy(my_id: u8, game_map: &types::GameMap) -> HashMap<types::Location, u8> {
    let poi = find_poi(game_map, my_id);
    let my_units = get_units_of_player(my_id, &game_map);
    let unit_count = my_units.len();
    let mut moves = HashMap::new();
    let mut targets: HashMap<types::Location, ()> = HashMap::new();
    for l in my_units {
        let site = game_map.get_site_ref(l, types::STILL);
        if site.strength < site.production * 5 {
            moves.insert(l, types::STILL);
        } else {
            let closest = find_closest_poi(l, game_map, &poi);
            let d = game_map.get_direction(l, closest);
            let proposed_loc = game_map.get_location(l, d);
            if site.strength > game_map.get_site_ref(proposed_loc, types::STILL).strength {
                moves.insert(l, d);
            }
            targets.insert(closest, ());
        }
    }
    //log(
    //    format!("{} POI, {} units, {} distinct targets\n", poi.len(), unit_count, targets.len()),
    //    my_id
    //);
    moves
}

use std::io::prelude::*;
use std::fs::OpenOptions;
fn log(s: String, id: u8) {
    let mut file = OpenOptions::new()
         .append(true)
         .create(true)
         .open(format!("output-{}.log", id))
         .unwrap();
    file.write(s.as_bytes()).unwrap();
}

fn main() {
    let (my_id, mut game_map) = networking::get_init();
    networking::send_init(format!("{}{}", "Asp2Insp".to_string(), my_id.to_string()));
    let mut turn_counter = 0;
    loop {
        networking::get_frame(&mut game_map);
        //let moves = poi_strategy(my_id, &game_map);

        let my_count = get_units_of_player(my_id, &game_map).len();
        let moves = if my_count < 20 {
            max_capture_strategy(my_id, &game_map)
        } else {
            local_best_strategy(my_id, &game_map)
        };
        networking::send_frame(moves);
        turn_counter += 1;
    }
}
