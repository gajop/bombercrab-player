use bomber_lib::{
    self,
    world::{Direction, Enemy, Object, Tile},
    Action, Player,
};
use bomber_macro::wasm_export;

/// Player struct. Can contain any arbitrary data, which will carry over between turns.
#[derive(Default)]
struct MyPlayer;

#[wasm_export]
impl Player for MyPlayer {
    fn act(
        &mut self,
        nearby: Vec<(Tile, Option<Object>, Option<Enemy>, bomber_lib::world::TileOffset)>,
    ) -> Action {
        for s in nearby {
            let t = s.0;
            let offset = s.3;
        }

        Action::Move(Direction::North)
    }

    fn name(&self) -> String {
        "<h1>{{username}}</h1>".to_owned()
    }

    fn team_name() -> String {
        "<div class\"winner\">My Team</div>".to_owned()
    }
}
