use crate::locale::Texts;
use glam::{Vec3, Vec4};
use shipyard::EntityId;
use std::{collections::HashMap, ops::RangeInclusive};
use steel_common::{
    app::{App, Command, CommandMut},
    data::{Data, EntitiesData, EntityData, Limit, Value, WorldData},
};

pub struct DataWindow {
    selected_entity: EntityId,
    selected_unique: String,
}

impl DataWindow {
    pub fn new() -> Self {
        DataWindow {
            selected_entity: EntityId::dead(),
            selected_unique: String::new(),
        }
    }

    pub fn entity_component_windows(
        &mut self,
        ctx: &egui::Context,
        world_data: &mut WorldData,
        app: &mut Box<dyn App>,
        texts: &Texts,
    ) {
        egui::Window::new(texts.get("Entities")).show(&ctx, |ui| {
            self.entities_view(ui, world_data, app, texts);
        });

        if let Some(entity_data) = world_data.entities.get_mut(&self.selected_entity) {
            egui::Window::new(texts.get("Components")).show(&ctx, |ui| {
                self.entity_view(ui, entity_data, app);
            });
        }
    }

    pub fn entities_view(
        &mut self,
        ui: &mut egui::Ui,
        world_data: &WorldData,
        app: &mut Box<dyn App>,
        texts: &Texts,
    ) {
        let hierarchy = world_data
            .uniques
            .get("Hierarchy")
            .expect("Hierarchy unique is missing!");
        let root_entities = match hierarchy.get("roots") {
            Some(Value::VecEntity(v)) => v,
            _ => panic!("Hierarchy does not have roots!"),
        };

        if !root_entities.is_empty() {
            let (mut drag_entity, mut drop_parent, mut drop_before) =
                (EntityId::dead(), None, EntityId::dead());
            self.entity_level(
                root_entities,
                EntityId::dead(),
                ui,
                world_data,
                app,
                &mut drag_entity,
                &mut drop_parent,
                &mut drop_before,
                texts,
            );
            if let Some(drop_parent) = drop_parent {
                if drag_entity != EntityId::dead() && ui.input(|input| input.pointer.any_released())
                {
                    app.command_mut(CommandMut::AttachBefore(
                        drag_entity,
                        drop_parent,
                        drop_before,
                    ));
                }
            }
        } else if !world_data.entities.is_empty() {
            panic!("entities_view: hierarchy.roots is empty but world_data.entities is not empty! world_data.entities={:?}", world_data.entities);
        }

        if ui.button("+").clicked() {
            app.command_mut(CommandMut::CreateEntity);
        }
    }

    fn entity_level(
        &mut self,
        entities: &Vec<EntityId>,
        parent: EntityId,
        ui: &mut egui::Ui,
        world_data: &WorldData,
        app: &mut Box<dyn App>,
        drag_entity: &mut EntityId,
        drop_parent: &mut Option<EntityId>,
        drop_before: &mut EntityId,
        texts: &Texts,
    ) {
        for (i, entity) in entities.iter().enumerate() {
            let entity = *entity;
            let entity_data = world_data
                .entities
                .get(&entity)
                .expect(format!("entity_level: non-existent entity: {entity:?}").as_str());

            let mut entity_item = |ui: &mut egui::Ui| {
                let drag_id = egui::Id::new(entity);
                if ui.memory(|mem| mem.is_being_dragged(drag_id)) {
                    *drag_entity = entity;
                }

                let can_accept_what_is_being_dragged = entity != *drag_entity;
                let can_insert_before = true;
                let can_insert_after = i == entities.len() - 1;

                let drop_result = Self::drop_target(
                    ui,
                    can_accept_what_is_being_dragged,
                    can_insert_before,
                    can_insert_after,
                    |ui| {
                        Self::drag_source(ui, drag_id, |ui| {
                            let r = ui.selectable_label(
                                self.selected_entity == entity,
                                Self::entity_label(&entity, entity_data),
                            );
                            if r.clicked() {
                                self.selected_entity = entity;
                            }
                            r.context_menu(|ui| {
                                if ui.button(texts.get("Duplicate")).clicked() {
                                    log::info!("entity_context_menu->Duplicate");
                                    Self::duplicate_entity(entity, world_data, app);
                                    ui.close_menu();
                                }
                                if ui.button(texts.get("Delete")).clicked() {
                                    log::info!("entity_context_menu->Delete");
                                    self.delete_entity(entity, app);
                                    ui.close_menu();
                                }
                            });
                        });
                    },
                );

                if let Some(drop_result) = drop_result {
                    match drop_result {
                        DropResult::Before => {
                            *drop_parent = Some(parent);
                            *drop_before = entity;
                        }
                        DropResult::Into => *drop_parent = Some(entity),
                        DropResult::After => {
                            *drop_parent = Some(parent);
                            *drop_before = if i + 1 < entities.len() {
                                entities[i + 1]
                            } else {
                                EntityId::dead()
                            };
                        }
                    }
                }
            };

            if let Some(parent) = entity_data.components.get("Parent") {
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    egui::Id::new(entity),
                    false,
                )
                .show_header(ui, |ui| entity_item(ui))
                .body(|ui| {
                    let children = match parent.get("children") {
                        Some(Value::VecEntity(v)) => v,
                        _ => panic!(
                            "entity_level: no children value in Parent component: {parent:?}"
                        ),
                    };
                    self.entity_level(
                        children,
                        entity,
                        ui,
                        world_data,
                        app,
                        drag_entity,
                        drop_parent,
                        drop_before,
                        texts,
                    )
                });
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(18.0); // align with header, TODO: get correct space value dynamically
                    entity_item(ui);
                });
            }
        }
    }

    pub fn duplicate_entity(entity: EntityId, world_data: &WorldData, app: &mut Box<dyn App>) {
        let mut entities_data = EntitiesData::new();
        let mut entities_to_add = vec![entity];
        while !entities_to_add.is_empty() {
            let mut new_entities_to_add = Vec::new();
            for entity in &entities_to_add {
                let entity_data = world_data.entities.get(entity).expect(
                    format!("entity_level::duplicate: non-existent entity: {entity:?}").as_str(),
                );
                entities_data.insert(*entity, entity_data.clone()); // TODO: avoid clone here
                if let Some(parent) = entity_data.components.get("Parent") {
                    let children = match parent.get("children") {
                        Some(Value::VecEntity(v)) => v,
                        _ => panic!(
                            "duplicate_entity: no children value in Parent component: {parent:?}"
                        ),
                    };
                    for e in children {
                        new_entities_to_add.push(*e);
                    }
                }
            }
            entities_to_add = new_entities_to_add;
        }
        let mut old_id_to_new_id = HashMap::new();
        app.command_mut(CommandMut::AddEntities(
            &entities_data,
            &mut old_id_to_new_id,
        ));
        let new_id = *old_id_to_new_id.get(&entity).unwrap();

        // attach duplicated entity next to the original entity
        let entity_data = world_data
            .entities
            .get(&entity)
            .expect(format!("duplicate_entity: non-existent entity: {entity:?}").as_str());
        let child = entity_data.components.get("Child").expect(
            format!(
                "duplicate_entity: missing Child component in entity: {entity:?} {entity_data:?}"
            )
            .as_str(),
        );
        let parent = match child.get("parent") {
            Some(Value::Entity(e)) => *e,
            _ => panic!("duplicate_entity: no parent value in Child component: {child:?}"),
        };
        app.command_mut(CommandMut::AttachAfter(new_id, parent, entity));
    }

    pub fn delete_entity(&mut self, entity: EntityId, app: &mut Box<dyn App>) {
        app.command_mut(CommandMut::DestroyEntity(entity));
        self.selected_entity = EntityId::dead();
    }

    fn drag_source<R>(ui: &mut egui::Ui, id: egui::Id, body: impl FnOnce(&mut egui::Ui) -> R) {
        let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(id));

        if !is_being_dragged {
            let response = ui.scope(body).response;

            // caculate press time
            let press_time = ui.input(|input| {
                if let Some(press_origin) = input.pointer.press_origin() {
                    if response.rect.contains(press_origin) {
                        if let Some(press_start_time) = input.pointer.press_start_time() {
                            return input.time - press_start_time;
                        }
                    }
                }
                return 0.0;
            });

            // start drag after pressing some time
            if press_time > 0.3 {
                ui.memory_mut(|mem| mem.set_dragged_id(id));
            }
        } else {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            // paint the body to a new layer
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
            let response = ui.with_layer_id(layer_id, body).response;

            // now we move the visuals of the body to where the mouse is
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().translate_layer(layer_id, delta);
            }
        }
    }

    fn drop_target<R>(
        ui: &mut egui::Ui,
        can_accept_what_is_being_dragged: bool,
        can_insert_before: bool,
        can_insert_after: bool,
        body: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<DropResult> {
        let is_anything_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

        let margin = egui::Vec2::splat(1.0);
        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);
        let where_to_put_background = ui.painter().add(egui::Shape::Noop);
        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
        body(&mut content_ui);
        let outer_rect =
            egui::Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
        let (rect, response) = ui.allocate_at_least(outer_rect.size(), egui::Sense::hover());

        if is_anything_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
            if let Some(hover_pos) = ui.input(|input| input.pointer.hover_pos()) {
                let style = ui.visuals().widgets.active;
                if can_insert_before && hover_pos.y - rect.top() < rect.height() / 4.0 {
                    ui.painter().set(
                        where_to_put_background,
                        egui::epaint::Shape::line_segment(
                            [rect.left_top(), rect.right_top()],
                            style.bg_stroke,
                        ),
                    );
                    return Some(DropResult::Before);
                } else if can_insert_after && hover_pos.y - rect.top() > rect.height() * 3.0 / 4.0 {
                    ui.painter().set(
                        where_to_put_background,
                        egui::epaint::Shape::line_segment(
                            [rect.left_bottom(), rect.right_bottom()],
                            style.bg_stroke,
                        ),
                    );
                    return Some(DropResult::After);
                } else {
                    ui.painter().set(
                        where_to_put_background,
                        egui::epaint::Shape::rect_stroke(rect, style.rounding, style.bg_stroke),
                    );
                    return Some(DropResult::Into);
                }
            }
        }
        None
    }

    fn entity_label(id: &EntityId, entity_data: &EntityData) -> impl Into<egui::WidgetText> {
        if let Some(entity_info) = entity_data.components.get("EntityInfo") {
            if let Some(Value::String(s)) = entity_info.values.get("name") {
                if !s.is_empty() {
                    return format!("{s}");
                }
            }
        }
        format!("{:?}", id)
    }

    pub fn entity_view(
        &mut self,
        ui: &mut egui::Ui,
        entity_data: &mut EntityData,
        app: &mut Box<dyn App>,
    ) {
        for (component_name, component_data) in &mut entity_data.components {
            ui.horizontal(|ui| {
                ui.label(component_name);
                if component_name != "Child" && component_name != "Parent" {
                    // TODO: use a more generic way to prevent some components from being destroyed by user
                    if ui.button("-").clicked() {
                        app.command_mut(CommandMut::DestroyComponent(
                            self.selected_entity,
                            component_name,
                        ));
                    }
                }
            });
            Self::data_view(ui, component_name, component_data);
            if component_name == "EntityInfo" {
                ui.horizontal(|ui| {
                    ui.label("id");
                    Self::color_label(
                        ui,
                        egui::Color32::BLACK,
                        format!("{:?}", self.selected_entity),
                    );
                });
            }
            ui.separator();
        }

        let mut components = Vec::new();
        app.command(Command::GetComponents(&mut components));
        ui.menu_button("+", |ui| {
            for component in components
                .into_iter()
                .filter(|c| *c != "Child" && *c != "Parent")
            {
                // TODO: use a more generic way to prevent some components from being created by user
                if ui.button(component).clicked() {
                    app.command_mut(CommandMut::CreateComponent(self.selected_entity, component));
                    ui.close_menu();
                }
            }
        });
    }

    pub fn data_view(ui: &mut egui::Ui, data_name: &String, data: &mut Data) {
        for (name, value) in &mut data.values {
            ui.horizontal(|ui| {
                ui.label(name);
                if let Some(Limit::ReadOnly) = data.limits.get(name) {
                    let color = egui::Color32::BLACK;
                    match value {
                        Value::Bool(b) => Self::color_label(ui, color, if *b { "☑" } else { "☐" }),
                        Value::Int32(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UInt32(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Float32(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::String(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Entity(v) => Self::color_label(ui, color, format!("{v:?}")),
                        Value::Vec2(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Vec3(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Vec4(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::IVec2(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::IVec3(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::IVec4(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UVec2(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UVec3(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UVec4(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::VecEntity(v) => {
                            ui.vertical(|ui| {
                                for e in v {
                                    Self::color_label(ui, color, format!("{e:?}"));
                                }
                            });
                        }
                    }
                } else {
                    match value {
                        Value::Bool(b) => {
                            ui.checkbox(b, "");
                        }
                        Value::Int32(v) => {
                            if let Some(Limit::Int32Enum(int_enum)) = data.limits.get(name) {
                                if int_enum.len() > 0 {
                                    let mut i = int_enum
                                        .iter()
                                        .enumerate()
                                        .find_map(
                                            |(i, (int, _))| {
                                                if v == int {
                                                    Some(i)
                                                } else {
                                                    None
                                                }
                                            },
                                        )
                                        .unwrap_or(0);
                                    // Use component_name/unique_name + value_name as id to make sure that every id is unique
                                    egui::ComboBox::from_id_source(format!(
                                        "{} {}",
                                        data_name, name
                                    ))
                                    .show_index(
                                        ui,
                                        &mut i,
                                        int_enum.len(),
                                        |i| &int_enum[i].1,
                                    );
                                    *v = int_enum[i].0;
                                } else {
                                    Self::color_label(
                                        ui,
                                        egui::Color32::RED,
                                        "zero length int_enum!",
                                    );
                                }
                            } else {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => Some(range.clone()),
                                    _ => None,
                                };
                                Self::drag_value(ui, v, range);
                            }
                        }
                        Value::UInt32(v) => {
                            let range = match data.limits.get(name) {
                                Some(Limit::UInt32Range(range)) => Some(range.clone()),
                                _ => None,
                            };
                            Self::drag_value(ui, v, range);
                        }
                        Value::String(v) => {
                            if let Some(Limit::StringMultiline) = data.limits.get(name) {
                                ui.text_edit_multiline(v);
                            } else {
                                ui.text_edit_singleline(v);
                            }
                        }
                        Value::Entity(v) => {
                            ui.label(format!("{v:?}")); // TODO: change entity in editor
                        }
                        Value::Float32(v) => {
                            if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                ui.drag_angle(v);
                            } else {
                                Self::drag_float32(
                                    ui,
                                    v,
                                    match data.limits.get(name) {
                                        Some(Limit::Float32Range(range)) => Some(range.clone()),
                                        _ => None,
                                    },
                                );
                            }
                        }
                        Value::Vec2(v) => {
                            ui.horizontal(|ui| {
                                if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                    ui.drag_angle(&mut v.x);
                                    ui.drag_angle(&mut v.y);
                                } else {
                                    let range = match data.limits.get(name) {
                                        Some(Limit::Float32Range(range)) => {
                                            vec![Some(range.clone()); 2]
                                        }
                                        Some(Limit::VecRange(range)) => range.clone(),
                                        _ => Vec::new(),
                                    };
                                    Self::drag_float32(
                                        ui,
                                        &mut v.x,
                                        range.get(0).and_then(|r| r.clone()),
                                    );
                                    Self::drag_float32(
                                        ui,
                                        &mut v.y,
                                        range.get(1).and_then(|r| r.clone()),
                                    );
                                }
                            });
                        }
                        Value::Vec3(v) => {
                            ui.horizontal(|ui| {
                                if let Some(Limit::Vec3Color) = data.limits.get(name) {
                                    let mut color = v.to_array();
                                    ui.color_edit_button_rgb(&mut color);
                                    *v = Vec3::from_array(color);
                                } else if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                    ui.drag_angle(&mut v.x);
                                    ui.drag_angle(&mut v.y);
                                    ui.drag_angle(&mut v.z);
                                } else {
                                    let range = match data.limits.get(name) {
                                        Some(Limit::Float32Range(range)) => {
                                            vec![Some(range.clone()); 3]
                                        }
                                        Some(Limit::VecRange(range)) => range.clone(),
                                        _ => Vec::new(),
                                    };
                                    Self::drag_float32(
                                        ui,
                                        &mut v.x,
                                        range.get(0).and_then(|r| r.clone()),
                                    );
                                    Self::drag_float32(
                                        ui,
                                        &mut v.y,
                                        range.get(1).and_then(|r| r.clone()),
                                    );
                                    Self::drag_float32(
                                        ui,
                                        &mut v.z,
                                        range.get(2).and_then(|r| r.clone()),
                                    );
                                }
                            });
                        }
                        Value::Vec4(v) => {
                            ui.horizontal(|ui| {
                                if let Some(Limit::Vec4Color) = data.limits.get(name) {
                                    let mut color = v.to_array();
                                    ui.color_edit_button_rgba_unmultiplied(&mut color);
                                    *v = Vec4::from_array(color);
                                } else if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                    ui.drag_angle(&mut v.x);
                                    ui.drag_angle(&mut v.y);
                                    ui.drag_angle(&mut v.z);
                                    ui.drag_angle(&mut v.w);
                                } else {
                                    let range = match data.limits.get(name) {
                                        Some(Limit::Float32Range(range)) => {
                                            vec![Some(range.clone()); 4]
                                        }
                                        Some(Limit::VecRange(range)) => range.clone(),
                                        _ => Vec::new(),
                                    };
                                    Self::drag_float32(
                                        ui,
                                        &mut v.x,
                                        range.get(0).and_then(|r| r.clone()),
                                    );
                                    Self::drag_float32(
                                        ui,
                                        &mut v.y,
                                        range.get(1).and_then(|r| r.clone()),
                                    );
                                    Self::drag_float32(
                                        ui,
                                        &mut v.z,
                                        range.get(2).and_then(|r| r.clone()),
                                    );
                                    Self::drag_float32(
                                        ui,
                                        &mut v.w,
                                        range.get(3).and_then(|r| r.clone()),
                                    );
                                }
                            });
                        }
                        Value::IVec2(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 2],
                                    Some(Limit::IVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(
                                    ui,
                                    &mut v.x,
                                    range.get(0).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.y,
                                    range.get(1).and_then(|r| r.clone()),
                                );
                            });
                        }
                        Value::IVec3(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 3],
                                    Some(Limit::IVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(
                                    ui,
                                    &mut v.x,
                                    range.get(0).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.y,
                                    range.get(1).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.z,
                                    range.get(2).and_then(|r| r.clone()),
                                );
                            });
                        }
                        Value::IVec4(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 4],
                                    Some(Limit::IVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(
                                    ui,
                                    &mut v.x,
                                    range.get(0).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.y,
                                    range.get(1).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.z,
                                    range.get(2).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.w,
                                    range.get(3).and_then(|r| r.clone()),
                                );
                            });
                        }
                        Value::UVec2(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 2],
                                    Some(Limit::UVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(
                                    ui,
                                    &mut v.x,
                                    range.get(0).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.y,
                                    range.get(1).and_then(|r| r.clone()),
                                );
                            });
                        }
                        Value::UVec3(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 3],
                                    Some(Limit::UVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(
                                    ui,
                                    &mut v.x,
                                    range.get(0).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.y,
                                    range.get(1).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.z,
                                    range.get(2).and_then(|r| r.clone()),
                                );
                            });
                        }
                        Value::UVec4(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 4],
                                    Some(Limit::UVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(
                                    ui,
                                    &mut v.x,
                                    range.get(0).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.y,
                                    range.get(1).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.z,
                                    range.get(2).and_then(|r| r.clone()),
                                );
                                Self::drag_value(
                                    ui,
                                    &mut v.w,
                                    range.get(3).and_then(|r| r.clone()),
                                );
                            });
                        }
                        Value::VecEntity(v) => {
                            ui.vertical(|ui| {
                                for e in v {
                                    Self::color_label(ui, egui::Color32::BLACK, format!("{e:?}"));
                                    // TODO: add/remove/change entity in editor
                                }
                            });
                        }
                    }
                }
            });
        }
    }

    fn color_label(ui: &mut egui::Ui, color: egui::Color32, text: impl Into<egui::WidgetText>) {
        egui::Frame::none()
            .inner_margin(egui::style::Margin::symmetric(3.0, 1.0))
            .rounding(egui::Rounding::same(3.0))
            .fill(color)
            .show(ui, |ui| ui.label(text));
    }

    fn drag_float32(ui: &mut egui::Ui, v: &mut f32, range: Option<RangeInclusive<f32>>) {
        let mut drag_value = egui::DragValue::new(v).max_decimals(100).speed(0.01);
        if let Some(range) = range {
            drag_value = drag_value.clamp_range(range);
        }
        ui.add(drag_value);
    }

    fn drag_value<V: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        v: &mut V,
        range: Option<RangeInclusive<V>>,
    ) {
        let mut drag_value = egui::DragValue::new(v);
        if let Some(range) = range {
            drag_value = drag_value.clamp_range(range);
        }
        ui.add(drag_value);
    }

    pub fn unique_windows(
        &mut self,
        ctx: &egui::Context,
        world_data: &mut WorldData,
        texts: &Texts,
    ) {
        egui::Window::new(texts.get("Uniques")).show(&ctx, |ui| {
            self.uniques_view(ui, world_data);
        });

        if let Some(unique_data) = world_data.uniques.get_mut(&self.selected_unique) {
            egui::Window::new(&self.selected_unique).show(&ctx, |ui| {
                Self::data_view(ui, &self.selected_unique, unique_data);
            });
        }
    }

    pub fn uniques_view(&mut self, ui: &mut egui::Ui, world_data: &WorldData) {
        egui::Grid::new("Uniques").show(ui, |ui| {
            for unique_name in world_data.uniques.keys() {
                if ui
                    .selectable_label(self.selected_unique == *unique_name, unique_name)
                    .clicked()
                {
                    self.selected_unique = unique_name.clone();
                }
                ui.end_row();
            }
        });
    }

    pub fn selected_entity(&self) -> EntityId {
        self.selected_entity
    }

    pub fn set_selected_entity(&mut self, selected_entity: EntityId) {
        self.selected_entity = selected_entity;
    }

    pub fn selected_unique(&self) -> &String {
        &self.selected_unique
    }
}

enum DropResult {
    Before,
    Into,
    After,
}
