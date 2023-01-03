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
