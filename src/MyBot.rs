mod hlt;
use hlt::{ networking, types };
use std::collections::HashMap;

struct MoveFeatures {
    d: u8,
    distance: i32,
    friendly: bool,
    strength_us: i32,
    strength_them: i32,
    production_them: i32,
    production_us: i32,
}

//fn get_best_move(loc: types::Location, map: &types::GameMap, my_id: u8) -> u8 {
//    let mut moves = vec![];
//    for d in &types::CARDINALS {
//        let proposed = map.get_site_ref(loc, *d);
//        let current = map.get_site_ref(loc, types::STILL);
//        moves.push(MoveFeatures {
//            d: *d,
//            distance: distance_to_border(loc, *d, &map, my_id),
//            friendly: proposed.owner == my_id,
//            strength_us: current.strength as i32,
//            strength_them: proposed.strength as i32,
//            production_us: current.production as i32,
//            production_them: proposed.production as i32,
//        });
//    }
//    let mut moves: Vec<(u8, i32)> = moves.iter()
//        .map(|mf| (mf.d, score_move(mf)))
//        .collect();
//    moves.push((types::STILL, score_still(loc, &map)));
//    moves.sort_by(|a, b| b.1.cmp(&a.1));
//    moves[0].0
//}

//fn score_still(loc: types::Location, map: &types::GameMap) -> i32 {
//    let current_strength = map.get_site_ref(loc, types::STILL).strength;
//    1100 - current_strength as i32
//}
//
//fn score_move(mf: &MoveFeatures) -> i32 {
//    let combined_strength = mf.strength_us + mf.strength_them;
//    (
//        1_000
//        // Penalize distance
//        - mf.distance * mf.distance
//        // Penalize losing battles
//        - if mf.friendly {0} else { mf.strength_them - mf.strength_us } * 8
//        // Penalize losing due to strength cap
//        - if mf.friendly && combined_strength > 255 {
//                combined_strength - 255
//            } else {
//                0
//            } * 5
//        // Reward gaining territory
//        + if !mf.friendly && mf.strength_us > mf.strength_them { 100 } else {0} * 10
//        // Reward moving to higher production
//        + if mf.production_us < mf.production_them {
//                mf.production_them - mf.production_us
//            } else {0} * 1
//    )
//}

fn get_best_move_simple(loc: types::Location, map: &types::GameMap, my_id: u8) -> u8  {
    let mut moves = vec![];
    for d in &types::CARDINALS {
        let proposed = map.get_site_ref(loc, *d);
        let current = map.get_site_ref(loc, types::STILL);
        moves.push(MoveFeatures {
            d: *d,
            distance: distance_to_border(loc, *d, &map, my_id),
            friendly: proposed.owner == my_id,
            strength_us: current.strength as i32,
            strength_them: proposed.strength as i32,
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
        .filter(|a| a.friendly || a.strength_us > a.strength_them)
        .filter(|a| !a.friendly || a.strength_us + a.strength_them <= 255)
        .collect();
    // If there's more than one move available remove losing battles
    match moves.len() {
        0 => return types::STILL,
        1 => return moves[0].d,
        _ => {},
    };
    // If there's still more than one move available, go for production
    moves.sort_by(|a, b| b.production_them.cmp(&a.production_them));
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

//fn find_poi(map: &types::GameMap, my_id: u8) -> Vec<types::Location> {
//    let mut poi = vec![];
//    for a in 0..map.height {
//        for b in 0..map.width {
//            let l = hlt::types::Location { x: b, y: a };
//            let site = map.get_site_ref(l, types::STILL);
//            if site.owner == my_id {
//                continue // Our own stuff isn't a POI
//            }
//            // Define a POI as a production peak
//            if site.production >= 10 &&
//               site.production >= map.get_site_ref(l, types::NORTH).production &&
//               site.production >= map.get_site_ref(l, types::EAST).production &&
//               site.production >= map.get_site_ref(l, types::SOUTH).production &&
//               site.production >= map.get_site_ref(l, types::WEST).production {
//                poi.push(l);
//            }
//            // TODO: define weak enemy as a POI
//            //if site.strength <= map.get_site_ref(l, types::NORTH).strength &&
//            //   site.strength <= map.get_site_ref(l, types::EAST).strength &&
//            //   site.strength <= map.get_site_ref(l, types::SOUTH).strength &&
//            //   site.strength <= map.get_site_ref(l, types::WEST).strength {
//            //    poi.push(l);
//            //}
//        }
//    }
//    poi
//}
//
//fn get_closest_poi(loc: types::Location, map: &types::GameMap, poi: &Vec<types::Location>) -> u8 {
//    let closest = poi.iter()
//        .min_by_key(|&p| {
//            map.get_distance(loc, *p)
//        })
//        .unwrap_or(&types::Location{x:0, y:0})
//        .clone();
//    let d = map.get_direction(loc, closest);
//    log(format!("{:?} -> {} -> {:?}\n", loc, d, closest));
//    d
//}
//
//use std::io::prelude::*;
//use std::fs::OpenOptions;
//
//fn log(s: String) {
//    let mut file = OpenOptions::new()
//        .append(true)
//        .create(true)
//        .open("output.log")
//        .unwrap();
//    file.write(s.as_bytes()).unwrap();
//}

fn main() {

    let (my_id, mut game_map) = networking::get_init();
    networking::send_init(format!("{}{}", "Asp2Insp".to_string(), my_id.to_string()));
    loop {
        networking::get_frame(&mut game_map);
        //let poi = find_poi(&game_map, my_id);
        let mut moves = HashMap::new();
        for a in 0..game_map.height {
            for b in 0..game_map.width {
                let l = hlt::types::Location { x: b, y: a };
                let site = game_map.get_site_ref(l, types::STILL);
                let threshold = 5 * site.production;
                match (site.owner, site.strength) {
                    (id, strength) if strength < threshold && my_id == id => {
                        moves.insert(l, types::STILL);
                    },
                    (id, _) if my_id == id => {
                        //let d = get_best_move(l, &game_map, my_id);
                        let d = get_best_move_simple(l, &game_map, my_id);
                        //let d = get_closest_poi(l, &game_map, &poi);
                        moves.insert(l, d);
                    },
                    (_, _) => {},
                };
            }
        }
        networking::send_frame(moves);
    }
}
