use std::collections::HashMap;

use bomber_lib::{
    self,
    world::{Direction, Enemy, Object, Tile, TileOffset},
    Action, Player,
};
use bomber_macro::wasm_export;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct TileInfo {
    pub tile: Tile,
    pub object: Option<Object>,
    pub enemy: Option<Enemy>,
    pub xy: TileOffset,
}

/// Player struct. Can contain any arbitrary data, which will carry over between turns.
struct MyPlayer {
    dir: Direction,
    on_hill: bool,
    initial: bool,
    world: HashMap<TileOffset, TileInfo>,
    pos: TileOffset,
    explored: Vec<TileOffset>,
}

impl Default for MyPlayer {
    fn default() -> Self {
        Self {
            dir: Direction::West,
            on_hill: false,
            initial: true,
            world: HashMap::new(),
            pos: TileOffset(0, 0),
            explored: Vec::new(),
        }
    }
}

fn can_go_to(maybe_object: Option<Object>) -> bool {
    if let Some(object) = maybe_object {
        !object.is_solid()
    } else {
        true
    }
}

fn get_left(dir: Direction) -> Direction {
    match dir {
        Direction::West => Direction::South,
        Direction::North => Direction::West,
        Direction::East => Direction::North,
        Direction::South => Direction::East,
    }
}

fn get_opposite(dir: Direction) -> Direction {
    match dir {
        Direction::West => Direction::East,
        Direction::North => Direction::South,
        Direction::East => Direction::West,
        Direction::South => Direction::North,
    }
}

use pathfinding::prelude::bfs;

#[wasm_export]
impl Player for MyPlayer {
    fn act(
        &mut self,
        nearby: Vec<(Tile, Option<Object>, Option<Enemy>, bomber_lib::world::TileOffset)>,
    ) -> Action {
        if self.on_hill {
            return Action::StayStill;
        }

        let world = &mut self.world;
        for s in &nearby {
            let pos = s.3 + self.pos;
            let state = TileInfo { tile: s.0, object: s.1, enemy: s.2.clone(), xy: pos };
            world.insert(pos, state);
        }

        let original_dir = self.dir;

        let finds = [self.search_goal(self.get_hills()), self.search_nongoals(&self.explored)];
        let mut found_path = false;
        for find in finds {
            if let Some(goals) = find {
                for goal in goals {
                    let delta = goal - self.pos;
                    let delta: Result<Direction, ()> = delta.try_into();
                    if let Ok(dir) = delta {
                        found_path = true;
                        self.dir = dir;
                        break;
                    }
                }
            }
            if found_path {
                break;
            }
        }

        if !found_path {
            let mut states = HashMap::new();
            let mut can_goes = HashMap::new();

            for dir in Direction::all() {
                let s = nearby.iter().filter(|s| s.3 == dir.extend(1)).next();
                states.insert(dir, s);
                if let Some(s) = s {
                    let mut can_go = can_go_to(s.1);
                    if let Tile::Wall = s.0 {
                        can_go = false;
                    }
                    if s.2.is_some() {
                        can_go = false;
                    }
                    can_goes.insert(dir, can_go);
                }
            }

            for i in 0..4 {
                match can_goes.get(&self.dir) {
                    Some(can_go) => {
                        if *can_go {
                            found_path = true;
                            break;
                        }
                    },
                    None => {},
                }
                self.dir = get_left(self.dir);

                let next_pos = self.pos + self.dir.extend(1);
                if !self.explored.contains(&next_pos) {
                    found_path = true;
                }
            }
        }

        let tile_info = self.world.get(&(self.pos + self.dir.extend(1)));
        if let Some(tile_info) = tile_info {
            if let Tile::Hill = tile_info.tile {
                self.on_hill = true;
            }
        }

        if !found_path {
            return Action::StayStill;
        }

        // if get_opposite(self.dir) == original_dir && !self.initial {
        if self.dir != original_dir && !self.initial {
            self.pos = self.pos + self.dir.extend(1);
            self.explored.push(self.pos);

            Action::DropBombAndMove(self.dir)
        } else {
            self.initial = false;
            self.pos = self.pos + self.dir.extend(1);
            self.explored.push(self.pos);
            Action::Move(self.dir)
        }
    }

    fn name(&self) -> String {
        "Hidari-Kun".to_owned()
    }

    fn team_name() -> String {
        "Heiwa".to_owned()
    }
}

impl MyPlayer {
    fn get_hills(&self) -> Vec<&TileInfo> {
        self.world
            .iter()
            .map(|s| s.1)
            .filter(|s| if let Tile::Hill = s.tile { true } else { false })
            .collect()
    }

    fn search_goal(&self, goals: Vec<&TileInfo>) -> Option<Vec<TileOffset>> {
        let goals: Vec<Pos> = goals.iter().map(|g| Pos(g.xy.0, g.xy.1)).collect();
        let current = Pos(self.pos.0, self.pos.1);
        let result = bfs(&current, |p| p.successors(&self.world), |p| goals.contains(p));
        if result.is_none() {
            None
        } else {
            let pos = result.unwrap().iter().map(|p| TileOffset(p.0, p.1)).collect();
            Some(pos)
        }
    }

    fn search_nongoals(&self, goals: &Vec<TileOffset>) -> Option<Vec<TileOffset>> {
        let goals: Vec<Pos> = goals.iter().map(|g| Pos(g.0, g.1)).collect();
        let current = Pos(self.pos.0, self.pos.1);
        let result = bfs(&current, |p| p.successors(&self.world), |p| !goals.contains(p));
        if result.is_none() {
            None
        } else {
            let pos = result.unwrap().iter().map(|p| TileOffset(p.0, p.1)).collect();
            Some(pos)
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Pos(i32, i32);

impl Pos {
    fn successors(&self, world: &HashMap<TileOffset, TileInfo>) -> Vec<Pos> {
        let &Pos(x, y) = self;
        let nearby = vec![Pos(x + 1, y), Pos(x - 1, y), Pos(x, y + 1), Pos(x, y - 1)];
        let tile_infos = nearby.iter().filter_map(|pos| world.get(&TileOffset(pos.0, pos.1)));
        tile_infos
            .filter(|tile_info| {
                let mut can_go = can_go_to(tile_info.object);
                if let Tile::Wall = tile_info.tile {
                    can_go = false;
                }
                if tile_info.enemy.is_some() {
                    can_go = false;
                }
                can_go
            })
            .map(|ti| Pos(ti.xy.0, ti.xy.1))
            .collect()
    }
}
