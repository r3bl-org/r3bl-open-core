// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{collections::HashMap, fmt::Debug, marker::PhantomData};

use super::HasFocus;
use crate::{BoxedSafeComponent, CommonResult, ContainsResult, EventPropagation,
            FlexBoxId, GlobalData, InputEvent};

#[derive(Debug)]
pub struct ComponentRegistry<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    _phantom: PhantomData<(S, AS)>,
}

pub type ComponentRegistryMap<S, A> = HashMap<FlexBoxId, BoxedSafeComponent<S, A>>;

impl<S, AS> ComponentRegistry<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub fn put(
        map: &mut ComponentRegistryMap<S, AS>,
        id: FlexBoxId,
        component: BoxedSafeComponent<S, AS>,
    ) {
        map.insert(id, component);
    }

    pub fn contains(
        map: &mut ComponentRegistryMap<S, AS>,
        id: FlexBoxId,
    ) -> ContainsResult {
        if map.contains_key(&id) {
            ContainsResult::DoesContain
        } else {
            ContainsResult::DoesNotContain
        }
    }

    pub fn get(
        map: &mut ComponentRegistryMap<S, AS>,
        id: FlexBoxId,
    ) -> Option<&BoxedSafeComponent<S, AS>> {
        map.get(&id)
    }

    pub fn remove(
        map: &mut ComponentRegistryMap<S, AS>,
        id: FlexBoxId,
    ) -> Option<BoxedSafeComponent<S, AS>> {
        map.remove(&id)
    }

    pub fn try_to_get_focused_component<'a>(
        map: &'a mut ComponentRegistryMap<S, AS>,
        has_focus: &'_ HasFocus,
    ) -> Option<&'a mut BoxedSafeComponent<S, AS>> {
        if let Some(ref id) = has_focus.get_id() {
            ComponentRegistry::try_to_get_component_by_id(map, *id)
        } else {
            None
        }
    }

    pub fn try_to_get_component_by_id(
        map: &mut ComponentRegistryMap<S, AS>,
        id: FlexBoxId,
    ) -> Option<&mut BoxedSafeComponent<S, AS>> {
        if let Some(component) = map.get_mut(&id) {
            return Some(component);
        }
        None
    }

    pub fn reset_component(map: &mut ComponentRegistryMap<S, AS>, id: FlexBoxId) {
        if let Some(it) = ComponentRegistry::try_to_get_component_by_id(map, id) {
            it.reset();
        }
    }

    pub fn reset_focused_component(
        map: &mut ComponentRegistryMap<S, AS>,
        has_focus: &mut HasFocus,
    ) {
        if let Some(it) = ComponentRegistry::try_to_get_focused_component(map, has_focus)
        {
            it.reset();
        }
    }

    /// Routes an input event to the currently focused component.
    ///
    /// # Errors
    ///
    /// Returns an error if the component fails to handle the event.
    pub fn route_event_to_focused_component(
        global_data: &mut GlobalData<S, AS>,
        input_event: InputEvent,
        component_registry_map: &mut ComponentRegistryMap<S, AS>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        // If component has focus, then route input_event to it. Return its
        // propagation enum.
        if let Some(component) = ComponentRegistry::try_to_get_focused_component(
            component_registry_map,
            has_focus,
        ) {
            let result_event_propagation =
                component.handle_event(global_data, input_event, has_focus)?;
            Ok(result_event_propagation)
        } else {
            // input_event not handled, propagate it.
            Ok(EventPropagation::Propagate)
        }
    }
}
