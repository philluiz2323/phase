//! Regression: Omnath, Locus of Mana (#2887) — "gets +1/+1 for each unspent
//! green mana you have" was parsed as a flat +1/+1 (the per-mana multiplier
//! dropped), so Omnath never grew with floating green mana.
//!
//! With the fix the static parses to `AddDynamicPower`/`AddDynamicToughness`
//! over `QuantityRef::UnspentMana { Green }`, which the layer system resolves
//! against the controller's mana pool. This drives the REAL layer-derivation
//! path (`evaluate_layers`): the P/T read after derivation is computed by the
//! engine from the floating green mana, not asserted into existence.
//!
//! CR references (verified against docs/MagicCompRules.txt):
//!   - CR 106.4: unspent mana stays in a player's mana pool.
//!   - CR 613.4c: dynamic power/toughness-modifying continuous effects.

use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;

const OMNATH: &str = "Omnath, Locus of Mana gets +1/+1 for each unspent green mana you have.";

fn add_green(runner: &mut GameRunner, n: usize) {
    for _ in 0..n {
        runner.state_mut().players[0].mana_pool.add(ManaUnit::new(
            ManaType::Green,
            ObjectId(0),
            false,
            vec![],
        ));
    }
}

fn derived_power(runner: &mut GameRunner, id: ObjectId) -> i32 {
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());
    runner
        .state()
        .objects
        .get(&id)
        .expect("Omnath exists")
        .power
        .expect("creature has power")
}

fn derived_toughness(runner: &mut GameRunner, id: ObjectId) -> i32 {
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());
    runner
        .state()
        .objects
        .get(&id)
        .unwrap()
        .toughness
        .expect("creature has toughness")
}

#[test]
fn omnath_scales_with_unspent_green_mana() {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let omnath = scenario
        .add_creature_from_oracle(P0, "Omnath, Locus of Mana", 1, 1, OMNATH)
        .id();
    let mut runner = scenario.build();

    // No floating green mana → base 1/1 (the bug made this scale-less but the
    // flat +1/+1 would have read 2/2 here).
    assert_eq!(derived_power(&mut runner, omnath), 1, "0 green → 1/1 power");
    assert_eq!(
        derived_toughness(&mut runner, omnath),
        1,
        "0 green → 1/1 toughness"
    );

    // Three unspent green mana → +3/+3 = 4/4.
    add_green(&mut runner, 3);
    assert_eq!(derived_power(&mut runner, omnath), 4, "3 green → 4 power");
    assert_eq!(
        derived_toughness(&mut runner, omnath),
        4,
        "3 green → 4 toughness"
    );

    // Two more (five total) → +5/+5 = 6/6, proving it re-derives live.
    add_green(&mut runner, 2);
    assert_eq!(derived_power(&mut runner, omnath), 6, "5 green → 6 power");
    assert_eq!(
        derived_toughness(&mut runner, omnath),
        6,
        "5 green → 6 toughness"
    );
}
