use steel_common::data::{Data, Value, Limit};
use shipyard::{Component, EntitiesViewMut, EntityId, Get, Remove, Unique, UniqueViewMut, ViewMut, World};
use crate::edit::Edit;

/// A parent in the hierarchy tree.
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Edit, Default)]
pub struct Parent {
    #[edit(limit = "Limit::ReadOnly")]
    children: Vec<EntityId>,
}

impl Parent {
    /// List of children.
    pub fn children(&self) -> &Vec<EntityId> {
        &self.children
    }
}

/// A child in the hierarchy tree. An entity is at the top level if the parent field of Child component is EntityId::dead().
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Edit, Default)]
pub struct Child {
    #[edit(limit = "Limit::ReadOnly")]
    parent: EntityId,
}

impl Child {
    /// The parent of this child. If parent is EntityId::dead(), this entity is a root entity which is at top level.
    pub fn parent(&self) -> EntityId {
        self.parent
    }
}

/// The Hierarchy unique stores all root entities at the top level.
#[derive(Unique, Edit, Default)]
pub struct Hierarchy {
    #[edit(limit = "Limit::ReadOnly")]
    roots: Vec<EntityId>,
}

impl Hierarchy {
    /// Get all root entities at the top level.
    pub fn roots(&self) -> &Vec<EntityId> {
        &self.roots
    }
}

/// The hierarchy maintain system. After running this system:
/// * All entities must have a Child component.
pub fn hierarchy_maintain_system(mut hierarchy: UniqueViewMut<Hierarchy>, parents: ViewMut<Parent>, mut children: ViewMut<Child>, entities: EntitiesViewMut) {
    // all entities must have a Child component
    let entities_without_child_component = entities.iter().filter(|eid| !children.contains(*eid)).collect::<Vec<_>>();
    for eid in entities_without_child_component {
        hierarchy.roots.push(eid);
        entities.add_component(eid, &mut children, Child { parent: EntityId::dead() });
    }
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
    log::debug!("Attach {eid:?} to {parent:?} before {before:?}");

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
