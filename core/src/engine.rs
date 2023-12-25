use crate::{context::GameContext, encryptor::EncryptorT, types::GameStatus};
use race_api::engine::InitAccount;
use race_api::error::{Error, HandleError};
use race_api::event::Event;
use race_api::random::RandomStatus;

pub fn general_init_state(
    _context: &mut GameContext,
    _init_account: &InitAccount,
) -> Result<(), HandleError> {
    Ok(())
}

/// A general function for system events handling.
pub fn general_handle_event(
    context: &mut GameContext,
    event: &Event,
    encryptor: &dyn EncryptorT,
) -> Result<(), Error> {
    // General event handling
    match event {
        Event::Ready => {
            // This is the first event, we make it a checkpoint
            // context.checkpoint = true;
            Ok(())
        }

        Event::ShareSecrets { sender, shares } => {
            let addr = context.id_to_addr(*sender)?;
            context.add_shared_secrets(addr, shares.clone())?;
            let mut random_ids = Vec::<usize>::default();
            for random_state in context.list_random_states_mut() {
                if random_state.status == RandomStatus::Shared {
                    random_ids.push(random_state.id);
                    random_state.status = RandomStatus::Ready;
                }
            }
            if !random_ids.is_empty() {
                context.dispatch_event_instantly(Event::SecretsReady { random_ids });
            }
            Ok(())
        }

        Event::AnswerDecision {
            sender,
            decision_id,
            ciphertext,
            digest,
        } => {
            let addr = context.id_to_addr(*sender)?;
            context.answer_decision(*decision_id, &addr, ciphertext.clone(), digest.clone())?;
            Ok(())
        }

        Event::Mask {
            sender,
            random_id,
            ciphertexts,
        } => {
            let addr = context.id_to_addr(*sender)?;
            context.randomize_and_mask(&addr, *random_id, ciphertexts.clone())?;
            Ok(())
        }

        Event::Lock {
            sender,
            random_id,
            ciphertexts_and_digests: ciphertexts_and_tests,
        } => {
            let addr = context.id_to_addr(*sender)?;
            context.lock(&addr, *random_id, ciphertexts_and_tests.clone())?;
            Ok(())
        }

        Event::RandomnessReady { .. } => Ok(()),

        Event::Sync { .. } => Ok(()),

        Event::Leave { .. } => {
            if !context.allow_exit {
                Err(Error::CantLeave)
            } else {
                Ok(())
            }
        }

        Event::GameStart { access_version } => {
            context.set_game_status(GameStatus::Running);
            context.set_node_ready(*access_version);
            Ok(())
        }

        Event::OperationTimeout { ids: _ } => {
            // This event is for game handler
            Ok(())
        }

        Event::WaitingTimeout => Ok(()),

        Event::ActionTimeout { player_id: _ } => {
            // This event is for game handler
            Ok(())
        }

        Event::SecretsReady { random_ids } => {
            let mut res = vec![];

            for rid in random_ids {
                if let Ok(random_state) = context.get_random_state_mut(*rid) {
                    let options = &random_state.options;
                    let revealed = encryptor
                        .decrypt_with_secrets(
                            random_state.list_revealed_ciphertexts(),
                            random_state.list_revealed_secrets()?,
                            options,
                        )
                        .or(Err(Error::DecryptionFailed))?;
                    res.push((random_state.id, revealed));
                }
            }
            for (random_id, revealed) in res.into_iter() {
                context.add_revealed_random(random_id, revealed)?;
            }

            let mut res = vec![];
            for decision_state in context.list_decision_states() {
                let secret = decision_state.get_secret()?;
                let mut buf = decision_state
                    .get_answer()
                    .ok_or(Error::InvalidDecisionAnswer)?
                    .ciphertext
                    .clone();
                encryptor.apply(secret, &mut buf);
                res.push((
                    decision_state.id,
                    String::from_utf8(buf).or(Err(Error::DecryptionFailed))?,
                ));
            }
            for (decision_id, revealed) in res.into_iter() {
                context.add_revealed_answer(decision_id, revealed)?;
            }
            Ok(())
        }

        _ => Ok(()),
    }
}

/// Context maintaining after event handling.
pub fn post_handle_event(
    _old_context: &GameContext,
    _new_context: &mut GameContext,
) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::encryptor::tests::DummyEncryptor;

    use super::*;

    #[test]
    fn test_handle_game_start() -> anyhow::Result<()> {
        let encryptor = DummyEncryptor::default();
        let mut context = GameContext::default();
        let event = Event::GameStart { access_version: 1 };
        general_handle_event(&mut context, &event, &encryptor)?;
        assert_eq!(context.status, GameStatus::Running);
        Ok(())
    }
}
