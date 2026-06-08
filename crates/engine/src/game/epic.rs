use crate::game::triggers::{PendingTrigger, PendingTriggerContext};
use crate::types::ability::{CopyRetargetPermission, Effect, ResolvedAbility, TargetFilter};
use crate::types::events::GameEvent;
use crate::types::game_state::{EpicEffect, GameState, StackEntry, StackEntryKind};
use crate::types::identifiers::ObjectId;
use crate::types::keywords::Keyword;
use crate::types::phase::Phase;
use crate::types::player::PlayerId;

pub fn player_is_epic_locked(state: &GameState, player: PlayerId) -> bool {
    state
        .epic_effects
        .iter()
        .any(|effect| effect.controller == player)
}

pub fn epic_source_entry(state: &GameState, source_id: ObjectId) -> Option<StackEntry> {
    state
        .epic_effects
        .iter()
        .find(|effect| effect.source_entry.id == source_id)
        .map(|effect| effect.source_entry.clone())
}

pub fn register_epic_effect(state: &mut GameState, entry: &StackEntry) {
    if !entry_has_epic_spell_keyword(state, entry) {
        return;
    }

    let source_name = state
        .objects
        .get(&entry.id)
        .map(|object| object.name.clone())
        .unwrap_or_default();
    let timestamp = state.next_timestamp() as u32;

    state.epic_effects.push(EpicEffect {
        controller: entry.controller,
        source_entry: entry.clone(),
        source_name,
        timestamp,
    });
}

pub fn collect_upkeep_triggers(
    state: &GameState,
    events: &[GameEvent],
    pending: &mut Vec<PendingTriggerContext>,
) {
    for event in events {
        if !matches!(
            event,
            GameEvent::PhaseChanged {
                phase: Phase::Upkeep
            }
        ) {
            continue;
        }

        let active = state.active_player;
        for effect in state
            .epic_effects
            .iter()
            .filter(|effect| effect.controller == active)
        {
            let ability = ResolvedAbility::new(
                Effect::CopySpell {
                    target: TargetFilter::SelfRef,
                    retarget: CopyRetargetPermission::MayChooseNewTargets,
                    copier: None,
                },
                Vec::new(),
                effect.source_entry.id,
                effect.controller,
            );

            pending.push(PendingTriggerContext {
                trigger_events: vec![event.clone()],
                pending: PendingTrigger {
                    source_id: effect.source_entry.id,
                    controller: effect.controller,
                    condition: None,
                    ability,
                    timestamp: effect.timestamp,
                    target_constraints: Vec::new(),
                    distribute: None,
                    trigger_event: Some(event.clone()),
                    modal: None,
                    mode_abilities: Vec::new(),
                    description: Some(format!("Epic - {}", effect.source_name)),
                    may_trigger_origin: None,
                    subject_match_count: None,
                    die_result: None,
                },
            });
        }
    }
}

pub fn remove_epic_keyword_from_copy(state: &mut GameState, object_id: ObjectId) {
    if let Some(object) = state.objects.get_mut(&object_id) {
        object
            .keywords
            .retain(|keyword| !matches!(keyword, Keyword::Epic));
        object
            .base_keywords
            .retain(|keyword| !matches!(keyword, Keyword::Epic));
    }
}

fn entry_has_epic_spell_keyword(state: &GameState, entry: &StackEntry) -> bool {
    matches!(&entry.kind, StackEntryKind::Spell { .. })
        && state.objects.get(&entry.id).is_some_and(|object| {
            !object.is_token
                && object
                    .keywords
                    .iter()
                    .any(|keyword| matches!(keyword, Keyword::Epic))
        })
}
