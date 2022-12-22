// use race_core::error::{Error, Result};
// use race_core::context::{GameContext, GameStatus};
//
// use crate::Holdem;
// use crate::GameEvent;

// pub fn player_fold(holdem: &mut Holdem, ctx: &mut GameContext) -> Result<()> // {
//
//     // 1. player_id(event owner) in context?
//     // 2. check game status == running/playing?
//     // 3. player_id(even owner) == player is in action
//     // 4. update current player status
//     // 5. go to next state
//     match ctx.players.iter().find(|&p| p.addr == holdem.player_id) {
//         Some(_) => {
//             println!("The player is valid and can fold!");
//         },
//         None => { return Err(Error::Custom(String::from("Player not found!"))); }
//     }
//
//     // 2. check if game is running
//     match ctx.status {
//         GameStatus::Running => { println!("Valid game status: {:?}", ctx.status); },
//         _ => {
//             println!("Invalid game status: {:?}", ctx.status);
//             return Err(Error::Custom(String::from("Invalid game status!")));
//         }
//     }
//
//     // 3. Check if player is at action pos
//     if holdem.table.btn == holdem.player_pos {
//         // let the player fold and exit the fn
//         holdem.player_status = PlayerStatus::Fold;
//         // Log this action (use the println! macro for now):
//         // fn log_action(player_id, action_type) -> Result<()>
//         println!("Player folded");
//
//         // add timestamp to this specific state
//
//         // 4. Update the game state
//         // TODO: add the fold logic here
//         todo!();
//     } else {
//         Err(Error::Custom(String::from("Player is not at action pos!")))
//     }
// }

// pub fn player_check(holdem: &mut Holdem, ctx: &mut GameContext) -> Result<()> {
    //  match ctx.players.iter().find(|&p| p.addr == holdem.player_id) {
    //     Some(_) => { println!("The player is valid and can fold!"); },
    //     None => { return Err(Error::Custom(String::from("Player not found!"))); }
    // }
    //
    // // 2. check if game is running
    // match ctx.status {
    //     GameStatus::Running => { println!("Valid game status: {:?}", ctx.status); },
    //     _ => {
    //         println!("Invalid game status: {:?}", ctx.status);
    //         return Err(Error::Custom(String::from("Invalid game status!")));
    //     }
    // }
    //
    // // 3. Check if player is at action pos
    // if holdem.table.btn == holdem.player_pos {
    //     if holdem.street_bet == 200 {
    //         todo!();
    //     } else {
    //         Err(Error::Custom(String::from("Player cannot check!")))
    //     }
    // } else {
    //     Err(Error::Custom(String::from("Player is not at action pos!")))
    // }
// }
