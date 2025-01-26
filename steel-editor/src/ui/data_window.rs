use crate::{
    locale::Texts,
    project::Project,
    utils::{err, EditorError},
};
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
                &world_data.entities,
                project,
                &asset_dir,
                &mut drag_entity,
                &mut drop_parent,
                &mut drop_before,
                texts,
            );
            if let Some(drop_parent) = drop_parent {
                if drag_entity != EntityId::dead() && ui.input(|input| input.pointer.any_released())
                {
                    project.app().unwrap().command(Command::AttachBefore(
                        drag_entity,
                        drop_parent,
                        drop_before,
                    ));
                }
            }
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
        drag_entity: &mut EntityId,
        drop_parent: &mut Option<EntityId>,
        drop_before: &mut EntityId,
        texts: &Texts,
    ) {
        for (i, &entity) in es.iter().enumerate() {
            let entity_data = if let Some(entity_data) = entities.get(&entity) {
                entity_data
            } else {
                log::warn!("entity_level: non-existent entity: {entity:?}");
                continue;
            };

            let mut entity_item = |ui: &mut egui::Ui| {
                let drag_id = egui::Id::new(entity);
                if ui.memory(|mem| mem.is_being_dragged(drag_id)) {
                    *drag_entity = entity;
                }

                let can_accept_what_is_being_dragged = entity != *drag_entity;
                let can_insert_before = true;
                let can_insert_after = i == es.len() - 1;

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
                                    Self::duplicate_entity(
                                        entity,
                                        entities,
                                        project.app().unwrap(),
                                    );
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
                                                entity, entities, project, &asset_dir,
                                            );
                                            ui.close_menu();
                                        }
                                    }
                                    if ui.button(texts.get("Save As Prefab")).clicked() {
                                        log::info!("entity_context_menu->Save As Prefab");
                                        Self::save_as_prefab(
                                            entity,
                                            entities,
                                            project.app().unwrap(),
                                            &asset_dir,
                                        );
                                        ui.close_menu();
                                    }
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
                            *drop_before = if i + 1 < es.len() {
                                es[i + 1]
                            } else {
                                EntityId::dead()
                            };
                        }
                    }
                }
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

    pub fn duplicate_entity(entity: EntityId, entities: &EntitiesData, app: &mut Box<dyn App>) {
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

    pub fn delete_entity(&mut self, entity: EntityId, app: &mut Box<dyn App>) {
        app.command(Command::DestroyEntity(entity));
        self.selected_entity = EntityId::dead();
    }

    pub fn save_as_prefab(
        entity: EntityId,
        entities: &EntitiesData,
        app: &mut Box<dyn App>,
        asset_dir: impl AsRef<Path>,
    ) {
        if let Err(e) = Self::save_as_prefab_inner(entity, entities, app, asset_dir) {
            log::error!("DataWindow::save_as_prefab error: {e:?}");
        }
    }

    fn save_as_prefab_inner(
        entity: EntityId,
        entities: &EntitiesData,
        app: &mut Box<dyn App>,
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
                return err("you must save in asset directory!");
            }
            file.set_extension("prefab");
            if file.exists() {
                return err("can not override existing file, you can use 'save prefab' to update existing prefab.");
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
    ) {
        if let Err(e) = Self::save_prefab_inner(entity, entities, project, asset_dir) {
            log::error!("DataWindow::save_prefab error: {e:?}");
        }
    }

    fn save_prefab_inner(
        entity: EntityId,
        entities: &EntitiesData,
        project: &mut Project,
        asset_dir: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error>> {
        // find prefab root entity
        let entity_data = entities.get(&entity).ok_or(EditorError::new(
            "DataWindow::save_prefab_inner: entity not found",
        ))?;
        let (prefab_asset, _, _, prefab_root_entity) = entity_data.prefab_info().ok_or(
            EditorError::new("DataWindow::save_prefab_inner: entity is not in prefab"),
        )?;
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
        let prefab_asset_path = prefab_asset_path.ok_or(EditorError::new(
            "DataWindow::save_prefab: prefab not found",
        ))?;
        crate::utils::save_to_file(
            prefab_data.as_ref(),
            asset_dir.as_ref().join(&prefab_asset_path),
        )?;

        // invalid asset cache now because we need reload scene right now, so we can't wait until it being invalidated next frame
        app.command(Command::InsertAsset(prefab_asset, prefab_asset_path));

        // reload scene for other prefab instances to update
        project.load_from_memory();

        Ok(())
    }

    pub fn create_new_entity(app: &mut Box<dyn App>) {
        app.command(Command::CreateEntity);
    }

    pub fn create_entities_from_prefab(app: &mut Box<dyn App>, asset_dir: impl AsRef<Path>) {
        if let Err(e) = Self::create_entities_from_prefab_inner(app, asset_dir) {
            log::error!("DataWindow::create_entities_from_prefab error: {e:?}");
        }
    }

    fn create_entities_from_prefab_inner(
        app: &mut Box<dyn App>,
        asset_dir: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error>> {
        let file = rfd::FileDialog::new().set_directory(&asset_dir).pick_file();
        if let Some(file) = file {
            if !file.starts_with(&asset_dir) {
                return err("you must load from file in asset directory!");
            }
            let asset_path = file
                .strip_prefix(&asset_dir)
                .expect("Already checked file.starts_with(&asset_dir) is true!");
            let asset_info =
                Project::get_asset_info_and_insert(&asset_dir, asset_path, app, false)?;
            let get_prefab_data_fn = |prefab_asset: AssetId| {
                let mut prefab_data = None;
                app.command(Command::GetPrefabData(prefab_asset, &mut prefab_data));
                prefab_data
            };
            let prefab_data = get_prefab_data_fn(asset_info.id)
                .ok_or(EditorError::new("failed to get prefab data!"))?;
            let (entities_data, entity_map) = prefab_data.to_entities_data(get_prefab_data_fn);
            let mut old_id_to_new_id = HashMap::new();
            app.command(Command::AddEntities(&entities_data, &mut old_id_to_new_id));

            // prefab asset is successfully loaded, we must update Prefab components
            let mut entity_id_to_prefab_entity_id_with_path = HashMap::new();
            for (entity_id_with_path, old_id) in entity_map {
                let new_id = old_id_to_new_id.get(&old_id).ok_or(EditorError::new(
                    "old_id_to_new_id should contain all EntityId!",
                ))?;
                entity_id_to_prefab_entity_id_with_path.insert(*new_id, entity_id_with_path);
            }
            let root_entity = *entities_data
                .root()
                .and_then(|e| old_id_to_new_id.get(&e))
                .ok_or(EditorError::new("there should be a root entity in prefab!"))?;
            app.command(Command::LoadPrefab(
                root_entity,
                asset_info.id,
                entity_id_to_prefab_entity_id_with_path,
            ));
        }
        Ok(())
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
        app: &mut Box<dyn App>,
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
            self.data_view(ui, component_name, component_data, app, &asset_dir, texts);
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
        &self,
        ui: &mut egui::Ui,
        data_name: &str,
        data: &mut Data,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
    ) {
        let color = egui::Color32::BLACK;
        for (name, value) in &mut data.values {
            ui.horizontal(|ui| {
                if !self.unnamed_regex.is_match(&name) {
                    ui.label(name);
                }
                let limit = data.limits.get(name);
                if let Some(Limit::ReadOnly) = limit {
                    Self::immutable_value_view(ui, value, color, app);
                } else {
                    Self::mutable_value_view(
                        ui,
                        value,
                        limit,
                        name,
                        data_name,
                        color,
                        app,
                        asset_dir.as_ref(),
                        texts,
                    );
                }
            });
        }
    }

    fn immutable_value_view(
        ui: &mut egui::Ui,
        value: &Value,
        color: egui::Color32,
        app: &Box<dyn App>,
    ) {
        match value {
            Value::Bool(b) => Self::color_label(ui, color, if *b { "☑" } else { "☐" }),
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
            Value::Entity(v) => Self::color_label(ui, color, format!("{v:?}")), // TODO: show entity name
            Value::Asset(v) => {
                Self::show_asset(ui, color, *v, app);
            }
            Value::VecBool(v) => Self::vec_value_view(ui, v, color),
            Value::VecInt32(v) => Self::vec_value_view(ui, v, color),
            Value::VecInt64(v) => Self::vec_value_view(ui, v, color),
            Value::VecUInt32(v) => Self::vec_value_view(ui, v, color),
            Value::VecUInt64(v) => Self::vec_value_view(ui, v, color),
            Value::VecFloat32(v) => Self::vec_value_view(ui, v, color),
            Value::VecFloat64(v) => Self::vec_value_view(ui, v, color),
            Value::VecString(v) => Self::vec_value_view(ui, v, color),
            Value::VecEntity(v) => Self::vec_value_view(ui, v, color),
            Value::VecAsset(v) => Self::vec_value_view(ui, v, color),
        }
    }

    fn vec_value_view<T: std::fmt::Debug>(ui: &mut egui::Ui, v: &Vec<T>, color: egui::Color32) {
        ui.vertical(|ui| {
            for e in v {
                Self::color_label(ui, color, format!("{e:?}"));
            }
        });
    }

    fn mutable_value_view(
        ui: &mut egui::Ui,
        value: &mut Value,
        limit: Option<&Limit>,
        name: &String,
        data_name: &str,
        color: egui::Color32,
        app: &Box<dyn App>,
        asset_dir: impl AsRef<Path>,
        texts: &Texts,
    ) {
        match value {
            Value::Bool(b) => {
                ui.checkbox(b, "");
            }
            Value::Int32(v) => {
                if let Some(Limit::Int32Enum(int_enum)) = limit {
                    if int_enum.len() > 0 {
                        let mut i = int_enum
                            .iter()
                            .enumerate()
                            .find_map(|(i, (int, _))| if v == int { Some(i) } else { None })
                            .unwrap_or(0);
                        // Use component_name/unique_name + value_name as id to make sure that every id is unique
                        egui::ComboBox::from_id_source(format!("{} {}", data_name, name))
                            .show_index(ui, &mut i, int_enum.len(), |i| &int_enum[i].1);
                        *v = int_enum[i].0;
                    } else {
                        Self::color_label(ui, egui::Color32::RED, "zero length int_enum!");
                    }
                } else {
                    let range = match limit {
                        Some(Limit::Int32Range(range)) => Some(range.clone()),
                        _ => None,
                    };
                    Self::drag_value(ui, v, range);
                }
            }
            Value::Int64(v) => {
                let range = match limit {
                    Some(Limit::Int64Range(range)) => Some(range.clone()),
                    _ => None,
                };
                Self::drag_value(ui, v, range);
            }
            Value::UInt32(v) => {
                let range = match limit {
                    Some(Limit::UInt32Range(range)) => Some(range.clone()),
                    _ => None,
                };
                Self::drag_value(ui, v, range);
            }
            Value::UInt64(v) => {
                let range = match limit {
                    Some(Limit::UInt64Range(range)) => Some(range.clone()),
                    _ => None,
                };
                Self::drag_value(ui, v, range);
            }
            Value::Float32(v) => {
                if let Some(Limit::Float32Rotation) = limit {
                    ui.drag_angle(v);
                } else {
                    Self::drag_float(
                        ui,
                        v,
                        match limit {
                            Some(Limit::Float32Range(range)) => Some(range.clone()),
                            _ => None,
                        },
                    );
                }
            }
            Value::Float64(v) => {
                Self::drag_float(
                    ui,
                    v,
                    match limit {
                        Some(Limit::Float64Range(range)) => Some(range.clone()),
                        _ => None,
                    },
                );
            }
            Value::String(v) => {
                if let Some(Limit::StringMultiline) = limit {
                    ui.text_edit_multiline(v);
                } else {
                    ui.text_edit_singleline(v);
                }
            }
            Value::Vec2(v) => {
                ui.horizontal(|ui| {
                    if let Some(Limit::Float32Rotation) = limit {
                        ui.drag_angle(&mut v.x);
                        ui.drag_angle(&mut v.y);
                    } else {
                        let range = match limit {
                            Some(Limit::Float32Range(range)) => {
                                vec![Some(range.clone()); 2]
                            }
                            Some(Limit::VecRange(range)) => range.clone(),
                            _ => Vec::new(),
                        };
                        Self::drag_float(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                        Self::drag_float(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
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
                            Some(Limit::Float32Range(range)) => {
                                vec![Some(range.clone()); 3]
                            }
                            Some(Limit::VecRange(range)) => range.clone(),
                            _ => Vec::new(),
                        };
                        Self::drag_float(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                        Self::drag_float(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                        Self::drag_float(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
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
                            Some(Limit::Float32Range(range)) => {
                                vec![Some(range.clone()); 4]
                            }
                            Some(Limit::VecRange(range)) => range.clone(),
                            _ => Vec::new(),
                        };
                        Self::drag_float(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                        Self::drag_float(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                        Self::drag_float(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                        Self::drag_float(ui, &mut v.w, range.get(3).and_then(|r| r.clone()));
                    }
                });
            }
            Value::IVec2(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 2],
                        Some(Limit::IVecRange(range)) => range.clone(),
                        _ => Vec::new(),
                    };
                    Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                });
            }
            Value::IVec3(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 3],
                        Some(Limit::IVecRange(range)) => range.clone(),
                        _ => Vec::new(),
                    };
                    Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                });
            }
            Value::IVec4(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 4],
                        Some(Limit::IVecRange(range)) => range.clone(),
                        _ => Vec::new(),
                    };
                    Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.w, range.get(3).and_then(|r| r.clone()));
                });
            }
            Value::UVec2(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 2],
                        Some(Limit::UVecRange(range)) => range.clone(),
                        _ => Vec::new(),
                    };
                    Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                });
            }
            Value::UVec3(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 3],
                        Some(Limit::UVecRange(range)) => range.clone(),
                        _ => Vec::new(),
                    };
                    Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                });
            }
            Value::UVec4(v) => {
                ui.horizontal(|ui| {
                    let range = match limit {
                        Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 4],
                        Some(Limit::UVecRange(range)) => range.clone(),
                        _ => Vec::new(),
                    };
                    Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                    Self::drag_value(ui, &mut v.w, range.get(3).and_then(|r| r.clone()));
                });
            }
            Value::Entity(v) => {
                ui.label(format!("{v:?}")); // TODO: change entity in editor
            }
            Value::Asset(v) => {
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
                                match Project::get_asset_info_and_insert(
                                    &asset_dir, asset_file, app, false,
                                ) {
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
                });
            }
            Value::VecBool(v) => Self::vec_value_view(ui, v, color), // TODO: add/remove/change
            Value::VecInt32(v) => Self::vec_value_view(ui, v, color), // TODO: limit/add/remove/change
            Value::VecInt64(v) => Self::vec_value_view(ui, v, color), // TODO: limit/add/remove/change
            Value::VecUInt32(v) => Self::vec_value_view(ui, v, color), // TODO: limit/add/remove/change
            Value::VecUInt64(v) => Self::vec_value_view(ui, v, color), // TODO: limit/add/remove/change
            Value::VecFloat32(v) => Self::vec_value_view(ui, v, color), // TODO: limit/add/remove/change
            Value::VecFloat64(v) => Self::vec_value_view(ui, v, color), // TODO: limit/add/remove/change
            Value::VecString(v) => Self::vec_value_view(ui, v, color),  // TODO: add/remove/change
            Value::VecEntity(v) => Self::vec_value_view(ui, v, color),  // TODO: add/remove/change
            Value::VecAsset(v) => Self::vec_value_view(ui, v, color),   // TODO: add/remove/change
        }
    }

    fn color_label(ui: &mut egui::Ui, color: egui::Color32, text: impl Into<egui::WidgetText>) {
        egui::Frame::none()
            .inner_margin(egui::style::Margin::symmetric(3.0, 1.0))
            .rounding(egui::Rounding::same(3.0))
            .fill(color)
            .show(ui, |ui| ui.label(text));
    }

    /// Displays a DragValue for floats.
    fn drag_float<F: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        v: &mut F,
        range: Option<RangeInclusive<F>>,
    ) {
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

enum DropResult {
    Before,
    Into,
    After,
}
