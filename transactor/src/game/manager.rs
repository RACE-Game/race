/// Manage game instances.
pub struct Manager {
    pub handle_map: HashMap<Address, GameHandle>,
}

impl Default for Manager {
    fn default() -> Self {
        Manager {
            handle_map: HashMap::new(),
        }
    }
}

impl Manager {
    pub async fn start_game(
        &mut self,
        addr: Address,
        game_type: GameType,
    ) -> Result<(), TransactorError> {
        if self.handle_map.contains_key(&addr) {
            return Err(TransactorError::GameAlreadyStarted)
        }
        println!("Start game {:?}", game_type);
        let handle = GameHandle::start(&addr, game_type).await?;
        println!("Created game handle {:?}", handle);
        self.handle_map.insert(addr, handle);
        Ok(())
    }

    pub fn get_game_handle(&self, addr: &Address) -> Result<&GameHandle, TransactorError> {
        if let Some(handle) = self.handle_map.get(addr) {
            Ok(handle)
        } else {
            Err(TransactorError::GameNotFound)
        }
    }
}
