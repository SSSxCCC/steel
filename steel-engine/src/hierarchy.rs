use std::collections::HashSet;
use steel_common::data::{Data, Value, Limit};
use shipyard::{AddComponent, AllStoragesViewMut, Component, EntitiesViewMut, EntityId, Get, IntoIter, IntoWithId, Remove, Unique, UniqueViewMut, ViewMut, World};
use crate::edit::Edit;

/// A parent in the hierarchy tree.
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Default)]
pub struct Parent {
    children: Vec<EntityId>,
}

impl Parent {
    /// List of children.
    pub fn children(&self) -> &Vec<EntityId> {
        &self.children
    }
}

impl Edit for Parent {
    fn name() -> &'static str { "Parent" }

    fn get_data(&self) -> Data {
        Data::new().insert_with_limit("children", Value::VecEntity(self.children.clone()), Limit::ReadOnly)
    }

    fn load_data(&mut self, data: &Data) {
        if let Some(Value::VecEntity(v)) = data.get("children") { self.children = v.clone(); }
    }
}

/// A child in the hierarchy tree. An entity is at the top level if the parent field of Child component is EntityId::dead().
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Default)]
#[track(All)]
pub struct Child {
    parent: EntityId,
}

impl Child {
    /// The parent of this child. If parent is EntityId::dead(), this entity is a root entity which is at top level.
    pub fn parent(&self) -> EntityId {
        self.parent
    }
}

impl Edit for Child {
    fn name() -> &'static str { "Child" }

    fn get_data(&self) -> Data {
        Data::new().insert_with_limit("parent", Value::Entity(self.parent), Limit::ReadOnly)
    }

    fn load_data(&mut self, data: &Data) {
        if let Some(Value::Entity(v)) = data.get("parent") { self.parent = *v; }
    }
}

/// The Hierarchy unique stores all root entities at the top level.
#[derive(Unique, Default)]
pub struct Hierarchy {
    roots: Vec<EntityId>,
}

impl Hierarchy {
    /// Get all root entities at the top level.
    pub fn roots(&self) -> &Vec<EntityId> {
        &self.roots
    }
}

impl Edit for Hierarchy {
    fn name() -> &'static str { "Hierarchy" }

    fn get_data(&self) -> Data {
        Data::new().insert_with_limit("roots", Value::VecEntity(self.roots.clone()), Limit::ReadOnly)
    }

    fn load_data(&mut self, data: &Data) {
        if let Some(Value::VecEntity(v)) = data.get("roots") { self.roots = v.clone(); }
    }
}

/// The hierarchy maintain system. After running this system:
/// * All children list of Parent components and Hierarchy unique must not miss any child.
/// * All entities must have a Child component.
/// * All entities in children list of Parent compnents must be alive.
/// * Entities which are not in hierarchy are deleted.
pub fn hierarchy_maintain_system(mut all_storages: AllStoragesViewMut) {
    let entities_to_delete = all_storages.run(|mut hierarchy: UniqueViewMut<Hierarchy>,
            mut parents: ViewMut<Parent>, mut children: ViewMut<Child>, entities: EntitiesViewMut| {
        // add entities which have newly created Child component to children list of Parent component
        let mut root_set = HashSet::new();
        for (eid, child) in children.inserted().iter().with_id() {
            if child.parent == EntityId::dead() {
                if root_set.is_empty() {
                    hierarchy.roots.iter().for_each(|e| { root_set.insert(*e); });
                }
                if !root_set.contains(&eid) {
                    root_set.insert(eid);
                    hierarchy.roots.push(eid);
                }
            } else {
                if !parents.contains(child.parent) {
                    parents.add_component_unchecked(child.parent, Parent::default());
                }
                let parent = (&mut parents).get(child.parent).unwrap();
                if !parent.children.contains(&eid) { // TODO: check in O(1)
                    parent.children.push(eid);
                }
            }
        }

        // all entities must have a Child component
        let entities_without_child_component = entities.iter().filter(|eid| !children.contains(*eid)).collect::<Vec<_>>();
        for eid in entities_without_child_component {
            hierarchy.roots.push(eid);
            entities.add_component(eid, &mut children, Child { parent: EntityId::dead() });
        }

        // remove deleted entities with Child component from children list of Parent component
        for (eid, child) in children.deleted() {
            if child.parent == EntityId::dead() {
                if let Some(i) = hierarchy.roots.iter().position(|c| *c == eid) {
                    hierarchy.roots.remove(i);
                }
            } else if let Ok(parent) = (&mut parents).get(child.parent) {
                if let Some(i) = parent.children.iter().position(|c| *c == eid) {
                    parent.children.remove(i);
                    if parent.children.is_empty() {
                        parents.remove(child.parent);
                    }
                }
            }
        }

        // find entities which are not in hierarchy
        let (mut alive, mut dead) = (HashSet::new(), HashSet::new());
        for eid in entities.iter() {
            check_in_hierarchy(eid, &mut alive, &mut dead, &children, &parents, &entities);
        }
        dead
    });

    // delete them, this is used to delete all descendants of any deleted entity
    for eid in entities_to_delete {
        all_storages.delete_entity(eid);
    }

    // clear track data
    all_storages.run(clear_track_data_system);
}

pub(crate) fn clear_track_data_system(mut children: ViewMut<Child>) {
    children.clear_all_removed_and_deleted();
    children.clear_all_inserted_and_modified();
}

fn check_in_hierarchy(eid: EntityId, alive: &mut HashSet<EntityId>, dead: &mut HashSet<EntityId>,
        children: &ViewMut<Child>, parents: &ViewMut<Parent>, entities: &EntitiesViewMut) -> bool {
    // already checked entity
    if alive.contains(&eid) {
        return true;
    } else if dead.contains(&eid) {
        return false;
    }

    // not yet checked, check now
    let child = children.get(eid).expect(format!("No Child component in entity {eid:?}").as_str());
    let in_hierarchy = if child.parent == EntityId::dead() { // eid is a root entity in top level
        true
    } else if entities.is_alive(child.parent) { // alive or dead depends on parent
        check_in_hierarchy(child.parent, alive, dead, children, parents, entities)
    } else { // parent is dead
        false
    };

    // remember check result and return
    if in_hierarchy {
        alive.insert(eid);
    } else {
        dead.insert(eid);
    }
    in_hierarchy
}

/// Dettach a Child form its Parent.
/// This function must be called after hierarchy_maintain_system, or may panic.
fn dettach(world: &mut World, eid: EntityId) {
    world.run(|mut hierarchy: UniqueViewMut<Hierarchy>, mut parents: ViewMut<Parent>, mut children: ViewMut<Child>| {
        // remove the Child component - if nonexistent, do nothing
        if let Some(child) = children.remove(eid) {
            // retrieve and update Parent component from ancestor
            if let Ok(parent) = (&mut parents).get(child.parent) {
                if let Some(i) = parent.children.iter().position(|e| *e == eid) {
                    parent.children.remove(i);
                    if parent.children.is_empty() {
                        parents.remove(child.parent);
                    }
                }
            } else if child.parent == EntityId::dead() { // child is at the top level
                if let Some(i) = hierarchy.roots.iter().position(|e| *e == eid) {
                    hierarchy.roots.remove(i);
                }
            }
        }
    });
}

/// Attach a Child to a Parent previous to before entity. If before is EntityId::dead(), attach as the last child.
/// This function must be called after hierarchy_maintain_system, or may panic.
pub(crate) fn attach_before(world: &mut World, eid: EntityId, parent: EntityId, before: EntityId) {
    log::trace!("Attach {eid:?} to {parent:?} before {before:?}");
    attach(world, eid, parent, before, true);
}

/// Attach a Child to a Parent next to after entity. If after is EntityId::dead(), attach as the first child.
/// This function must be called after hierarchy_maintain_system, or may panic.
pub(crate) fn attach_after(world: &mut World, eid: EntityId, parent: EntityId, after: EntityId) {
    log::trace!("Attach {eid:?} to {parent:?} after {after:?}");
    attach(world, eid, parent, after, false);
}

/// Attach a Child to a Parent adjacent to adjacent entity.
/// ### If prev is true:
/// attach previous to adjacent. If adjacent is EntityId::dead(), attach as the last child.
/// ### If prev is false:
/// attach next to adjacent. If adjacent is EntityId::dead(), attach as the first child.
fn attach(world: &mut World, eid: EntityId, parent: EntityId, adjacent: EntityId, prev: bool) {
    // the entity we want to attach might already be attached to another parent
    dettach(world, eid);

    world.run(|mut hierarchy: UniqueViewMut<Hierarchy>, mut parents: ViewMut<Parent>, children: ViewMut<Child>, entities: EntitiesViewMut| {
        if let Ok(p) = (&mut parents).get(parent) { // the parent entity already has a Parent component
            let i = get_insert_position(adjacent, prev, p.children.iter());
            p.children.insert(i, eid);
        } else if parent == EntityId::dead() { // attach to the top level
            let i = get_insert_position(adjacent, prev, hierarchy.roots.iter());
            hierarchy.roots.insert(i, eid);
        } else { // in this case our parent entity is missing a Parent component
            entities.add_component(parent, parents, Parent { children: vec![eid] });
        }
        entities.add_component(eid, children, Child { parent });
    });
}

fn get_insert_position<'a>(adjacent: EntityId, prev: bool, mut iter: impl ExactSizeIterator<Item = &'a EntityId>) -> usize {
    if prev {
        if adjacent == EntityId::dead() { iter.len() } else { iter.position(|e| *e == adjacent).unwrap() }
    } else {
        if adjacent == EntityId::dead() { 0 } else { iter.position(|e| *e == adjacent).unwrap() + 1 }
    }
}
