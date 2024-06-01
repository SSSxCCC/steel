use steel_common::data::{Data, Value, Limit};
use shipyard::{Component, EntitiesViewMut, EntityId, Get, Remove, Unique, UniqueViewMut, ViewMut, World};
use crate::edit::Edit;

/// A parent in the hierarchy tree. You can use Parent.first_child to find its first child.
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Edit, Default)]
pub struct Parent {
    #[edit(limit = "Limit::ReadOnly")]
    num_children: u32,
    #[edit(limit = "Limit::ReadOnly")]
    first_child: EntityId,
}

impl Parent {
    /// Number of children.
    pub fn num_children(&self) -> u32 {
        self.num_children
    }

    /// The first child. If number of children is zero, the Parent component is removed,
    /// so first_child should be always valid.
    pub fn first_child(&self) -> EntityId {
        self.first_child
    }
}

/// A child in the hierarchy tree. An entity is at the top level if the parent of Child component is EntityId::dead().
/// The Child is a node in circular linked list, you can use Child.prev and Child.next to find adjcent children.
/// If this Child dose not have siblings in this level, prev and next are the entity itself.
///
/// Warning: Users should not add or remove this component, otherwise a panic will occur.
#[derive(Component, Edit, Default)]
pub struct Child {
    #[edit(limit = "Limit::ReadOnly")]
    parent: EntityId,
    #[edit(limit = "Limit::ReadOnly")]
    prev: EntityId,
    #[edit(limit = "Limit::ReadOnly")]
    next: EntityId,
}

impl Child {
    /// The parent of this child. If parent is EntityId::dead(), this entity is at top level.
    pub fn parent(&self) -> EntityId {
        self.parent
    }

    /// The previous entity adjcent to this Child. If this Child dose not have siblings,
    /// prev is the entity itself.
    pub fn prev(&self) -> EntityId {
        self.prev
    }

    /// The next entity adjcent to this Child. If this Child dose not have siblings,
    /// next is the entity itself.
    pub fn next(&self) -> EntityId {
        self.next
    }
}

/// The Hierarchy unique marks the first root entity in the ecs world.
#[derive(Unique, Edit, Default)]
pub struct Hierarchy {
    #[edit(limit = "Limit::ReadOnly")]
    num_children: u32,
    #[edit(limit = "Limit::ReadOnly")]
    first_child: EntityId,
}

impl Hierarchy {
    /// Number of children in the top level.
    pub fn num_children(&self) -> u32 {
        self.num_children
    }

    /// The first child in the top level, this is Entity::dead() if there is no entity in the world.
    fn first_child(&self) -> EntityId {
        self.first_child
    }
}

/// The hierarchy maintain system. After running this system:
/// * The next and prev of all Child components must not be Entity::dead().
/// * All entities must have a Child component.
/// * Hierarchy.first should be a valid entity if there is at least one entity in the world.
/// * Hierarchy.first should be EntityId::dead() if there is no entity in the world.
pub fn hierarchy_maintain_system(mut hierarchy: UniqueViewMut<Hierarchy>, parents: ViewMut<Parent>, mut children: ViewMut<Child>, entities: EntitiesViewMut) {
    // all entities must have a Child component
    let entities_without_child_component = entities.iter().filter(|eid| !children.contains(*eid)).collect::<Vec<_>>();
    for eid in entities_without_child_component {
        let (prev, next) = if hierarchy.first_child == EntityId::dead() {
            hierarchy.first_child = eid;
            (eid, eid)
        } else {
            let prev = children[hierarchy.first_child].prev;
            let next = hierarchy.first_child;
            children[prev].next = eid;
            children[next].prev = eid;
            (prev, next)
        };
        entities.add_component(eid, &mut children, Child { parent: EntityId::dead(), prev, next });
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
                parent.num_children -= 1;
                if parent.num_children == 0 { // if the number of children is zero, the Parent component must be removed
                    parents.remove(child.parent);
                } else {
                    // the ancestor still has children, and we have to change some linking
                    // check if we have to change first_child
                    if parent.first_child == eid {
                        parent.first_child = child.next;
                    }
                    // remove the detached child from the sibling chain
                    // next and prev must not be Entity::dead() after hierarchy_maintain_system
                    children[child.prev].next = child.next;
                    children[child.next].prev = child.prev;
                }
            } else if child.parent == EntityId::dead() { // child is at the top level
                hierarchy.num_children -= 1;
                if hierarchy.num_children == 0 { // if the number of children is zero, the hierarchy.first should be EntityId::dead()
                    hierarchy.first_child = EntityId::dead();
                } else {
                    // there are still children in top level, and we have to change some linking
                    // check if we have to change hierarchy.first
                    if hierarchy.first_child == eid {
                        hierarchy.first_child = child.next;
                    }
                    // remove the detached child from the sibling chain
                    // next and prev must not be Entity::dead() after hierarchy_maintain_system
                    children[child.prev].next = child.next;
                    children[child.next].prev = child.prev;
                }
            }
        }
    });
}

/// Attach a Child to a Parent next to pre entity. If pre is EntityId::dead(), attach as the first child.
/// This function must be called after hierarchy_maintain_system, or may panic.
pub(crate) fn attach(world: &mut World, eid: EntityId, parent: EntityId, pre: EntityId) {
    // the entity we want to attach might already be attached to another parent
    dettach(world, eid);

    world.run(|mut hierarchy: UniqueViewMut<Hierarchy>, mut parents: ViewMut<Parent>, mut children: ViewMut<Child>, entities: EntitiesViewMut| {
        if let Ok(p) = (&mut parents).get(parent) { // the parent entity already has a Parent component
            p.num_children += 1;

            // get the ids of the new previous and next siblings of our new child
            let (prev, next) = if pre == EntityId::dead() { // attach as the first child
                let first_child = p.first_child;
                p.first_child = eid;
                (children[first_child].prev, first_child)
            } else { // attach next to the pre
                (pre, children[pre].next)
            };

            // change the linking
            children[prev].next = eid;
            children[next].prev = eid;

            // add the Child component to the new entity
            entities.add_component(eid, children, Child { parent, prev, next });
        } else if parent == EntityId::dead() { // attach to the top level
            hierarchy.num_children += 1;

            // get the ids of the new previous and next siblings of our new child
            let (prev, next) = if pre == EntityId::dead() { // attach as the first child
                let first_child = hierarchy.first_child;
                hierarchy.first_child = eid;
                if first_child == EntityId::dead() {
                    (eid, eid) // no siblings
                } else {
                    (children[first_child].prev, first_child)
                }
            } else { // attach next to the pre
                (pre, children[pre].next)
            };

            if prev != eid { // change the linking if there is any sibling
                children[prev].next = eid;
                children[next].prev = eid;
            }

            // add the Child component to the new entity
            entities.add_component(eid, children, Child { parent, prev, next });
        } else { // in this case our parent entity is missing a Parent component
            // we don't need to change any links, just insert both components
            entities.add_component(eid, children, Child { parent, prev: eid, next: eid });
            entities.add_component(parent, parents, Parent { num_children: 1, first_child: eid });
        }
    });
}
