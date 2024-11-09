use std::{fs::File, io::Read};

use race_core::types::{GameAccount, GameBundle, RecipientAccount, RegistrationAccount, ServerAccount, TokenAccount};
use regex::Regex;
use rusqlite::Connection;
use crate::{db::{create_game_account, create_game_bundle, create_player_info, create_recipient_account, create_server_account, create_token_account, list_game_accounts, list_token_accounts, prepare_all_tables, read_game_account, read_game_bundle, read_player_info, read_recipient_account, read_registration_account, read_server_account, read_token_account, update_game_account, update_player_info, update_recipient_account, PlayerInfo}, GameSpec};


pub struct Context {
    conn: Connection,
}

impl Default for Context {
    fn default() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        prepare_all_tables(&conn).unwrap();
        Context {
            conn
        }
    }
}

impl Context {
    pub fn load_games(&self, spec_paths: &[&str]) -> anyhow::Result<()> {
        for spec_path in spec_paths.iter() {
            self.add_game(spec_path)?;
        }
        Ok(())
    }

    pub fn load_bundles(&self, bundle_paths: &[&str]) -> anyhow::Result<()> {
        for bundle_path in bundle_paths.iter() {
            self.add_bundle(bundle_path)?;
        }
        Ok(())
    }

    pub fn add_token(&self, token_account: TokenAccount) -> anyhow::Result<()> {
        create_token_account(&self.conn, &token_account)?;
        Ok(())
    }

    pub fn load_default_tokens(&self) -> anyhow::Result<()> {
        self.add_token(TokenAccount {
            name: "USD Coin".into(),
            symbol: "USDC".into(),
            decimals: 6,
            icon: "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png".into(),
            addr: "FACADE_USDC".into(),
        })?;
        self.add_token(TokenAccount {
            name: "Tether USD".into(),
            symbol: "USDT".into(),
            decimals: 6,
            icon: "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.svg".into(),
            addr: "FACADE_USDT".into(),
        })?;
        self.add_token(TokenAccount {
            name: "Native Token".into(),
            symbol: "NATIVE".into(),
            decimals: 9,
            icon: "https://arweave.net/SH106hrChudKjQ_c6e6yd0tsGUbFIScv2LL6Dp-LDiI".into(),
            addr: "FACADE_NATIVE".into(),
        })?;
        self.add_token(TokenAccount {
            name: "Race Protocol".into(),
            symbol: "RACE".into(),
            decimals: 9,
            icon: "https://raw.githubusercontent.com/NutsPokerTeam/token-list/main/assets/mainnet/RACE5fnTKB9obGtCusArTQ6hhdNXAtf3HarvJM17rxJ/logo.svg".into(),
            addr: "FACADE_RACE".into(),
        })?;
        Ok(())
    }

    pub fn add_server(&self, server_account: &ServerAccount) -> anyhow::Result<()> {
        create_server_account(&self.conn, &server_account)?;
        println!("+ Server: {}", server_account.addr);
        Ok(())
    }

    pub fn add_bundle(&self, bundle_path: &str) -> anyhow::Result<()> {
        let re = Regex::new(r"[^a-zA-Z0-9]").unwrap();
        let bundle_addr = re.replace_all(&bundle_path, "").into_owned();
        let mut f = File::open(bundle_path).expect(&format!("Bundle {} not found", &bundle_path));
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        let bundle = GameBundle {
            addr: bundle_addr.clone(),
            name: bundle_addr.clone(),
            uri: "".into(),
            data,
        };
        create_game_bundle(&self.conn, &bundle)?;
        println!("+ Bundle: {}", bundle_addr);
        Ok(())
    }

    pub fn add_game(&self, spec_path: &str) -> anyhow::Result<()> {
        let f = File::open(spec_path).expect("Spec file not found");
        let GameSpec {
            title,
            bundle,
            token,
            max_players,
            entry_type,
            data: spec_data,
        } = serde_json::from_reader(f).expect(&format!("Invalid spec file: {}", spec_path));

        let re = Regex::new(r"[^a-zA-Z0-9]").unwrap();
        let bundle_addr = re.replace_all(&bundle, "").into_owned();
        let game_addr = re.replace_all(&spec_path, "").into_owned();
        let recipient_addr = format!("{}_recipient", game_addr);
        let mut f = File::open(&bundle).expect(&format!("Bundle {} not found", &bundle));
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        println!("Game bundle data length: {}", data.len());
        let bundle = GameBundle {
            addr: bundle_addr.clone(),
            name: bundle_addr.clone(),
            uri: "".into(),
            data,
        };
        let game = GameAccount {
            addr: game_addr.clone(),
            title,
            token_addr: token.to_owned(),
            bundle_addr: bundle_addr.clone(),
            data_len: spec_data.len() as u32,
            data: spec_data,
            max_players,
            entry_type,
            ..Default::default()
        };
        let recipient = RecipientAccount {
            addr: recipient_addr.clone(),
            ..Default::default()
        };
        create_game_bundle(&self.conn, &bundle)?;
        create_game_account(&self.conn, &game)?;
        create_recipient_account(&self.conn, &recipient)?;
        println!("! Load game from `{}`", spec_path);
        println!("+ Game: {}", game_addr);
        println!("+ Bundle: {}", bundle_addr);

        Ok(())
    }


    pub fn create_game_account(&self, game_account: &GameAccount) -> anyhow::Result<()> {
        create_game_account(&self.conn, &game_account)?;
        println!("+ Game: {}", game_account.addr);
        Ok(())
    }

    pub fn create_recipient_account(&self, recipient_account: &RecipientAccount) -> anyhow::Result<()> {
        create_recipient_account(&self.conn, &recipient_account)?;
        println!("+ Recipient: {}", recipient_account.addr);
        Ok(())
    }

    pub fn create_player_info(&self, player_info: &PlayerInfo) -> anyhow::Result<()> {
        create_player_info(&self.conn, &player_info)?;
        println!("+ Player profile: {}", player_info.profile.addr);
        Ok(())
    }

    pub fn get_game_bundle(&self, addr: &str) -> anyhow::Result<Option<GameBundle>> {
        Ok(read_game_bundle(&self.conn, addr)?)
    }

    pub fn get_game_account(&self, addr: &str) -> anyhow::Result<Option<GameAccount>> {
        Ok(read_game_account(&self.conn, addr)?)
    }

    pub fn list_game_accounts(&self) -> anyhow::Result<Vec<GameAccount>> {
        Ok(list_game_accounts(&self.conn)?)
    }

    pub fn list_token_accounts(&self) -> anyhow::Result<Vec<TokenAccount>> {
        Ok(list_token_accounts(&self.conn)?)
    }

    pub fn get_player_info(&self, player_addr: &str) -> anyhow::Result<Option<PlayerInfo>> {
        Ok(read_player_info(&self.conn, player_addr)?)
    }

    #[allow(unused)]
    pub fn get_registration_account(&self, addr: &str) -> anyhow::Result<Option<RegistrationAccount>> {
        Ok(read_registration_account(&self.conn, addr)?)
    }

    #[allow(unused)]
    pub fn get_token_account(&self, addr: &str) -> anyhow::Result<Option<TokenAccount>> {
        Ok(read_token_account(&self.conn, addr)?)
    }

    pub fn get_recipient_account(&self, addr: &str) -> anyhow::Result<Option<RecipientAccount>> {
        Ok(read_recipient_account(&self.conn, addr)?)
    }

    pub fn get_server_account(&self, addr: &str) -> anyhow::Result<Option<ServerAccount>> {
        Ok(read_server_account(&self.conn, addr)?)
    }

    pub fn update_game_account(&self, game_account: &GameAccount) -> anyhow::Result<()> {
        update_game_account(&self.conn, &game_account)?;
        Ok(())
    }

    pub fn update_player_info(&self, player_info: &PlayerInfo) -> anyhow::Result<()> {
        update_player_info(&self.conn, &player_info)?;
        Ok(())
    }

    #[allow(unused)]
    pub fn update_recipient_account(&self, recipient_account: &RecipientAccount) -> anyhow::Result<()> {
        update_recipient_account(&self.conn, &recipient_account)?;
        Ok(())
    }
}
