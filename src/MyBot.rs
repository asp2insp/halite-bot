#![allow(warnings)]

mod hlt;
use hlt::{ networking, types };
use std::collections::HashMap;

struct MoveFeatures {
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
    current_moves: &HashMap<types::Location, i32>) -> u8  {
    let mut moves = vec![];
    for d in &types::CARDINALS {
        let proposed_loc = map.get_location(loc, *d);
        let proposed = map.get_site_ref(loc, *d);
        let current = map.get_site_ref(loc, types::STILL);
        let already_assigned_strength: i32 = *current_moves.get(&proposed_loc)
            .unwrap_or(&0i32);
        moves.push(MoveFeatures {
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
        .filter(|a| a.distance == shortest)
        // Don't allow losing battles
        .filter(|a| a.adjacent_strength_us > (a.strength_them + a.production_them))
        // Allow a small loss so full strength don't get stuck
        .filter(|a| !a.friendly || a.strength_us + a.strength_them <= 260)
        // Don't allow too many troops to move into the same space
        .filter(|a| a.strength_us + a.assigned_strength <= 260)
        .collect();
    match moves.len() {
        0 => return types::STILL,
        1 => return moves[0].d,
        _ => {},
    };
    // If there's still more than one move available, go for production
    moves.sort_by(|a, b| b.production_them.cmp(&a.production_them));
    // Prefer losing less strength
    moves.sort_by(|a, b| a.strength_them.cmp(&b.strength_them));
    // Prefer capturing enemy to neutral
    //moves.sort_by(|a, b| b.owner_them.cmp(&a.owner_them));
    return moves[0].d
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

fn main() {

    let (my_id, mut game_map) = networking::get_init();
    networking::send_init(format!("{}{}", "Asp2Insp".to_string(), my_id.to_string()));
    loop {
        networking::get_frame(&mut game_map);
        //let poi = find_poi(&game_map, my_id);
        let mut moves = HashMap::new();
        let mut planned_moves: HashMap<types::Location, i32> = HashMap::new();
        for a in 0..game_map.height {
            for b in 0..game_map.width {
                let l = types::Location { x: b, y: a };
                let site = game_map.get_site_ref(l, types::STILL);
                let threshold = 5 * site.production;
                match (site.owner, site.strength) {
                    (id, strength) if strength < threshold && my_id == id => {
                        moves.insert(l, types::STILL);
                    },
                    (id, _) if my_id == id => {
                        let d = get_best_move_simple(l, &game_map, my_id, &planned_moves);
                        let next_loc = game_map.get_location(l, d);
                        // Record the planned move
                        let strength = site.strength as i32;
                        *planned_moves.entry(l).or_insert(strength) -= strength;
                        *planned_moves.entry(next_loc).or_insert(0) += strength;

                        moves.insert(l, d);
                    },
                    (_, _) => {},
                };
            }
        }
        networking::send_frame(moves);
    }
}
