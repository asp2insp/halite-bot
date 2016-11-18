#![allow(warnings)]

mod hlt;
use hlt::networking;
use hlt::types::*;
use std::collections::{HashMap, HashSet};

struct MoveFeatures {
    loc: Location,
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

fn get_total_adjacent_strength(loc: Location, map: &GameMap, my_id: u8) -> i32 {
    // Check to see if we can use multiple moves to capture
    let total_adjacent_strength: i32 = CARDINALS.iter()
        .map(|d| {
            let site = map.get_site_ref(loc, *d);
            if site.owner == my_id { site.strength as i32 } else { 0 }
        })
        .sum();
    total_adjacent_strength
}

fn get_best_move_simple(
    loc: Location,
    map: &GameMap,
    my_id: u8) -> u8  {
    let mut moves = vec![];
    for d in &CARDINALS {
        let proposed_loc = map.get_location(loc, *d);
        let proposed = map.get_site_ref(loc, *d);
        let current = map.get_site_ref(loc, STILL);
        let already_assigned_strength: i32 = 0;
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
        .filter(|a| a.strength_us > (a.strength_them + a.production_them))
        // Allow a small loss so full strength don't get stuck
        .filter(|a| !a.friendly || a.strength_us + a.strength_them <= 260)
        // Don't allow too many troops to move into the same space
        .filter(|a| a.strength_us + a.assigned_strength <= 260)
        .collect();
    if moves.len() == 0 {
        return STILL
    }
    // If there's still more than one move available, go for production
    moves.sort_by(|a, b| a.production_them.cmp(&b.production_them));
    // Prefer losing less strength
    moves.sort_by(|a, b| b.strength_them.cmp(&a.strength_them));

    let m = moves.pop().unwrap();
    m.d
}

fn distance_to_border(loc: Location, dir: u8, map: &GameMap, my_id: u8) -> i32 {
    let mut l = loc.clone();
    let mut counter = 0;
    loop {
        if map.get_site_ref(l, STILL).owner != my_id {
            return counter
        }
        l = map.get_location(l, dir);
        if l == loc {
            return map.width as i32
        }
        counter += 1;
    }
}

fn get_units_of_player(id: u8, map: &GameMap) -> Vec<Location> {
    let mut result = Vec::new();
    for a in 0..map.height {
        for b in 0..map.width {
            let l = Location { x: b, y: a };
            let site = map.get_site_ref(l, STILL);
            if site.owner == id {
                result.push(l);
            }
        }
    }
    result
}

fn max_capture_strategy(game_map: &GameMap, my_id: u8) -> HashMap<Location, u8> {
    let my_units = get_units_of_player(my_id, &game_map);
    let mut possibilities = my_units
        .iter()
        .flat_map(|l| {
            CARDINALS.iter()
                .cloned()
                .map(move |d| (*l, d))
                .collect::<Vec<_>>()
        })
        .map(|(l, d)| {
            let proposed = game_map.get_site_ref(l, d);
            let current = game_map.get_site_ref(l, STILL);
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
            for adj in &CARDINALS {
                let adj_loc = game_map.get_location(m.loc, *adj);
                if !moves.contains_key(&adj_loc) {
                    moves.insert(adj_loc, reverse(adj));
                }
            }
            moves.insert(m.loc, STILL);
        }
    }
    for remaining in my_units {
        if !moves.contains_key(&remaining) {
            moves.insert(remaining, STILL);
        }
    }
    moves
}

fn reverse(d: &u8) -> u8 {
    match d {
        &NORTH => SOUTH,
        &SOUTH => NORTH,
        &WEST => EAST,
        &EAST => WEST,
        _ => panic!("unknown direction {}", d),
    }
}

fn find_poi(map: &GameMap, my_id: u8) -> Vec<Location> {
    let mut total = 0f32;
    for a in 0..map.height {
        for b in 0..map.width {
            let l = Location { x: b, y: a };
            let site = map.get_site_ref(l, STILL);
            total += site.production as f32;
        }
    }
    let avg_production = total / (map.height as f32 * map.width as f32);
    let avg_production = avg_production as u8;
    let mut result = Vec::new();
    for a in 0..map.height {
        for b in 0..map.width {
            let l = Location { x: b, y: a };
            let site = map.get_site_ref(l, STILL);
            if site.production >= avg_production * 2 && site.owner != my_id {
                result.push(l);
            }
        }
    }
    result
}

fn find_closest_poi(l: Location, map: &GameMap, poi: &Vec<Location>) -> Location {
    poi.iter()
    .min_by_key(|p| map.get_distance(l, **p)).unwrap_or(&Location{x: 0, y: 0}).clone()
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

#[derive(Copy, Clone)]
enum Troop {
    Interior(Location), // Surrounded by at least 1 square of friendly
    VerticalWall(Location), // A straight line along the enemy
    HorizontalWall(Location), // A straight line along the enemy
    Pincer(Location, Location, Location), // Two troops that can corner against an enemy
    Pincer3(Location, Location, Location, Location), // Three troops that are almost surrounding an enemy
    Lance(Location), // Surrounded on three sides by enemy
    Island(Location), // Surrounded on all 8 sides by enemy
    Reinforcement(Location), // Surrounded by friendly, but with a diagonal enemy
    Unknown(Location),
}

fn troop_strategy(map: &GameMap, my_id: u8) -> HashMap<Location, u8> {
    use Troop::*;
    let friendly = |l| { if map.get_site_ref(l, STILL).owner == my_id {1} else {0} };
    let my_units = get_units_of_player(my_id, map);
    let troops = classify(my_units, map, my_id);
    let mut moves = HashMap::new();
    let poi = find_poi(map, my_id);
    let mut assigned_strength: HashMap<Location, usize> = HashMap::new();
    let mut commit_move = |moves: &mut HashMap<Location, u8>, l, d| {
        let proposed = map.get_site_ref(l, d);
        let proposed_loc = map.get_location(l, d);
        let strength = map.get_site_ref(l, STILL).strength;
        if proposed.owner == my_id && strength as u16 + proposed.strength as u16 > 260u16 {
            moves.insert(l, STILL);
        } else if proposed.owner == my_id && strength as usize + *assigned_strength.entry(proposed_loc).or_insert(0) > 260 {
            moves.insert(l, STILL);
        } else {
            *assigned_strength.entry(proposed_loc).or_insert(0) += strength as usize;
            *assigned_strength.entry(l).or_insert(strength as usize) -= strength as usize;
            moves.insert(l, d);
        }
    };
    for t in troops {
        match t {
            Interior(l) => {
                //moves.insert(l, get_best_move_simple(l, map, my_id));
                let site = map.get_site_ref(l, STILL);
                if site.strength < site.production * 5 {
                    commit_move(&mut moves, l, STILL);
                } else {
                    let closest = find_closest_poi(l, map, &poi);
                    let d = map.get_direction(l, closest);
                    let proposed = map.get_site_ref(l, d);
                    if site.strength as u16 + proposed.strength as u16 > 260u16  {
                        commit_move(&mut moves, l, STILL);
                    } else {
                        commit_move(&mut moves, l, d);
                    }
                }
            },
            VerticalWall(l) => {
                let site = map.get_site_ref(l, STILL);
                let left = map.get_site_ref(l, WEST);
                let right = map.get_site_ref(l, EAST);
                if left.owner != my_id && site.strength > left.strength {
                    commit_move(&mut moves, l, WEST);
                } else if right.owner != my_id && site.strength > right.strength {
                    commit_move(&mut moves, l, EAST);
                } else if site.strength < site.production * 5 {
                    commit_move(&mut moves, l, STILL);
                } else {
                    let up = map.get_site_ref(l, NORTH);
                    let down = map.get_site_ref(l, SOUTH);
                    if up.strength > site.strength {
                        commit_move(&mut moves, l, NORTH);
                    } else if down.strength > site.strength {
                        commit_move(&mut moves, l, SOUTH);
                    } else {
                        commit_move(&mut moves, l, STILL);
                    }
                }
            },
            HorizontalWall(l) => {
                let site = map.get_site_ref(l, STILL);
                let up = map.get_site_ref(l, NORTH);
                let down = map.get_site_ref(l, SOUTH);
                if up.owner != my_id && site.strength > up.strength {
                    commit_move(&mut moves, l, NORTH);
                } else if down.owner != my_id && site.strength > down.strength {
                    commit_move(&mut moves, l, SOUTH);
                } else if site.strength < site.production * 5 {
                    commit_move(&mut moves, l, STILL);
                } else {
                    let left = map.get_site_ref(l, WEST);
                    let right = map.get_site_ref(l, EAST);
                    if left.strength > site.strength {
                        commit_move(&mut moves, l, WEST);
                    } else if right.strength > site.strength {
                        commit_move(&mut moves, l, EAST);
                    } else {
                        commit_move(&mut moves, l, STILL);
                    }
                }
            },
            Pincer(l1, l2, e) => {
                let site1 = map.get_site_ref(l1, STILL);
                let site2 = map.get_site_ref(l2, STILL);
                let enemy = map.get_site_ref(e, STILL);
                if site1.strength + site2.strength > enemy.strength {
                    commit_move(&mut moves, l1, map.get_direction(l1, e));
                    commit_move(&mut moves, l2, map.get_direction(l2, e));
                } else {
                    commit_move(&mut moves, l1, STILL);
                    commit_move(&mut moves, l2, STILL);
                }
            },
            Pincer3(l1, l2, l3, e) => {
                let site1 = map.get_site_ref(l1, STILL);
                let site2 = map.get_site_ref(l2, STILL);
                let site3 = map.get_site_ref(l3, STILL);
                let enemy = map.get_site_ref(e, STILL);
                if site1.strength + site2.strength > enemy.strength {
                    commit_move(&mut moves, l1, map.get_direction(l1, e));
                    commit_move(&mut moves, l2, map.get_direction(l2, e));
                } else if site3.strength + site2.strength > enemy.strength {
                    commit_move(&mut moves, l3, map.get_direction(l3, e));
                    commit_move(&mut moves, l2, map.get_direction(l2, e));
                } else if site3.strength + site2.strength + site1.strength > enemy.strength {
                    commit_move(&mut moves, l1, map.get_direction(l1, e));
                    commit_move(&mut moves, l2, map.get_direction(l2, e));
                    commit_move(&mut moves, l3, map.get_direction(l3, e));
                } else {
                    commit_move(&mut moves, l1, STILL);
                    commit_move(&mut moves, l2, STILL);
                    commit_move(&mut moves, l3, STILL);
                }
            },
            Lance(l) => {
                let site = map.get_site_ref(l, STILL);
                for d in &CARDINALS {
                    let enemy = map.get_site_ref(l, *d);
                    if enemy.owner != my_id && site.strength > enemy.strength {
                        commit_move(&mut moves, l, *d);
                        break;
                    }
                }
            },
            Island(l) => {
                let site = map.get_site_ref(l, STILL);
                for d in &CARDINALS {
                    let enemy = map.get_site_ref(l, *d);
                    if site.strength > enemy.strength {
                        commit_move(&mut moves, l, *d);
                        break;
                    }
                }
            },
            Reinforcement(l) => {
                commit_move(&mut moves, l, get_best_move_simple(l, map, my_id));
            },
            Unknown(l) => {
                commit_move(&mut moves, l, get_best_move_simple(l, map, my_id));
            },
        }
    }
    moves
}

fn classify(locs: Vec<Location>, map: &GameMap, my_id: u8) -> Vec<Troop> {
    let mut done: HashSet<Location> = HashSet::new();
    let mut result = Vec::new();
    for l in locs {
        if done.contains(&l) {
            continue
        }
        let t = classify_loc(l, map, my_id);
        use Troop::*;
        match t {
            Pincer(l1, l2, e) => {
                done.insert(l1); done.insert(l2);
            },
            Pincer3(l1, l2, l3, e) => {
                done.insert(l1); done.insert(l2); done.insert(l3);
            },
            _ => {
                done.insert(l);
            },
        };
        result.push(t);
    }
    result
}

fn classify_loc(loc: Location, map: &GameMap, my_id: u8) -> Troop {
    use Troop::*;
    let owner = |l| { map.get_site_ref(l, STILL).owner };
    let strength = |l| { map.get_site_ref(l, STILL).strength };
    let production = |l| { map.get_site_ref(l, STILL).production };
    let get_location = |l, d1, d2| {
        map.get_location(map.get_location(l, d1), d2)
    };
    let friendly = |l| { if map.get_site_ref(l, STILL).owner == my_id {1} else {0} };

    // nw nn ne
    // ww    ee
    // sw ss se
    let (nw, nn, ne) = (get_location(loc, WEST, NORTH), map.get_location(loc, NORTH), get_location(loc, EAST, NORTH));
    let (ww, ee) = (map.get_location(loc, WEST), map.get_location(loc, EAST));
    let (sw, ss, se) = (get_location(loc, WEST, SOUTH), map.get_location(loc, SOUTH), get_location(loc, EAST, SOUTH));

    let surroundings = (friendly(nw), friendly(nn), friendly(ne),
                       friendly(ww),               friendly(ee),
                       friendly(sw), friendly(ss), friendly(se));
    match surroundings {
        (1, 1, 1,
         1,    1,
         1, 1, 1) => Interior(loc),
        (_, 1, _,
         1,    1,
         _, 1, _) => Reinforcement(loc),
        (_, 0, _,
         0,    0,
         _, 0, _) => Island(loc),
        (_, a, _,
         d,    b,
         _, c, _) if a + b + c + d == 1 => Lance(loc),
        (_, 1, _,
         a,    b,
         _, 1, _) if a + b < 2 => VerticalWall(loc),
        (_, a, _,
         1,    1,
         _, b, _) if a + b < 2 => HorizontalWall(loc),
        (1, 0, 1,
         _,    _,
         _, _, _) => Pincer3(nw, loc, ne, nn),
        (1, _, _,
         0,    _,
         1, _, _) => Pincer3(nw, loc, sw, ww),
        (_, _, 1,
         _,    0,
         _, _, 1) => Pincer3(ne, loc, se, ee),
        (_, _, _,
         _,    _,
         1, 0, 1) => Pincer3(sw, loc, se, ss),

        (1, 0, _,
         _,    _,
         _, _, _) => Pincer(nw, loc, nn),
        (_, _, _,
         0,    _,
         1, _, _) => Pincer(loc, sw, ww),
        (_, _, 1,
         _,    0,
         _, _, _) => Pincer(ne, loc, ee),
        (_, _, _,
         _,    _,
         1, 0, _) => Pincer(sw, loc, ss),
        (_, 0, 1,
         _,    _,
         _, _, _) => Pincer(loc, ne, nn),
        (1, _, _,
         0,    _,
         _, _, _) => Pincer(nw, loc, ww),
        (_, _, _,
         _,    0,
         _, _, 1) => Pincer(loc, se, ee),
        (_, _, _,
         _,    _,
         _, 0, 1) => Pincer(loc, se, ss),
        _ => Unknown(loc),
    }
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
            max_capture_strategy(&game_map, my_id)
        } else {
            troop_strategy(&game_map, my_id)
        };
        networking::send_frame(moves);
        turn_counter += 1;
    }
}
