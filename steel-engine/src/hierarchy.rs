use crate::edit::Edit;
use shipyard::{
    AddComponent, AllStoragesViewMut, Component, EntitiesView, EntityId, Get, Remove, Unique,
    UniqueViewMut, ViewMut,
};
use std::collections::HashSet;
use steel_common::data::{Data, Limit, Value};

/// Stores the parent of this entity in the hierarchy tree. An entity without Parent component is at the top level.
/// You can use *parent to dereference the Parent component to get the entity id of parent.
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Default)]
#[track(All)]
pub struct Parent(EntityId);

impl std::ops::Deref for Parent {
    type Target = EntityId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Edit for Parent {
    fn name() -> &'static str {
        "Parent"
    }

    fn get_data(&self) -> Data {
        Data::new().insert_with_limit("unnamed-0", Value::Entity(self.0), Limit::ReadOnly)
    }

    fn load_data(&mut self, data: &Data) {
        if let Some(Value::Entity(v)) = data.get("unnamed-0") {
            self.0 = *v;
        }
    }
}

/// Stores all children of this entity in the hierarchy tree.
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Default)]
pub struct Children(Vec<EntityId>);

impl std::ops::Deref for Children {
    type Target = Vec<EntityId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for Children {
    type Item = <Vec<EntityId> as IntoIterator>::Item;
    type IntoIter = <Vec<EntityId> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = <&'a Vec<EntityId> as IntoIterator>::Item;
    type IntoIter = <&'a Vec<EntityId> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl Edit for Children {
    fn name() -> &'static str {
        "Children"
    }

    fn get_data(&self) -> Data {
        Data::new().insert_with_limit(
            "unnamed-0",
            Value::VecEntity(self.0.clone()),
            Limit::ReadOnly,
        )
    }

    fn load_data(&mut self, data: &Data) {
        if let Some(Value::VecEntity(v)) = data.get("unnamed-0") {
            self.0 = v.clone();
        }
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
    fn name() -> &'static str {
        "Hierarchy"
    }

    fn get_data(&self) -> Data {
        Data::new().insert_with_limit(
            "roots",
            Value::VecEntity(self.roots.clone()),
            Limit::ReadOnly,
        )
    }

    fn load_data(&mut self, data: &Data) {
        if let Some(Value::VecEntity(v)) = data.get("roots") {
            self.roots = v.clone();
        }
    }
}

/// The hierarchy maintain system deals with:
/// * For newly created entities, attach them to the top layer.
/// * For deleted entities, dettach them from their parents and delete their ancestors.
///
/// After running this system:
/// * Dead entities in hierarchy root list must be deleted.
/// * Entities with a Parent component must be added to the Children component of their parent entity.
/// * Entities without a Parent component must be added to the root list of the Hierarchy unique.
/// * Entities which are not in hierarchy are deleted.
pub fn hierarchy_maintain_system(mut all_storages: AllStoragesViewMut) {
    let entities_to_delete = all_storages.run(
        |mut hierarchy: UniqueViewMut<Hierarchy>,
         mut childrens: ViewMut<Children>,
         parents: ViewMut<Parent>,
         entities: EntitiesView| {
            // remove dead entities in hierarchy root list
            for i in (0..hierarchy.roots.len()).rev() {
                if !entities.is_alive(hierarchy.roots[i]) {
                    hierarchy.roots.remove(i);
                }
            }

            // remove dead entities with Parent component from Children component
            for (e, parent) in parents.deleted() {
                if let Ok(mut children) = (&mut childrens).get(**parent) {
                    if let Some(i) = children.iter().position(|c| *c == e) {
                        children.0.remove(i);
                        if children.is_empty() {
                            childrens.remove(**parent);
                        }
                    }
                }
            }

            // entities without a Parent component must be added to the root list of the Hierarchy unique
            let mut root_set = HashSet::new();
            hierarchy.roots.iter().for_each(|&e| {
                root_set.insert(e);
            });
            let entities_without_parent_component = entities
                .iter()
                .filter(|&e| !parents.contains(e))
                .collect::<Vec<_>>();
            for e in entities_without_parent_component {
                if !root_set.contains(&e) {
                    hierarchy.roots.push(e);
                    // root_set.insert(e); // if root_set is needed later, this line must be uncommented
                }
            }

            // find entities which are not in hierarchy
            let (mut alive, mut dead) = (HashSet::new(), HashSet::new());
            for eid in entities.iter() {
                check_in_hierarchy(eid, &mut alive, &mut dead, &parents, &childrens, &entities);
            }
            dead
        },
    );

    // delete them, this is used to delete all descendants of any deleted entity
    for eid in entities_to_delete {
        all_storages.delete_entity(eid);
    }

    // clear track data
    all_storages.run(clear_track_data_system);
}

pub(crate) fn clear_track_data_system(mut parents: ViewMut<Parent>) {
    parents.clear_all_removed_and_deleted();
    parents.clear_all_inserted_and_modified();
}

fn check_in_hierarchy(
    eid: EntityId,
    alive: &mut HashSet<EntityId>,
    dead: &mut HashSet<EntityId>,
    parents: &ViewMut<Parent>,
    childrens: &ViewMut<Children>,
    entities: &EntitiesView,
) -> bool {
    // already checked entity
    if alive.contains(&eid) {
        return true;
    } else if dead.contains(&eid) {
        return false;
    }

    // not yet checked, check now
    let in_hierarchy = if let Ok(parent) = parents.get(eid) {
        if entities.is_alive(**parent) {
            // alive or dead depends on parent
            check_in_hierarchy(**parent, alive, dead, parents, childrens, entities)
        } else {
            // parent is dead
            false
        }
    } else {
        // eid is a root entity in top level
        true
    };

    // remember check result and return
    if in_hierarchy {
        alive.insert(eid);
    } else {
        dead.insert(eid);
    }
    in_hierarchy
}

/// Dettach a child form its parent.
fn dettach(
    hierarchy: &mut Hierarchy,
    childrens: &mut ViewMut<Children>,
    parents: &mut ViewMut<Parent>,
    eid: EntityId,
) {
    // remove the Parent component if exists
    if let Some(parent) = parents.remove(eid) {
        // retrieve and update Children component of it's parent
        if let Ok(mut children) = childrens.get(*parent) {
            if let Some(i) = children.iter().position(|e| *e == eid) {
                children.0.remove(i);
                if children.is_empty() {
                    childrens.remove(*parent);
                }
            }
        }
    } else {
        // child is at the top level, update root list of Hierarchy unique
        if let Some(i) = hierarchy.roots.iter().position(|e| *e == eid) {
            hierarchy.roots.remove(i);
        }
    }
}

/// Attach a child to a parent previous to before entity. If before is EntityId::dead(), attach as the last child.
/// # Example
/// ```rust
/// use shipyard::{EntitiesViewMut, EntityId, UniqueView, UniqueViewMut, ViewMut};
/// use steel::{
///     hierarchy::{Children, Hierarchy, Parent},
///     input::Input,
/// };
/// fn my_system(
///     mut hierarchy: UniqueViewMut<Hierarchy>,
///     mut childrens: ViewMut<Children>,
///     mut parents: ViewMut<Parent>,
///     mut entities: EntitiesViewMut,
///     input: UniqueView<Input>,
/// ) {
///     if input.mouse_pressed(0) {
///         let child = entities.add_entity((), ());
///         let parent = entities.add_entity((), ());
///         steel::hierarchy::attach_before(
///             &mut hierarchy,
///             &mut childrens,
///             &mut parents,
///             child,
///             parent,
///             EntityId::dead(),
///         );
///     }
/// }
/// ```
pub fn attach_before(
    hierarchy: &mut Hierarchy,
    childrens: &mut ViewMut<Children>,
    parents: &mut ViewMut<Parent>,
    eid: EntityId,
    parent: EntityId,
    before: EntityId,
) {
    log::trace!("Attach {eid:?} to {parent:?} before {before:?}");
    attach(hierarchy, childrens, parents, eid, parent, before, true);
}

/// Attach a child to a parent next to after entity. If after is EntityId::dead(), attach as the first child.
/// # Example
/// ```rust
/// use shipyard::{EntitiesViewMut, EntityId, UniqueView, UniqueViewMut, ViewMut};
/// use steel::{
///     hierarchy::{Children, Hierarchy, Parent},
///     input::Input,
/// };
/// fn my_system(
///     mut hierarchy: UniqueViewMut<Hierarchy>,
///     mut childrens: ViewMut<Children>,
///     mut parents: ViewMut<Parent>,
///     mut entities: EntitiesViewMut,
///     input: UniqueView<Input>,
/// ) {
///     if input.mouse_pressed(0) {
///         let child = entities.add_entity((), ());
///         let parent = entities.add_entity((), ());
///         steel::hierarchy::attach_after(
///             &mut hierarchy,
///             &mut childrens,
///             &mut parents,
///             child,
///             parent,
///             EntityId::dead(),
///         );
///     }
/// }
/// ```
pub fn attach_after(
    hierarchy: &mut Hierarchy,
    childrens: &mut ViewMut<Children>,
    parents: &mut ViewMut<Parent>,
    eid: EntityId,
    parent: EntityId,
    after: EntityId,
) {
    log::trace!("Attach {eid:?} to {parent:?} after {after:?}");
    attach(hierarchy, childrens, parents, eid, parent, after, false);
}

/// Attach a child to a parent adjacent to adjacent entity.
/// ### If prev is true:
/// attach previous to adjacent. If adjacent is EntityId::dead(), attach as the last child.
/// ### If prev is false:
/// attach next to adjacent. If adjacent is EntityId::dead(), attach as the first child.
fn attach(
    hierarchy: &mut Hierarchy,
    childrens: &mut ViewMut<Children>,
    parents: &mut ViewMut<Parent>,
    eid: EntityId,
    parent: EntityId,
    adjacent: EntityId,
    prev: bool,
) {
    // the entity we want to attach might already be attached to another parent
    dettach(hierarchy, childrens, parents, eid);

    if parent == EntityId::dead() {
        // attach to the top level
        let i = get_insert_position(adjacent, prev, hierarchy.roots.iter());
        hierarchy.roots.insert(i, eid);
    } else {
        if let Ok(mut children) = childrens.get(parent) {
            // the parent entity already has a Children component
            let i = get_insert_position(adjacent, prev, children.iter());
            children.0.insert(i, eid);
        } else {
            // in this case our parent entity is missing a Children component
            childrens.add_component_unchecked(parent, Children(vec![eid]));
        }
        parents.add_component_unchecked(eid, Parent(parent));
    }
}

fn get_insert_position<'a>(
    adjacent: EntityId,
    prev: bool,
    mut iter: impl ExactSizeIterator<Item = &'a EntityId>,
) -> usize {
    if prev {
        if adjacent == EntityId::dead() {
            iter.len()
        } else {
            iter.position(|e| *e == adjacent).unwrap()
        }
    } else {
        if adjacent == EntityId::dead() {
            0
        } else {
            iter.position(|e| *e == adjacent).unwrap() + 1
        }
    }
}
