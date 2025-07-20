use crate::{locale::Texts, project::Project};
use glam::{Vec3, Vec4};
use regex::Regex;
use shipyard::EntityId;
use std::{
    collections::HashMap,
    error::Error,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    sync::Arc,
};
use steel_common::{
    app::{App, Command},
    asset::{AssetId, AssetInfo},
    data::{Data, EntitiesData, EntityData, Limit, Value, WorldData},
    prefab::PrefabData,
};

pub struct DataWindow {
    selected_entity: EntityId,
    selected_unique: String,
    unnamed_regex: Regex,
}

impl DataWindow {
    pub fn new() -> Self {
        DataWindow {
            selected_entity: EntityId::dead(),
            selected_unique: String::new(),
            unnamed_regex: Regex::new(r"^unnamed-(\d+)$").unwrap(),
        }
    }

    pub fn entities_view(
        &mut self,
        ui: &mut egui::Ui,
        world_data: &WorldData,
        project: &mut Project,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
        load_world_data_this_frame: &mut bool,
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
            self.entity_level(
                root_entities,
                EntityId::dead(),
                ui,
                &world_data.entities,
                project,
                &asset_dir,
                texts,
                load_world_data_this_frame,
            );
        }

        ui.menu_button("+", |ui| {
            if ui.button(texts.get("New Entity")).clicked() {
                log::info!("entities_view_create_menu->New Entity");
                Self::create_new_entity(project.app().unwrap());
                ui.close_menu();
            }
            if ui.button(texts.get("From Prefab")).clicked() {
                log::info!("entities_view_create_menu->From Prefab");
                Self::create_entities_from_prefab(project.app().unwrap(), asset_dir);
                ui.close_menu();
            }
        });
    }

    fn entity_level(
        &mut self,
        es: &Vec<EntityId>,
        parent: EntityId,
        ui: &mut egui::Ui,
        entities: &EntitiesData,
        project: &mut Project,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
        load_world_data_this_frame: &mut bool,
    ) {
        for (i, &entity) in es.iter().enumerate() {
            let entity_data = if let Some(entity_data) = entities.get(&entity) {
                entity_data
            } else {
                log::warn!("entity_level: non-existent entity: {entity:?}");
                continue;
            };

            let mut entity_item = |ui: &mut egui::Ui| {
                let response = Self::drag_entity_hierarchy(ui, entity, |ui| {
                    self.entity_item(
                        ui,
                        entity,
                        entities,
                        entity_data,
                        project,
                        &asset_dir,
                        texts,
                        load_world_data_this_frame,
                    )
                })
                .response;

                Self::drop_entity_hierarchy(ui, response, entity, i, es, parent, project);
            };

            if let Some(children) = entity_data.children() {
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    egui::Id::new(entity),
                    false,
                )
                .show_header(ui, |ui| entity_item(ui))
                .body(|ui| {
                    self.entity_level(
                        children,
                        entity,
                        ui,
                        entities,
                        project,
                        asset_dir.as_ref(),
                        texts,
                        load_world_data_this_frame,
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

    fn entity_item(
        &mut self,
        ui: &mut egui::Ui,
        entity: EntityId,
        entities: &EntitiesData,
        entity_data: &EntityData,
        project: &mut Project,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
        load_world_data_this_frame: &mut bool,
    ) {
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
                Self::duplicate_entity(entity, entities, project.app().unwrap());
                ui.close_menu();
            }
            if ui.button(texts.get("Delete")).clicked() {
                log::info!("entity_context_menu->Delete");
                self.delete_entity(entity, project.app().unwrap());
                ui.close_menu();
            }
            if !project.is_running() {
                if entities
                    .get(&entity)
                    .and_then(|entity_data| entity_data.prefab_asset())
                    .is_some()
                {
                    if ui.button(texts.get("Save Prefab")).clicked() {
                        log::info!("entity_context_menu->Save Prefab");
                        Self::save_prefab(
                            entity,
                            entities,
                            project,
                            &asset_dir,
                            load_world_data_this_frame,
                        );
                        ui.close_menu();
                    }
                }
                if ui.button(texts.get("Save As Prefab")).clicked() {
                    log::info!("entity_context_menu->Save As Prefab");
                    Self::save_as_prefab(entity, entities, project.app().unwrap(), &asset_dir);
                    ui.close_menu();
                }
            }
        });
    }

    fn drag_entity_hierarchy<R>(
        ui: &mut egui::Ui,
        entity: EntityId,
        body: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let id = egui::Id::new(entity);
        let is_being_dragged = ui.ctx().is_being_dragged(id);

        if is_being_dragged {
            // paint the body to a new layer
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
            let egui::InnerResponse { inner, response } =
                ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), body);

            // now we move the visuals of the body to where the mouse is
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().transform_layer_shapes(
                    layer_id,
                    egui::emath::TSTransform::from_translation(delta),
                );
            }

            egui::InnerResponse::new(inner, response)
        } else {
            let egui::InnerResponse { inner, response } = ui.scope(body);

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
                ui.ctx().set_dragged_id(id);
                egui::DragAndDrop::set_payload(ui.ctx(), entity);
            }

            egui::InnerResponse::new(inner, response)
        }
    }

    fn drop_entity_hierarchy(
        ui: &mut egui::Ui,
        response: egui::Response,
        entity: EntityId,
        i: usize,
        es: &Vec<EntityId>,
        parent: EntityId,
        project: &mut Project,
    ) {
        if let (Some(pointer_pos), Some(drag_entity)) = (
            ui.input(|i| i.pointer.interact_pos()),
            response.dnd_hover_payload::<EntityId>(),
        ) {
            if entity != *drag_entity {
                let rect = response.rect;
                let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

                let can_insert_before = true;
                let can_insert_after = i == es.len() - 1;

                let (drop_parent, drop_before) = if can_insert_before
                    && pointer_pos.y - rect.top() < rect.height() / 4.0
                {
                    ui.painter().hline(rect.x_range(), rect.top(), stroke);
                    (parent, entity)
                } else if can_insert_after && pointer_pos.y - rect.top() > rect.height() * 3.0 / 4.0
                {
                    ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                    (
                        parent,
                        if i + 1 < es.len() {
                            es[i + 1]
                        } else {
                            EntityId::dead()
                        },
                    )
                } else {
                    ui.painter().rect_stroke(
                        rect,
                        ui.visuals().widgets.active.corner_radius,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                    (entity, EntityId::dead())
                };

                if let Some(drag_entity) = response.dnd_release_payload() {
                    project.app().unwrap().command(Command::AttachBefore(
                        *drag_entity,
                        drop_parent,
                        drop_before,
                    ));
                }
            }
        }
    }

    pub fn duplicate_entity(entity: EntityId, entities: &EntitiesData, app: &Box<dyn App>) {
        let entities_data = Self::get_entities_data_of_entity(entity, entities);
        let mut old_id_to_new_id = HashMap::new();
        app.command(Command::AddEntities(&entities_data, &mut old_id_to_new_id));
        let new_id = *old_id_to_new_id.get(&entity).unwrap();

        // attach duplicated entity next to the original entity
        let entity_data = entities
            .get(&entity)
            .expect(format!("duplicate_entity: non-existent entity: {entity:?}").as_str());
        app.command(Command::AttachAfter(new_id, entity_data.parent(), entity));
    }

    /// Get EntitiesData of an entity with its ancestors. This function will keep input entity as the first
    /// entity in returned EntitiesData so that it will be the first entity in the PrefabData created later.
    fn get_entities_data_of_entity(entity: EntityId, entities: &EntitiesData) -> EntitiesData {
        let mut entities_data = EntitiesData::default();
        let mut entities_to_add = vec![entity];
        while !entities_to_add.is_empty() {
            let mut new_entities_to_add = Vec::new();
            for entity in &entities_to_add {
                let entity_data = entities.get(entity).expect(
                    format!("get_entities_data_of_entity: non-existent entity: {entity:?}")
                        .as_str(),
                );
                entities_data.insert(*entity, entity_data.clone()); // TODO: avoid clone here
                for e in entity_data.children().into_iter().flatten() {
                    new_entities_to_add.push(*e);
                }
            }
            entities_to_add = new_entities_to_add;
        }
        entities_data
    }

    pub fn delete_entity(&mut self, entity: EntityId, app: &Box<dyn App>) {
        app.command(Command::DestroyEntity(entity));
        self.selected_entity = EntityId::dead();
    }

    pub fn save_as_prefab(
        entity: EntityId,
        entities: &EntitiesData,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
    ) {
        if let Err(e) = Self::save_as_prefab_inner(entity, entities, app, asset_dir) {
            log::error!("DataWindow::save_as_prefab error: {e:?}");
        }
    }

    fn save_as_prefab_inner(
        entity: EntityId,
        entities: &EntitiesData,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error>> {
        // get all entities that we will save as prefab
        let entities = Self::get_entities_data_of_entity(entity, entities);

        // convert entities to prefab data
        let get_prefab_data_fn = |prefab_asset: AssetId| {
            let mut prefab_data = None;
            app.command(Command::GetPrefabData(prefab_asset, &mut prefab_data));
            prefab_data
        };
        let (mut prefab_data, prefab_root_entity_to_nested_prefabs_index) =
            PrefabData::new(entities, get_prefab_data_fn);
        prefab_data.cut();

        // open file dialog to select a path to save prefab data
        let file = rfd::FileDialog::new().set_directory(&asset_dir).save_file();
        if let Some(mut file) = file {
            if !file.starts_with(&asset_dir) {
                return Err("you must save in asset directory!".into());
            }
            file.set_extension("prefab");
            if file.exists() {
                return Err("can not override existing file, you can use 'save prefab' to update existing prefab.".into());
            }
            crate::utils::save_to_file(&prefab_data, &file)?;

            // prefab asset is successfully saved, we must update Prefab components
            let asset_path = file
                .strip_prefix(&asset_dir)
                .expect("Already checked file.starts_with(&asset_dir) is true!");
            let asset_info =
                Project::get_asset_info_and_insert(&asset_dir, asset_path, app, false)?;
            app.command(Command::CreatePrefab(
                entity,
                asset_info.id,
                prefab_root_entity_to_nested_prefabs_index,
            ));
        }
        Ok(())
    }

    pub fn save_prefab(
        entity: EntityId,
        entities: &EntitiesData,
        project: &mut Project,
        asset_dir: impl AsRef<Path>,
        load_world_data_this_frame: &mut bool,
    ) {
        if let Err(e) = Self::save_prefab_inner(
            entity,
            entities,
            project,
            asset_dir,
            load_world_data_this_frame,
        ) {
            log::error!("DataWindow::save_prefab error: {e:?}");
        }
    }

    fn save_prefab_inner(
        entity: EntityId,
        entities: &EntitiesData,
        project: &mut Project,
        asset_dir: impl AsRef<Path>,
        load_world_data_this_frame: &mut bool,
    ) -> Result<(), Box<dyn Error>> {
        // find prefab root entity
        let entity_data = entities
            .get(&entity)
            .ok_or("DataWindow::save_prefab_inner: entity not found")?;
        let (prefab_asset, _, _, prefab_root_entity) = entity_data
            .prefab_info()
            .ok_or("DataWindow::save_prefab_inner: entity is not in prefab")?;
        let entity = prefab_root_entity;

        // get all entities in the prefab that we will save
        let entities = Self::get_entities_data_of_entity(entity, entities);

        // create updated prefab data
        let app = project.app().unwrap();
        let get_prefab_data_fn = |prefab_asset: AssetId| {
            let mut prefab_data = None;
            app.command(Command::GetPrefabData(prefab_asset, &mut prefab_data));
            prefab_data
        };
        let (prefab_data, entity_id_to_prefab_entity_id_with_path) =
            PrefabData::update(entities, get_prefab_data_fn)?;

        // update Prefab components
        app.command(Command::LoadPrefab(
            entity,
            prefab_asset,
            entity_id_to_prefab_entity_id_with_path,
        ));

        // save scene data before saving prefab data
        let prefab_data = Arc::new(prefab_data);
        project.save_to_memory(Some((entity, prefab_data.clone())));

        // save prefab data to file.
        let app = project.app().unwrap();
        let mut prefab_asset_path = None;
        app.command(Command::GetAssetPath(prefab_asset, &mut prefab_asset_path));
        let prefab_asset_path =
            prefab_asset_path.ok_or("DataWindow::save_prefab: prefab not found")?;
        crate::utils::save_to_file(
            prefab_data.as_ref(),
            asset_dir.as_ref().join(&prefab_asset_path),
        )?;

        // invalid asset cache now because we need reload scene right now, so we can't wait until it being invalidated next frame
        app.command(Command::InsertAsset(prefab_asset, prefab_asset_path));

        // reload scene for other prefab instances to update
        project.load_from_memory();
        *load_world_data_this_frame = false;

        Ok(())
    }

    pub fn create_new_entity(app: &Box<dyn App>) {
        app.command(Command::CreateEntity);
    }

    pub fn create_entities_from_prefab(app: &Box<dyn App>, asset_dir: impl AsRef<Path>) {
        if let Err(e) = Self::create_entities_from_prefab_inner(app, asset_dir) {
            log::error!("DataWindow::create_entities_from_prefab error: {e:?}");
        }
    }

    fn create_entities_from_prefab_inner(
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error>> {
        let file = rfd::FileDialog::new().set_directory(&asset_dir).pick_file();
        if let Some(file) = file {
            if !file.starts_with(&asset_dir) {
                return Err("you must load from file in asset directory!".into());
            }
            let asset_path = file
                .strip_prefix(&asset_dir)
                .expect("Already checked file.starts_with(&asset_dir) is true!");
            let asset_info =
                Project::get_asset_info_and_insert(&asset_dir, asset_path, app, false)?;

            let mut prefab_root_entity = Err("".into());
            app.command(Command::AddEntitiesFromPrefab(
                asset_info.id,
                &mut prefab_root_entity,
            ));
            prefab_root_entity?;
        }
        Ok(())
    }

    fn entity_label(id: &EntityId, entity_data: &EntityData) -> impl Into<egui::WidgetText> {
        if let Some(name) = entity_data.name() {
            if !name.is_empty() {
                return format!("{name}");
            }
        }
        format!("{:?}", id)
    }

    pub fn entity_view(
        &mut self,
        ui: &mut egui::Ui,
        entity_data: &mut EntityData,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
    ) {
        Self::color_label(
            ui,
            egui::Color32::BLACK,
            format!("{:?}", self.selected_entity),
        );
        ui.separator();
        for (component_name, component_data) in &mut entity_data.components {
            ui.horizontal(|ui| {
                ui.label(component_name);
                if component_name != "Children"
                    && component_name != "Parent"
                    && component_name != "Prefab"
                {
                    // TODO: use a more generic way to prevent some components from being destroyed by user
                    if ui.button("-").clicked() {
                        app.command(Command::DestroyComponent(
                            self.selected_entity,
                            component_name,
                        ));
                    }
                }
            });
            self.data_view(
                ui,
                component_name,
                component_data,
                app,
                &asset_dir,
                texts,
                false,
            );
            ui.separator();
        }

        let mut components = Vec::new();
        app.command(Command::GetComponents(&mut components));
        ui.menu_button("+", |ui| {
            for component in components
                .into_iter()
                .filter(|c| *c != "Children" && *c != "Parent" && *c != "Prefab")
            {
                // TODO: use a more generic way to prevent some components from being created by user
                if ui.button(component).clicked() {
                    app.command(Command::CreateComponent(self.selected_entity, component));
                    ui.close_menu();
                }
            }
        });
    }

    pub fn data_view(
        &mut self,
        ui: &mut egui::Ui,
        data_name: &str,
        data: &mut Data,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
        read_only: bool,
    ) {
        let color = egui::Color32::BLACK;
        for (name, value) in &mut data.values {
            ui.horizontal(|ui| {
                if !self.unnamed_regex.is_match(&name) {
                    ui.label(name);
                }
                let limit = data.limits.get(name);
                self.mutable_value_view(
                    ui,
                    value,
                    limit,
                    name,
                    data_name,
                    color,
                    app,
                    asset_dir.as_ref(),
                    texts,
                    read_only || Some(&Limit::ReadOnly) == limit,
                );
            });
        }
    }

    fn mutable_value_view(
        &mut self,
        ui: &mut egui::Ui,
        value: &mut Value,
        limit: Option<&Limit>,
        name: &String,
        data_name: &str,
        color: egui::Color32,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path> + Copy,
        texts: &Texts,
        read_only: bool,
    ) {
        if read_only {
            match value {
                Value::Bool(v) => Self::color_label(ui, color, if *v { "☑" } else { "☐" }),
                Value::Int32(v) => Self::color_label(ui, color, format!("{v}")),
                Value::Int64(v) => Self::color_label(ui, color, format!("{v}")),
                Value::UInt32(v) => Self::color_label(ui, color, format!("{v}")),
                Value::UInt64(v) => Self::color_label(ui, color, format!("{v}")),
                Value::Float32(v) => Self::color_label(ui, color, format!("{v}")),
                Value::Float64(v) => Self::color_label(ui, color, format!("{v}")),
                Value::String(v) => Self::color_label(ui, color, format!("{v}")),
                Value::Vec2(v) => Self::color_label(ui, color, format!("{v}")),
                Value::Vec3(v) => Self::color_label(ui, color, format!("{v}")),
                Value::Vec4(v) => Self::color_label(ui, color, format!("{v}")),
                Value::IVec2(v) => Self::color_label(ui, color, format!("{v}")),
                Value::IVec3(v) => Self::color_label(ui, color, format!("{v}")),
                Value::IVec4(v) => Self::color_label(ui, color, format!("{v}")),
                Value::UVec2(v) => Self::color_label(ui, color, format!("{v}")),
                Value::UVec3(v) => Self::color_label(ui, color, format!("{v}")),
                Value::UVec4(v) => Self::color_label(ui, color, format!("{v}")),
                Value::Entity(v) => self.show_entity(ui, v, app),
                Value::Asset(v) => {
                    Self::show_asset(ui, color, *v, app);
                }
                Value::VecBool(v) => Self::vec_value_view(ui, v, |ui, _, e| {
                    Self::color_label(ui, color, if *e { "☑" } else { "☐" })
                }),
                Value::VecInt32(v) => Self::vec_value_default_view(ui, v, color),
                Value::VecInt64(v) => Self::vec_value_default_view(ui, v, color),
                Value::VecUInt32(v) => Self::vec_value_default_view(ui, v, color),
                Value::VecUInt64(v) => Self::vec_value_default_view(ui, v, color),
                Value::VecFloat32(v) => Self::vec_value_default_view(ui, v, color),
                Value::VecFloat64(v) => Self::vec_value_default_view(ui, v, color),
                Value::VecString(v) => Self::vec_value_default_view(ui, v, color),
                Value::VecEntity(v) => Self::vec_value_view(ui, v, |ui, _, e| {
                    self.show_entity(ui, e, app);
                }),
                Value::VecAsset(v) => Self::vec_value_default_view(ui, v, color),
                Value::Data(data) => {
                    ui.vertical(|ui| {
                        self.data_view(
                            ui,
                            &format!("{data_name} {name}"),
                            data,
                            app,
                            asset_dir,
                            texts,
                            read_only,
                        )
                    });
                }
                Value::VecData(v) => Self::vec_value_view(ui, v, |ui, i, e| {
                    ui.vertical(|ui| {
                        self.data_view(
                            ui,
                            &format!("{data_name} {name} {i}"),
                            e,
                            app,
                            asset_dir,
                            texts,
                            read_only,
                        )
                    });
                }),
            }
            return;
        }
        match value {
            Value::Bool(v) => {
                Self::mutable_bool_view(ui, v);
            }
            Value::Int32(v) => {
                Self::mutable_i32_view(ui, v, limit, name, data_name);
            }
            Value::Int64(v) => {
                Self::mutable_i64_view(ui, v, limit);
            }
            Value::UInt32(v) => {
                Self::mutable_u32_view(ui, v, limit);
            }
            Value::UInt64(v) => {
                Self::mutable_u64_view(ui, v, limit);
            }
            Value::Float32(v) => {
                Self::mutable_f32_view(ui, v, limit);
            }
            Value::Float64(v) => {
                Self::mutable_f64_view(ui, v, limit);
            }
            Value::String(v) => {
                Self::mutable_string_view(ui, v, limit);
            }
            Value::Vec2(v) => {
                ui.horizontal(|ui| {
                    if let Some(Limit::Float32Rotation) = limit {
                        ui.drag_angle(&mut v.x);
                        ui.drag_angle(&mut v.y);
                    } else {
                        let range = match limit {
                            Some(Limit::Float32Range(range)) => Some(range),
                            _ => None,
                        };
                        Self::drag_float(ui, &mut v.x, range);
                        Self::drag_float(ui, &mut v.y, range);
                    }
                });
            }
            Value::Vec3(v) => {
                ui.horizontal(|ui| {
                    if let Some(Limit::Vec3Color) = limit {
                        Self::drag_float(ui, &mut v.x, None);
                        Self::drag_float(ui, &mut v.y, None);
                        Self::drag_float(ui, &mut v.z, None);
                        let mut color = v.to_array();
                        ui.color_edit_button_rgb(&mut color);
                        *v = Vec3::from_array(color);
                    } else if let Some(Limit::Float32Rotation) = limit {
                        ui.drag_angle(&mut v.x);
                        ui.drag_angle(&mut v.y);
                        ui.drag_angle(&mut v.z);
                    } else {
                        let range = match limit {
                            Some(Limit::Float32Range(range)) => Some(range),
                            _ => None,
                        };
                        Self::drag_float(ui, &mut v.x, range);
                        Self::drag_float(ui, &mut v.y, range);
                        Self::drag_float(ui, &mut v.z, range);
                    }
                });
            }
            Value::Vec4(v) => {
                ui.horizontal(|ui| {
                    if let Some(Limit::Vec4Color) = limit {
                        Self::drag_float(ui, &mut v.x, None);
                        Self::drag_float(ui, &mut v.y, None);
                        Self::drag_float(ui, &mut v.z, None);
                        Self::drag_float(ui, &mut v.w, None);
                        let mut color = v.to_array();
                        ui.color_edit_button_rgba_unmultiplied(&mut color);
                        *v = Vec4::from_array(color);
                    } else if let Some(Limit::Float32Rotation) = limit {
                        ui.drag_angle(&mut v.x);
                        ui.drag_angle(&mut v.y);
                        ui.drag_angle(&mut v.z);
                        ui.drag_angle(&mut v.w);
                    } else {
                        let range = match limit {
                            Some(Limit::Float32Range(range)) => Some(range),
                            _ => None,
                        };
                        Self::drag_float(ui, &mut v.x, range);
                        Self::drag_float(ui, &mut v.y, range);
                        Self::drag_float(ui, &mut v.z, range);
                        Self::drag_float(ui, &mut v.w, range);
                    }
                });
            }
            Value::IVec2(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::Int32Range(range)) => Some(range),
                        _ => None,
                    };
                    Self::drag_value(ui, &mut v.x, range);
                    Self::drag_value(ui, &mut v.y, range);
                });
            }
            Value::IVec3(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::Int32Range(range)) => Some(range),
                        _ => None,
                    };
                    Self::drag_value(ui, &mut v.x, range);
                    Self::drag_value(ui, &mut v.y, range);
                    Self::drag_value(ui, &mut v.z, range);
                });
            }
            Value::IVec4(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::Int32Range(range)) => Some(range),
                        _ => None,
                    };
                    Self::drag_value(ui, &mut v.x, range);
                    Self::drag_value(ui, &mut v.y, range);
                    Self::drag_value(ui, &mut v.z, range);
                    Self::drag_value(ui, &mut v.w, range);
                });
            }
            Value::UVec2(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::UInt32Range(range)) => Some(range),
                        _ => None,
                    };
                    Self::drag_value(ui, &mut v.x, range);
                    Self::drag_value(ui, &mut v.y, range);
                });
            }
            Value::UVec3(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::UInt32Range(range)) => Some(range),
                        _ => None,
                    };
                    Self::drag_value(ui, &mut v.x, range);
                    Self::drag_value(ui, &mut v.y, range);
                    Self::drag_value(ui, &mut v.z, range);
                });
            }
            Value::UVec4(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::UInt32Range(range)) => Some(range),
                        _ => None,
                    };
                    Self::drag_value(ui, &mut v.x, range);
                    Self::drag_value(ui, &mut v.y, range);
                    Self::drag_value(ui, &mut v.z, range);
                    Self::drag_value(ui, &mut v.w, range);
                });
            }
            Value::Entity(v) => {
                self.mutable_entity_view(ui, v, app);
            }
            Value::Asset(v) => {
                Self::mutable_asset_view(ui, v, color, app, asset_dir, texts);
            }
            Value::VecBool(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_bool_view(ui, e);
            }),
            Value::VecInt32(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_i32_view(ui, e, limit, name, data_name);
            }),
            Value::VecInt64(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_i64_view(ui, e, limit);
            }),
            Value::VecUInt32(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_u32_view(ui, e, limit);
            }),
            Value::VecUInt64(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_u64_view(ui, e, limit);
            }),
            Value::VecFloat32(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_f32_view(ui, e, limit);
            }),
            Value::VecFloat64(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_f64_view(ui, e, limit);
            }),
            Value::VecString(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_string_view(ui, e, limit);
            }),
            Value::VecEntity(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                self.mutable_entity_view(ui, e, app);
            }),
            Value::VecAsset(v) => Self::mutable_vec_value_view(ui, v, |ui, _, e| {
                Self::mutable_asset_view(ui, e, color, app, asset_dir, texts);
            }),
            Value::Data(data) => {
                ui.vertical(|ui| {
                    self.data_view(
                        ui,
                        &format!("{data_name} {name}"),
                        data,
                        app,
                        asset_dir,
                        texts,
                        read_only,
                    )
                });
            }
            Value::VecData(v) => Self::mutable_vec_value_view(ui, v, |ui, i, e| {
                ui.vertical(|ui| {
                    self.data_view(
                        ui,
                        &format!("{data_name} {name} {i}"),
                        e,
                        app,
                        asset_dir,
                        texts,
                        read_only,
                    )
                });
            }),
        }
    }

    fn color_label(ui: &mut egui::Ui, color: egui::Color32, text: impl Into<egui::WidgetText>) {
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(3, 1))
            .corner_radius(egui::CornerRadius::same(3))
            .fill(color)
            .show(ui, |ui| ui.label(text));
    }

    fn vec_value_default_view<T: std::fmt::Debug>(
        ui: &mut egui::Ui,
        v: &Vec<T>,
        color: egui::Color32,
    ) {
        ui.vertical(|ui| {
            for e in v {
                Self::color_label(ui, color, format!("{e:?}"));
            }
        });
    }

    fn vec_value_view<T>(
        ui: &mut egui::Ui,
        v: &mut Vec<T>,
        mut value_view: impl FnMut(&mut egui::Ui, usize, &mut T),
    ) {
        ui.vertical(|ui| {
            for (i, e) in v.iter_mut().enumerate() {
                value_view(ui, i, e);
            }
        });
    }

    fn mutable_vec_value_view<T: Default>(
        ui: &mut egui::Ui,
        v: &mut Vec<T>,
        mut mutable_value_view: impl FnMut(&mut egui::Ui, usize, &mut T),
    ) {
        ui.vertical(|ui| {
            let mut remove_index = None;
            for (i, e) in v.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    mutable_value_view(ui, i, e);
                    if ui.button("-").clicked() {
                        remove_index = Some(i);
                    }
                });
            }
            if let Some(remove_index) = remove_index {
                v.remove(remove_index);
            }
            if ui.button("+").clicked() {
                v.push(T::default());
            }
        });
    }

    fn mutable_bool_view(ui: &mut egui::Ui, v: &mut bool) {
        ui.checkbox(v, "");
    }

    fn mutable_i32_view(
        ui: &mut egui::Ui,
        v: &mut i32,
        limit: Option<&Limit>,
        name: &String,
        data_name: &str,
    ) {
        if let Some(Limit::Int32Enum(int_enum)) = limit {
            if int_enum.len() > 0 {
                let mut i = int_enum
                    .iter()
                    .enumerate()
                    .find_map(|(i, (int, _))| if v == int { Some(i) } else { None })
                    .unwrap_or(0);
                // Use component_name/unique_name + value_name as id to make sure that every id is unique
                egui::ComboBox::from_id_salt(format!("{} {}", data_name, name)).show_index(
                    ui,
                    &mut i,
                    int_enum.len(),
                    |i| &int_enum[i].1,
                );
                *v = int_enum[i].0;
            } else {
                Self::color_label(ui, egui::Color32::RED, "zero length int_enum!");
            }
        } else {
            let range = match limit {
                Some(Limit::Int32Range(range)) => Some(range),
                _ => None,
            };
            Self::drag_value(ui, v, range);
        }
    }

    fn mutable_i64_view(ui: &mut egui::Ui, v: &mut i64, limit: Option<&Limit>) {
        let range = match limit {
            Some(Limit::Int64Range(range)) => Some(range),
            _ => None,
        };
        Self::drag_value(ui, v, range);
    }

    fn mutable_u32_view(ui: &mut egui::Ui, v: &mut u32, limit: Option<&Limit>) {
        let range = match limit {
            Some(Limit::UInt32Range(range)) => Some(range),
            _ => None,
        };
        Self::drag_value(ui, v, range);
    }

    fn mutable_u64_view(ui: &mut egui::Ui, v: &mut u64, limit: Option<&Limit>) {
        let range = match limit {
            Some(Limit::UInt64Range(range)) => Some(range),
            _ => None,
        };
        Self::drag_value(ui, v, range);
    }

    fn mutable_f32_view(ui: &mut egui::Ui, v: &mut f32, limit: Option<&Limit>) {
        if let Some(Limit::Float32Rotation) = limit {
            ui.drag_angle(v);
        } else {
            Self::drag_float(
                ui,
                v,
                match limit {
                    Some(Limit::Float32Range(range)) => Some(range),
                    _ => None,
                },
            );
        }
    }

    fn mutable_f64_view(ui: &mut egui::Ui, v: &mut f64, limit: Option<&Limit>) {
        Self::drag_float(
            ui,
            v,
            match limit {
                Some(Limit::Float64Range(range)) => Some(range),
                _ => None,
            },
        );
    }

    fn mutable_string_view(ui: &mut egui::Ui, v: &mut String, limit: Option<&Limit>) {
        if let Some(Limit::StringMultiline) = limit {
            ui.text_edit_multiline(v);
        } else {
            ui.text_edit_singleline(v);
        }
    }

    /// Displays a DragValue for floats.
    fn drag_float<F: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        v: &mut F,
        range: Option<&RangeInclusive<F>>,
    ) {
        let mut drag_value = egui::DragValue::new(v).speed(0.01);
        if let Some(range) = range {
            drag_value = drag_value.range(range.clone());
        }
        ui.add(drag_value);
    }

    fn drag_value<V: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        v: &mut V,
        range: Option<&RangeInclusive<V>>,
    ) {
        let mut drag_value = egui::DragValue::new(v);
        if let Some(range) = range {
            drag_value = drag_value.range(range.clone());
        }
        ui.add(drag_value);
    }

    fn mutable_entity_view(&mut self, ui: &mut egui::Ui, v: &mut EntityId, app: &Box<dyn App>) {
        let response = ui
            .scope(|ui| {
                self.show_entity(ui, v, app);
            })
            .response;

        if response.dnd_hover_payload::<EntityId>().is_some() {
            ui.painter().rect_stroke(
                response.rect,
                ui.visuals().widgets.active.corner_radius,
                egui::Stroke::new(1.0, egui::Color32::WHITE),
                egui::StrokeKind::Middle,
            );

            if let Some(drag_entity) = response.dnd_release_payload() {
                *v = *drag_entity;
            }
        }
    }

    fn show_entity(&mut self, ui: &mut egui::Ui, v: &EntityId, app: &Box<dyn App>) {
        let mut name = None;
        app.command(Command::GetEntityName(*v, &mut name));
        let name = name.map(|name| format!("{name} - ")).unwrap_or_default();
        if ui.button(format!("{name}{v:?}")).clicked() {
            if *v != EntityId::dead() {
                self.selected_entity = *v;
                // TODO: center the selected entity in entities tab
            }
        }
    }

    fn mutable_asset_view(
        ui: &mut egui::Ui,
        v: &mut AssetId,
        color: egui::Color32,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
    ) {
        ui.horizontal(|ui| {
            let asest_path = Self::show_asset(ui, color, *v, app);
            if ui.button(texts.get("Select")).clicked() {
                let starting_dir = if let Some(asset_path) = &asest_path {
                    asset_dir
                        .as_ref()
                        .join(asset_path)
                        .parent()
                        .unwrap()
                        .to_path_buf()
                } else {
                    asset_dir.as_ref().to_path_buf()
                };
                let file = rfd::FileDialog::new()
                    .set_directory(starting_dir)
                    .pick_file();
                if let Some(mut file) = file {
                    if file.starts_with(&asset_dir) {
                        if file
                            .extension()
                            .is_some_and(|extension| extension == "asset")
                        {
                            file = AssetInfo::asset_info_path_to_asset_path(file);
                        }
                        let asset_file = file.strip_prefix(&asset_dir).unwrap();
                        match Project::get_asset_info_and_insert(&asset_dir, asset_file, app, false)
                        {
                            Ok(asset_info) => *v = asset_info.id,
                            Err(e) => log::error!("Failed to get asset info, error: {e}"),
                        }
                    } else {
                        log::error!(
                            "You must select a file in asset directory: {}",
                            asset_dir.as_ref().display()
                        );
                    }
                }
            }
            if *v != AssetId::INVALID && ui.button(texts.get("Reset")).clicked() {
                *v = AssetId::INVALID;
            }
        });
    }

    fn show_asset(
        ui: &mut egui::Ui,
        color: egui::Color32,
        asset_id: AssetId,
        app: &Box<dyn App>,
    ) -> Option<PathBuf> {
        let mut asset_path = None;
        app.command(Command::GetAssetPath(asset_id, &mut asset_path));
        if let Some(asset_path) = &asset_path {
            Self::color_label(ui, color, format!("Asset({})", asset_path.display()));
        } else {
            Self::color_label(ui, color, format!("Invalid{asset_id:?}"));
        }
        asset_path
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
