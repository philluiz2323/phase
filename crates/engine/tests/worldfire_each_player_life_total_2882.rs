//! Worldfire — "Each player's life total becomes 1" applies to EVERY player.
//!
//! Oracle (third clause): "Each player's life total becomes 1."
//!
//! Regression for issue #2882: the clause used to parse to a SetLifeTotal with
//! an unscoped target, so the engine prompted the caster to choose a single
//! player (and, absent a chosen target, only the controller's life changed).
//! CR 119.3 — a non-targeted set-life effect applies to all players with no
//! targeting.
//!
//! The fix parses the "each player's life total" possessive into a
//! `player_scope: All` fan-out over a `TargetFilter::Controller`-bound
//! SetLifeTotal, so each per-player iteration rebinds the controller and sets
//! that player's life. This test drives the parsed ability through
//! `resolve_ability_chain` against an asymmetric two-player board and asserts
//! BOTH players — controller and non-controller — are set to 1. The
//! non-controller assertion is the discriminator: under the old single-target
//! behavior P1 would keep its life total untouched.

use engine::game::ability_utils::build_resolved_from_def;
use engine::game::effects::resolve_ability_chain;
use engine::game::zones::create_object;
use engine::parser::oracle_effect::parse_effect_chain;
use engine::types::ability::AbilityKind;
use engine::types::game_state::GameState;
use engine::types::identifiers::CardId;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const ORACLE: &str = "Each player's life total becomes 1.";

fn set_life(state: &mut GameState, player: PlayerId, life: i32) {
    state
        .players
        .iter_mut()
        .find(|p| p.id == player)
        .expect("player exists")
        .life = life;
}

fn life(state: &GameState, player: PlayerId) -> i32 {
    state
        .players
        .iter()
        .find(|p| p.id == player)
        .expect("player exists")
        .life
}

#[test]
fn worldfire_set_life_applies_to_all_players() {
    let mut state = GameState::new_two_player(42);
    // Asymmetric, both above the target: the controller and a non-controller
    // start at different life totals so "set everyone to 1" is observable.
    set_life(&mut state, PlayerId(0), 20);
    set_life(&mut state, PlayerId(1), 7);

    let source = create_object(
        &mut state,
        CardId(1),
        PlayerId(0),
        "Worldfire".to_string(),
        Zone::Stack,
    );

    let def = parse_effect_chain(ORACLE, AbilityKind::Spell);
    let ability = build_resolved_from_def(&def, source, PlayerId(0));

    let mut events = Vec::new();
    resolve_ability_chain(&mut state, &ability, &mut events, 0)
        .expect("a non-targeted set-life must resolve without a pending choice");

    assert_eq!(
        life(&state, PlayerId(0)),
        1,
        "the controller's life total must become 1"
    );
    assert_eq!(
        life(&state, PlayerId(1)),
        1,
        "the NON-controller's life total must become 1 too (CR 119.3 — all \
         players, not a single chosen target)"
    );
}
