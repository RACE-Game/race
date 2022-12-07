use crate::{context::GameContext, error::Result, event::Event, types::GameAccount};

pub trait GameHandler: Sized {
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self>;

    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()>;
}

// pub struct GeneralGameHandler {
//     pub handler: Box<dyn GameHandler>,
// }

// impl GeneralGameHandler {
//     pub fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
//         match event {
//             Event::Custom(_) => todo!(),
//             Event::Join { ref player_addr, timestamp: _ } => {
//                 if context.players.iter().find(|p| p.addr.eq(player_addr)).is_some() {
//                     return Err(Error::PlayerAlreadyInGame);
//                 }
//                 context.players.push(Player::new(player_addr.to_owned()));
//                 self.handler.handle_event(context, event)
//             }
//             Event::Leave { ref player_addr, timestamp: _ } => {
//                 if context.players.iter().find(|p| p.addr.eq(player_addr)).is_none() {
//                     return Err(Error::NoSuchPlayer);
//                 }
//                 context.players.retain(|p|p.addr.ne(player_addr));
//                 self.handler.handle_event(context, event)
//             }
//             Event::Ready { player_addr, timestamp } => todo!(),
//             Event::GameStart { timestamp } => todo!(),
//             Event::WaitTimeout { timestamp } => todo!(),
//             Event::ActionTimeout { player_addr, timestamp } => todo!(),
//             Event::SecretsReady { timestamp } => todo!(),
//             Event::RandomnessReady { timestamp } => todo!(),
//             _ => todo!()
//         }
//     }
// }
