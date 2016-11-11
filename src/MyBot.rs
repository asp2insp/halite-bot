#![allow(non_snake_case)]
#![allow(warnings)]
#[macro_use] extern crate log;
extern crate fern;

#[macro_use] extern crate text_io;

//Notice: due to Rust's extreme dislike of (even private!) global mutables, we do not reset the production values of each tile during get_frame.
//If you change them, you may not be able to recover the actual production values of the map, so we recommend not editing them.
//However, if your code calls for it, you're welcome to edit the production values of the sites of the map - just do so at your own risk.

mod hlt;
use hlt::{ networking, types };
use std::collections::{HashMap, VecDeque, HashSet};

fn find_nearest_border(loc: types::Location, map: &types::GameMap, my_id: u8) -> u8 {
    // Breadth-first search to try and locate the nearest border
    let mut queue = VecDeque::new();
    for d in &types::CARDINALS {
        queue.push_back((loc, *d));
    }
    while let Some((l, d)) = queue.pop_front() {
        if map.get_site_ref(l, types::STILL).owner != my_id {
            return d
        } else {
            let next = map.get_location(l, d);
            if next != loc { // Don't loop around
                queue.push_back((next, d));
            }
        }
    }
    // Default direction
    types::NORTH
}

fn angle_to_direction(angle: f64) -> u8 {
    let pi: f64 = std::f64::consts::PI;
    let a = angle * 4f64/pi;
    if angle <= 1f64 || a > 7f64 {
        types::EAST
    } else if angle <= 3f64 {
        types::NORTH
    } else if angle <= 5f64 {
        types::WEST
    } else {
        types::SOUTH
    }
}

fn main() {
    let logger_config = fern::DispatchConfig {
        format: Box::new(|msg: &str, level: &log::LogLevel, _location: &log::LogLocation| {
            // This is a fairly simple format, though it's possible to do more complicated ones.
            // This closure can contain any code, as long as it produces a String message.
            format!("[{}] {}", level, msg)
        }),
        output: vec![fern::OutputConfig::file("output.log")],
        level: log::LogLevelFilter::Debug,
    };

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
                    (my_id, strength) if strength < threshold => {
                        moves.insert(l, types::STILL);
                    },
                    (my_id, _) => {
                        let direction = find_nearest_border(l, &game_map, my_id);
                        let prop = game_map.get_site_ref(l, direction);
                        let safe_own = prop.owner == my_id && prop.strength < 255 - site.strength;
                        let safe_other = prop.owner != my_id && prop.strength < 2 * site.strength;
                        let must_move = site.strength >= 230;
                        if safe_own || safe_other || must_move {
                            moves.insert(l, direction);
                        } else {
                            moves.insert(l, types::STILL);
                        }
                    },
                    //(_, _) => {},
                };
            }
        }
        networking::send_frame(moves);
    }
}
