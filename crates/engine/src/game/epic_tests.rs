use crate::game::game_object::GameObject;
use crate::game::{casting, epic, stack, triggers};
use crate::types::ability::{Effect, QuantityExpr, ResolvedAbility, TargetFilter};
use crate::types::card_type::CoreType;
use crate::types::events::GameEvent;
use crate::types::game_state::{CastingVariant, GameState, StackEntry, StackEntryKind};
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::keywords::Keyword;
use crate::types::mana::ManaCost;
use crate::types::phase::Phase;
use crate::types::player::PlayerId;
use crate::types::zones::Zone;

fn blank_ability(source_id: ObjectId) -> ResolvedAbility {
    ResolvedAbility::new(
        Effect::Draw {
            count: QuantityExpr::Fixed { value: 0 },
            target: TargetFilter::Controller,
        },
        Vec::new(),
        source_id,
        PlayerId(0),
    )
}

fn make_spell_object(
    state: &mut GameState,
    id: ObjectId,
    card_id: CardId,
    zone: Zone,
    keywords: Vec<Keyword>,
) {
    let mut object = GameObject::new(id, card_id, PlayerId(0), "Endless Swarm".to_string(), zone);
    object.card_types.core_types.push(CoreType::Instant);
    object.mana_cost = ManaCost::zero();
    object.keywords = keywords.clone();
    object.base_keywords = keywords;
    object.sync_missing_base_characteristics();
    state.objects.insert(id, object);
    if state.next_object_id <= id.0 {
        state.next_object_id = id.0 + 1;
    }
}

fn push_epic_spell(state: &mut GameState, spell_id: ObjectId) {
    let card_id = CardId(100);
    make_spell_object(state, spell_id, card_id, Zone::Stack, vec![Keyword::Epic]);
    state.stack.push_back(StackEntry {
        id: spell_id,
        source_id: spell_id,
        controller: PlayerId(0),
        kind: StackEntryKind::Spell {
            card_id,
            ability: Some(blank_ability(spell_id)),
            casting_variant: CastingVariant::Normal,
            actual_mana_spent: 0,
        },
    });
}

#[test]
fn resolving_epic_spell_registers_rest_of_game_effect_and_locks_casting() {
    let mut state = GameState::new_two_player(1);
    push_epic_spell(&mut state, ObjectId(10));

    let mut events = Vec::new();
    stack::resolve_top(&mut state, &mut events);

    assert_eq!(state.epic_effects.len(), 1);
    assert!(epic::player_is_epic_locked(&state, PlayerId(0)));
    assert!(matches!(
        state.objects.get(&ObjectId(10)).map(|object| object.zone),
        Some(Zone::Graveyard)
    ));

    make_spell_object(
        &mut state,
        ObjectId(20),
        CardId(200),
        Zone::Hand,
        Vec::new(),
    );

    assert!(!casting::can_cast_object_now(
        &state,
        PlayerId(0),
        ObjectId(20)
    ));
    assert!(casting::handle_cast_spell(
        &mut state,
        PlayerId(0),
        ObjectId(20),
        CardId(200),
        &mut Vec::new(),
    )
    .is_err());
}

#[test]
fn epic_upkeep_trigger_copies_spell_without_casting_or_rearming_epic() {
    let mut state = GameState::new_two_player(2);
    push_epic_spell(&mut state, ObjectId(10));

    let mut events = Vec::new();
    stack::resolve_top(&mut state, &mut events);
    assert_eq!(state.epic_effects.len(), 1);

    state.active_player = PlayerId(0);
    triggers::process_triggers(
        &mut state,
        &[GameEvent::PhaseChanged {
            phase: Phase::Upkeep,
        }],
    );
    assert_eq!(state.stack.len(), 1);

    events.clear();
    stack::resolve_top(&mut state, &mut events);
    assert_eq!(state.stack.len(), 1);
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::SpellCopied {
            original_id: ObjectId(10),
            ..
        }
    )));

    let copy_id = state.stack.back().expect("Epic copy on stack").id;
    let copy = state.objects.get(&copy_id).expect("Epic copy object");
    assert!(copy.is_token);
    assert!(!copy
        .keywords
        .iter()
        .any(|keyword| matches!(keyword, Keyword::Epic)));

    events.clear();
    stack::resolve_top(&mut state, &mut events);
    assert_eq!(state.epic_effects.len(), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GameEvent::SpellCast { .. })));
}
