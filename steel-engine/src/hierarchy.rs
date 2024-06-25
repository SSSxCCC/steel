use std::collections::HashSet;
use steel_common::data::{Data, Value, Limit};
use shipyard::{AllStoragesViewMut, Component, EntitiesViewMut, EntityId, Get, Remove, Unique, UniqueViewMut, ViewMut, World};
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
#[track(Deletion)]
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
/// * All entities must have a Child component.
/// * All entities in children list of Parent compnents must be alive.
/// * Entities which are not in hierarchy are deleted.
pub fn hierarchy_maintain_system(mut all_storages: AllStoragesViewMut) {
    let entities_to_delete = all_storages.run(|mut hierarchy: UniqueViewMut<Hierarchy>,
            mut parents: ViewMut<Parent>, mut children: ViewMut<Child>, entities: EntitiesViewMut| {
        // all entities must have a Child component
        let entities_without_child_component = entities.iter().filter(|eid| !children.contains(*eid)).collect::<Vec<_>>();
        for eid in entities_without_child_component {
            log::trace!("hierarchy_maintain_system: Add Child component for {eid:?}");
            hierarchy.roots.push(eid);
            entities.add_component(eid, &mut children, Child { parent: EntityId::dead() });
        }

        // remove deleted entities with Child component from children list of Parent component
        for (eid, child) in children.deleted() {
            log::trace!("hierarchy_maintain_system: Deleted child eid: {eid:?}");
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

    // delete them
    for eid in entities_to_delete {
        all_storages.delete_entity(eid);
    }

    // clear track data
    all_storages.run(clear_track_data_system);
}

pub(crate) fn clear_track_data_system(mut children: ViewMut<Child>) {
    children.clear_all_deleted();
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
                parent.children.remove(parent.children.iter().position(|e| *e == eid).unwrap());
                if parent.children.is_empty() {
                    parents.remove(child.parent);
                }
            } else if child.parent == EntityId::dead() { // child is at the top level
                let i = hierarchy.roots.iter().position(|e| *e == eid).unwrap();
                hierarchy.roots.remove(i);
            }
        }
    });
}

/// Attach a Child to a Parent previous to before entity. If before is EntityId::dead(), attach as the last child.
/// This function must be called after hierarchy_maintain_system, or may panic.
pub(crate) fn attach(world: &mut World, eid: EntityId, parent: EntityId, before: EntityId) {
    log::trace!("Attach {eid:?} to {parent:?} before {before:?}");

    // the entity we want to attach might already be attached to another parent
    dettach(world, eid);

    world.run(|mut hierarchy: UniqueViewMut<Hierarchy>, mut parents: ViewMut<Parent>, children: ViewMut<Child>, entities: EntitiesViewMut| {
        if let Ok(p) = (&mut parents).get(parent) { // the parent entity already has a Parent component
            let i = if before == EntityId::dead() { p.children.len() } else { p.children.iter().position(|e| *e == before).unwrap() };
            p.children.insert(i, eid);
        } else if parent == EntityId::dead() { // attach to the top level
            let i = if before == EntityId::dead() { hierarchy.roots.len() } else { hierarchy.roots.iter().position(|e| *e == before).unwrap() };
            hierarchy.roots.insert(i, eid);
        } else { // in this case our parent entity is missing a Parent component
            entities.add_component(parent, parents, Parent { children: vec![eid] });
        }
        entities.add_component(eid, children, Child { parent });
    });
}
