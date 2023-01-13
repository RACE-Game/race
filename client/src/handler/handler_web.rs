

pub struct Handler {}


impl Handler {
    pub fn new(game_bundle: GameBundle) -> Self {
        Self {}
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        Ok(())
    }
}
