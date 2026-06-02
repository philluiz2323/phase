use crate::types::ability::{
    ContinuousModification, Duration, EffectError, EffectKind, ResolvedAbility, TargetFilter,
    TargetRef,
};
use crate::types::events::GameEvent;
use crate::types::game_state::GameState;
use crate::types::statics::StaticMode;

/// CR 702.39a / CR 509.1c: Force block — the target creature must block the
/// *specific* attacker (this effect's source) this turn if able.
///
/// Grants the `MustBlockAttacker { attacker }` static mode via a transient
/// continuous effect that expires at end of turn, where `attacker` is
/// `ability.source_id` — the provoking creature ("…block **it** this turn if
/// able"). Combat validation in `validate_blockers()` then requires the target
/// to be declared as a blocker of *that* attacker when it can legally block it,
/// rather than the attacker-agnostic generic `MustBlock`.
///
/// Note: `MustBlock` (creature must block any attacker), `MustBlockAttacker`
/// (creature must block one specific attacker), and `MustBeBlocked` (creature
/// must be blocked by others) are three distinct requirements (CR 509.1c).
pub fn resolve(
    state: &mut GameState,
    ability: &ResolvedAbility,
    events: &mut Vec<GameEvent>,
) -> Result<(), EffectError> {
    for target in &ability.targets {
        if let TargetRef::Object(obj_id) = target {
            // CR 509.1c: Requirements that creatures must block are checked during
            // the declare blockers step.
            if !state.objects.contains_key(obj_id) {
                continue;
            }

            // CR 702.39a / CR 509.1c: Grant MustBlockAttacker bound to this
            // effect's source (the provoking attacker) until end of turn, so the
            // target must block that specific attacker — not any attacker.
            state.add_transient_continuous_effect(
                ability.source_id,
                ability.controller,
                Duration::UntilEndOfTurn,
                TargetFilter::SpecificObject { id: *obj_id },
                vec![ContinuousModification::AddStaticMode {
                    mode: StaticMode::MustBlockAttacker {
                        attacker: ability.source_id,
                    },
                }],
                None,
            );
        }
    }

    events.push(GameEvent::EffectResolved {
        kind: EffectKind::ForceBlock,
        source_id: ability.source_id,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::zones::create_object;
    use crate::types::ability::{Effect, TargetRef};
    use crate::types::identifiers::{CardId, ObjectId};
    use crate::types::player::PlayerId;
    use crate::types::zones::Zone;

    fn make_force_block_ability(source: ObjectId, target: ObjectId) -> ResolvedAbility {
        ResolvedAbility::new(
            Effect::ForceBlock {
                target: TargetFilter::Any,
            },
            vec![TargetRef::Object(target)],
            source,
            PlayerId(0),
        )
    }

    #[test]
    fn force_block_grants_must_block_static() {
        let mut state = GameState::new_two_player(42);
        let source = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Provocateur".to_string(),
            Zone::Battlefield,
        );
        let target = create_object(
            &mut state,
            CardId(2),
            PlayerId(1),
            "Bear".to_string(),
            Zone::Battlefield,
        );

        let ability = make_force_block_ability(source, target);
        let mut events = Vec::new();
        resolve(&mut state, &ability, &mut events).unwrap();

        // Verify transient continuous effect was created, bound to the source
        // (provoking attacker) rather than the attacker-agnostic MustBlock.
        assert!(
            state.transient_continuous_effects.iter().any(|ce| {
                ce.modifications.iter().any(|m| {
                    matches!(
                        m,
                        ContinuousModification::AddStaticMode {
                            mode: StaticMode::MustBlockAttacker { attacker },
                        } if *attacker == source
                    )
                })
            }),
            "Should grant MustBlockAttacker(source) static to target"
        );

        // Verify EffectResolved emitted
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::EffectResolved {
                kind: EffectKind::ForceBlock,
                ..
            }
        )));
    }

    #[test]
    fn force_block_multiple_targets() {
        let mut state = GameState::new_two_player(42);
        let source = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Source".to_string(),
            Zone::Battlefield,
        );
        let target1 = create_object(
            &mut state,
            CardId(2),
            PlayerId(1),
            "Bear1".to_string(),
            Zone::Battlefield,
        );
        let target2 = create_object(
            &mut state,
            CardId(3),
            PlayerId(1),
            "Bear2".to_string(),
            Zone::Battlefield,
        );

        let ability = ResolvedAbility::new(
            Effect::ForceBlock {
                target: TargetFilter::Any,
            },
            vec![TargetRef::Object(target1), TargetRef::Object(target2)],
            source,
            PlayerId(0),
        );
        let mut events = Vec::new();
        resolve(&mut state, &ability, &mut events).unwrap();

        let must_block_count = state
            .transient_continuous_effects
            .iter()
            .filter(|ce| {
                ce.modifications.iter().any(|m| {
                    matches!(
                        m,
                        ContinuousModification::AddStaticMode {
                            mode: StaticMode::MustBlockAttacker { attacker },
                        } if *attacker == source
                    )
                })
            })
            .count();
        assert_eq!(must_block_count, 2, "Should create one effect per target");
    }
}
