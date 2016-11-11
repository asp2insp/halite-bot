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

fn get_best_move(loc: types::Location, map: &types::GameMap, my_id: u8) -> u8 {
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
    let mut moves: Vec<(u8, i32)> = moves.iter()
        .map(|mf| (mf.d, score_move(mf)))
        .collect();
    moves.push((types::STILL, score_still(loc, &map)));
    moves.sort_by(|a, b| b.1.cmp(&a.1));
    moves[0].0
}

fn score_still(loc: types::Location, map: &types::GameMap) -> i32 {
    let current_strength = map.get_site_ref(loc, types::STILL).strength;
    1000 - current_strength as i32
}

fn score_move(mf: &MoveFeatures) -> i32 {
    let combined_strength = mf.strength_us + mf.strength_them;
    (
        1_000
        // Penalize distance
        - mf.distance * 5
        // Penalize losing battles
        - if mf.friendly {0} else { mf.strength_them - mf.strength_us } * 8
        // Penalize losing due to strength cap
        - if mf.friendly && combined_strength > 255 {
                combined_strength - 255
            } else {
                0
            } * 5
        // Reward gaining territory
        + if !mf.friendly && mf.strength_us > mf.strength_them { 100 } else {0} * 10
        // Reward moving to higher production
        + if mf.production_us < mf.production_them {
                mf.production_them - mf.production_us
            } else {0} * 1
    )
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
                        let d = get_best_move(l, &game_map, my_id);
                        moves.insert(l, d);
                    },
                    (_, _) => {},
                };
            }
        }
        networking::send_frame(moves);
    }
}
