use anyhow::{anyhow, Result};

use crate::{
    context::Context,
    db::{ProductEventLogEntry, UserProgress},
    product_rules::{
        level_for_xp, rank_tier_for_level, HAND_PARTICIPATION_XP, HAND_WIN_BONUS,
        JOIN_XP_BONUS, MIN_HANDS_FOR_SESSION_COMPLETION, MIN_SESSION_DURATION_SECONDS,
        SESSION_COMPLETION_XP,
    },
};

#[derive(Clone, Debug)]
pub enum ProductEvent {
    GuestTableJoined {
        event_id: String,
        guest_id: String,
        player_addr: String,
        game_id: String,
        timestamp: u64,
    },
    GuestHandFinished {
        event_id: String,
        guest_id: String,
        player_addr: String,
        hand_id: String,
        did_participate: bool,
        did_win_hand: bool,
        timestamp: u64,
    },
    GuestSessionFinished {
        event_id: String,
        guest_id: String,
        player_addr: String,
        session_id: String,
        hands_played_in_session: u64,
        session_duration_seconds: u64,
        timestamp: u64,
    },
}

#[allow(dead_code)]
pub struct ProductEventApplyResult {
    pub applied: bool,
    pub xp_delta: u64,
}

pub struct ProductEventService;

impl ProductEventService {
    pub fn apply(context: &mut Context, event: ProductEvent) -> Result<ProductEventApplyResult> {
        match event {
            ProductEvent::GuestTableJoined {
                event_id,
                guest_id,
                player_addr,
                game_id,
                timestamp,
            } => {
                validate_guest_identity(context, &guest_id, &player_addr)?;
                let was_inserted = context.record_product_event_once(ProductEventLogEntry {
                    event_id,
                    event_type: format!("guest_table_joined:{game_id}"),
                    guest_id: guest_id.clone(),
                    created_at: timestamp,
                })?;
                if !was_inserted {
                    return Ok(ProductEventApplyResult { applied: false, xp_delta: 0 });
                }

                context.record_user_joined_game(&guest_id, timestamp)?;
                apply_xp_delta(context, &guest_id, JOIN_XP_BONUS, timestamp)?;

                Ok(ProductEventApplyResult {
                    applied: true,
                    xp_delta: JOIN_XP_BONUS,
                })
            }
            ProductEvent::GuestHandFinished {
                event_id,
                guest_id,
                player_addr,
                hand_id,
                did_participate,
                did_win_hand,
                timestamp,
            } => {
                validate_guest_identity(context, &guest_id, &player_addr)?;
                let was_inserted = context.record_product_event_once(ProductEventLogEntry {
                    event_id,
                    event_type: format!("guest_hand_finished:{hand_id}"),
                    guest_id: guest_id.clone(),
                    created_at: timestamp,
                })?;
                if !was_inserted {
                    return Ok(ProductEventApplyResult { applied: false, xp_delta: 0 });
                }
                if !did_participate {
                    return Ok(ProductEventApplyResult { applied: true, xp_delta: 0 });
                }

                context.increment_user_hands_played(&guest_id)?;
                let xp_delta = HAND_PARTICIPATION_XP
                    + if did_win_hand { HAND_WIN_BONUS } else { 0 };
                apply_xp_delta(context, &guest_id, xp_delta, timestamp)?;

                Ok(ProductEventApplyResult {
                    applied: true,
                    xp_delta,
                })
            }
            ProductEvent::GuestSessionFinished {
                event_id,
                guest_id,
                player_addr,
                session_id,
                hands_played_in_session,
                session_duration_seconds,
                timestamp,
            } => {
                validate_guest_identity(context, &guest_id, &player_addr)?;
                let was_inserted = context.record_product_event_once(ProductEventLogEntry {
                    event_id,
                    event_type: format!("guest_session_finished:{session_id}"),
                    guest_id: guest_id.clone(),
                    created_at: timestamp,
                })?;
                if !was_inserted {
                    return Ok(ProductEventApplyResult { applied: false, xp_delta: 0 });
                }

                if hands_played_in_session < MIN_HANDS_FOR_SESSION_COMPLETION
                    || session_duration_seconds < MIN_SESSION_DURATION_SECONDS
                {
                    return Ok(ProductEventApplyResult { applied: true, xp_delta: 0 });
                }

                apply_xp_delta(context, &guest_id, SESSION_COMPLETION_XP, timestamp)?;
                Ok(ProductEventApplyResult {
                    applied: true,
                    xp_delta: SESSION_COMPLETION_XP,
                })
            }
        }
    }
}

fn validate_guest_identity(context: &mut Context, guest_id: &str, player_addr: &str) -> Result<()> {
    let guest = context
        .get_guest_account_by_guest_id(guest_id)?
        .ok_or_else(|| anyhow!("guest-account-not-found"))?;

    if guest.player_addr != player_addr {
        return Err(anyhow!("guest-player-mismatch"));
    }

    Ok(())
}

fn apply_xp_delta(context: &mut Context, guest_id: &str, xp_delta: u64, timestamp: u64) -> Result<()> {
    let current = context
        .get_user_progress(guest_id)?
        .ok_or_else(|| anyhow!("user-progress-not-found"))?;
    let xp = current.xp.saturating_add(xp_delta);
    let level = level_for_xp(xp);
    let rank_tier = rank_tier_for_level(level).to_string();

    context.update_user_progress(&UserProgress {
        guest_id: guest_id.to_string(),
        rank_tier,
        xp,
        level,
        updated_at: timestamp,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ProductEvent, ProductEventService};
    use crate::{
        context::Context,
        db::{GuestAccount, PlayerInfo},
        product_rules::{HAND_PARTICIPATION_XP, HAND_WIN_BONUS, JOIN_XP_BONUS, SESSION_COMPLETION_XP},
    };
    use race_core::types::PlayerProfile;
    use std::collections::HashMap;

    const GUEST_TOKEN_ADDR: &str = "FACADE_GUEST_CHIPS";

    fn seed_guest_context() -> Context {
        let mut context = Context::in_memory();
        context.load_default_tokens().unwrap();

        let guest_account = GuestAccount {
            guest_id: "guest-evt-1".into(),
            player_addr: "guest_player_evt_1".into(),
            nick: "EventGuest".into(),
            status: "active".into(),
            created_at: 10,
            updated_at: 10,
        };
        let player_info = PlayerInfo {
            balances: HashMap::from([(GUEST_TOKEN_ADDR.to_string(), 1_000_000)]),
            nfts: HashMap::new(),
            profile: PlayerProfile {
                addr: guest_account.player_addr.clone(),
                nick: guest_account.nick.clone(),
                pfp: None,
                credentials: vec![1],
            },
        };

        context.create_guest_account(&guest_account).unwrap();
        context.create_player_info(&player_info).unwrap();
        context
    }

    #[test]
    fn applies_table_join_event_once() {
        let mut context = seed_guest_context();

        let first = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestTableJoined {
                event_id: "join:1".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                game_id: "table-1".into(),
                timestamp: 100,
            },
        )
        .unwrap();
        let second = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestTableJoined {
                event_id: "join:1".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                game_id: "table-1".into(),
                timestamp: 100,
            },
        )
        .unwrap();

        let progress = context.get_user_progress("guest-evt-1").unwrap().unwrap();
        let stats = context.get_user_stats("guest-evt-1").unwrap().unwrap();

        assert!(first.applied);
        assert!(!second.applied);
        assert_eq!(progress.xp, JOIN_XP_BONUS);
        assert_eq!(progress.level, 1);
        assert_eq!(progress.rank_tier, "Bronze I");
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.last_played_at, Some(100));
    }

    #[test]
    fn applies_hand_event_with_win_bonus_once() {
        let mut context = seed_guest_context();

        let first = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestHandFinished {
                event_id: "hand:1".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                hand_id: "h1".into(),
                did_participate: true,
                did_win_hand: true,
                timestamp: 200,
            },
        )
        .unwrap();
        let second = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestHandFinished {
                event_id: "hand:1".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                hand_id: "h1".into(),
                did_participate: true,
                did_win_hand: true,
                timestamp: 200,
            },
        )
        .unwrap();

        let progress = context.get_user_progress("guest-evt-1").unwrap().unwrap();
        let stats = context.get_user_stats("guest-evt-1").unwrap().unwrap();

        assert!(first.applied);
        assert!(!second.applied);
        assert_eq!(first.xp_delta, HAND_PARTICIPATION_XP + HAND_WIN_BONUS);
        assert_eq!(stats.hands_played, 1);
        assert_eq!(progress.xp, HAND_PARTICIPATION_XP + HAND_WIN_BONUS);
    }

    #[test]
    fn ignores_non_participating_hand_for_stats_and_xp() {
        let mut context = seed_guest_context();

        let result = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestHandFinished {
                event_id: "hand:np".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                hand_id: "hnp".into(),
                did_participate: false,
                did_win_hand: false,
                timestamp: 210,
            },
        )
        .unwrap();

        let progress = context.get_user_progress("guest-evt-1").unwrap().unwrap();
        let stats = context.get_user_stats("guest-evt-1").unwrap().unwrap();

        assert!(result.applied);
        assert_eq!(result.xp_delta, 0);
        assert_eq!(stats.hands_played, 0);
        assert_eq!(progress.xp, 0);
    }

    #[test]
    fn applies_session_completion_only_when_eligible() {
        let mut context = seed_guest_context();

        let too_short = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestSessionFinished {
                event_id: "session:short".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                session_id: "s-short".into(),
                hands_played_in_session: 1,
                session_duration_seconds: 60,
                timestamp: 300,
            },
        )
        .unwrap();
        let eligible = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestSessionFinished {
                event_id: "session:eligible".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                session_id: "s-ok".into(),
                hands_played_in_session: 3,
                session_duration_seconds: 300,
                timestamp: 301,
            },
        )
        .unwrap();
        let duplicate = ProductEventService::apply(
            &mut context,
            ProductEvent::GuestSessionFinished {
                event_id: "session:eligible".into(),
                guest_id: "guest-evt-1".into(),
                player_addr: "guest_player_evt_1".into(),
                session_id: "s-ok".into(),
                hands_played_in_session: 3,
                session_duration_seconds: 300,
                timestamp: 301,
            },
        )
        .unwrap();

        let progress = context.get_user_progress("guest-evt-1").unwrap().unwrap();

        assert!(too_short.applied);
        assert_eq!(too_short.xp_delta, 0);
        assert!(eligible.applied);
        assert_eq!(eligible.xp_delta, SESSION_COMPLETION_XP);
        assert!(!duplicate.applied);
        assert_eq!(progress.xp, SESSION_COMPLETION_XP);
    }
}
