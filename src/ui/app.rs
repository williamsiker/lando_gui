use std::cell::Cell;
use crate::core::commands::*;
use crate::models::app::LandoGui;
use crate::models::commands::LandoCommandOutcome;
use crate::models::lando::LandoService;
use eframe::egui;
use egui_term::{BackendCommand, TerminalView};
use std::thread;

impl eframe::App for LandoGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_receiver_messages(ctx);
        self.show_terminal_popup(ctx);

        self.show_top_panel(ctx);
        self.show_side_panel(ctx);
        self.show_central_panel(ctx);
    }
}

impl LandoGui {
    fn handle_receiver_messages(&mut self, ctx: &egui::Context) {
        if let Ok(outcome) = self.receiver.try_recv() {
            self.is_loading.set(false);
            self.error_message = None;
            self.success_message = None;

            match outcome {
                LandoCommandOutcome::List(apps) => self.apps = apps,
                LandoCommandOutcome::Projects(new_projects) => {
                    self.projects.extend(new_projects);
                    self.projects.sort();
                    self.projects.dedup();
                }
                LandoCommandOutcome::Info(services) => self.services = services,
                LandoCommandOutcome::DbQueryResult(result) => {
                    self.handle_db_query_result(result);
                },
                LandoCommandOutcome::Error(msg) => {
                    self.handle_error_message(msg);
                }
                LandoCommandOutcome::CommandSuccess(msg) => self.success_message = Some(msg),
                LandoCommandOutcome::FinishedLoading => { /* No hacer nada */ }
                LandoCommandOutcome::LogOutput(output) => {
                    self.handle_log_output(output);
                }
            }
        }
    }

    fn handle_db_query_result(&mut self, result: String) {
        self.db_query_result = Some(result.clone());
        for database_ui in self.service_ui_manager.take().database_uis.values_mut() {
            database_ui.process_query_result(result.clone(), false);
        }
    }

    fn handle_error_message(&mut self, msg: String) {
        self.error_message = Some(msg.clone());
        if self.db_query_result.is_some() || !self.db_query_input.is_empty() {
            self.db_query_result = self.error_message.clone();
            for database_ui in self.service_ui_manager.take().database_uis.values_mut() {
                database_ui.process_query_result(msg.clone(), true);
            }
        }
    }

    fn handle_log_output(&mut self, output: Vec<u8>) {
        self.log_buffer.push(String::try_from(output.clone().to_owned()).unwrap());
        if self.terminal_filter.is_empty()
            || String::from_utf8_lossy(&output).contains(self.terminal_filter.as_str())
        {
            self.terminal.borrow_mut().process_command(BackendCommand::Write(output));
        }
        self.show_terminal_popup = true;
    }

    fn show_terminal_popup(&mut self, ctx: &egui::Context) {
        if !self.show_terminal_popup {
            return;
        }

        egui::Window::new("📟 Terminal de Logs ")
            .resizable(true)
            .default_width(800.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                self.render_terminal_controls(ui);
                ui.separator();
                TerminalView::new(ui, &mut self.terminal.borrow_mut());
            });
    }

    fn render_terminal_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("🔍 Filtro:");
            if ui.text_edit_singleline(&mut self.terminal_filter).changed() {
                self.reapply_terminal_filter();
            }
            if ui.button("🗑️ Limpiar ").clicked() {
                self.clear_terminal();
            }
        });
    }

    fn reapply_terminal_filter(&mut self) {
        self.terminal.borrow_mut().process_command(BackendCommand::Write("clear".into()));
        for log in &self.log_buffer {
            if self.terminal_filter.is_empty() || log.contains(&self.terminal_filter) {
                self.terminal.borrow_mut().process_command(BackendCommand::Write(log.clone().into()));
            }
        }
    }

    fn clear_terminal(&mut self) {
        self.terminal.borrow_mut().process_command(BackendCommand::Write("clear".into()));
        self.log_buffer.clear();
        self.terminal_filter.clear();
    }

    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🚀 Lando GUI ");
                ui.separator();
                self.render_quick_stats(ui);
                self.render_top_controls(ui);
            });
        });
    }

    fn render_quick_stats(&self, ui: &mut egui::Ui) {
        ui.label(format!("📦 Apps: {}", self.apps.len()));
        ui.label(format!("📂 Proyectos: {}", self.projects.len()));
        ui.label(format!("⚙️ Servicios: {}", self.services.len()));
    }

    fn render_top_controls(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if self.is_loading.get() {
                ui.spinner();
                ui.label("Cargando...");
            }

            if ui.button("🔄 Refrescar Todo ").clicked() && !self.is_loading.get() {
                self.refresh_all();
            }

            if ui.button("📟 Terminal ").clicked() {
                self.show_terminal_popup = !self.show_terminal_popup;
            }

            if ui.button("🏠 Home ").clicked() {
                self.navigate_home();
            }
        });
    }

    fn refresh_all(&mut self) {
        self.is_loading.set(true);
        list_apps(self.sender.clone());
        if let Some(path) = &self.selected_project_path {
            get_project_info(self.sender.clone(), path.clone());
        }
    }

    fn navigate_home(&mut self) {
        self.selected_project_path = None;
        self.services.clear();
        self.db_query_result = None;
        self.error_message = None;
        self.success_message = None;
    }

    fn show_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("📁 Proyectos Lando ");
                ui.separator();

                self.render_project_search_section(ui);
                ui.separator();

                self.render_database_services_section(ui);
                ui.separator();

                self.render_discovered_projects_section(ui);
                ui.separator();

                self.render_running_apps_section(ui);
                self.render_selected_project_info(ui);
            });
    }

    fn render_project_search_section(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui.button("🔍 Buscar Proyectos ").clicked() && !self.is_loading.get() {
                    self.is_loading.set(true);
                    let sender = self.sender.clone();

                    thread::spawn(move || {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            scan_for_projects(sender, path);
                        } else {
                            let _ = sender.send(LandoCommandOutcome::FinishedLoading);
                        }
                    });
                }

                if ui.small_button("🗑️").on_hover_text("Limpiar lista ").clicked() {
                    self.clear_projects_list();
                }
            });
        });
    }

    fn clear_projects_list(&mut self) {
        self.projects.clear();
        if self.selected_project_path.is_some() {
            self.selected_project_path = None;
            self.services.clear();
        }
    }

    fn get_database_services(&self) -> Vec<&LandoService> {
        self.services.iter()
            .filter(|s| self.service_ui_manager.borrow_mut().is_database_service(&s.service) ||
                s.r#type.to_lowercase() == "database")
            .collect()
    }

    fn render_database_services_section(&mut self, ui: &mut egui::Ui) {
        let services_info: Vec<_> = self.get_database_services()
            .iter()
            .map(|s| (
                s.service.clone(),
                s.creds.as_ref().and_then(|c| c.database.clone())
            ))
            .collect();

        if services_info.is_empty(){
            return;
        }

        let header = format!("🗄️ Bases de Datos ({})", services_info.len());
        ui.collapsing(header, |ui| {
            for (service_name,database) in &services_info  {
                self.render_database_service_item_ui(ui, service_name, database.as_deref());
                ui.separator();
            }
        });
    }

    fn render_database_service_item_ui(
        &mut self,
        ui: &mut egui::Ui,
        service_name: &str,
        database: Option<&str>,
    ) {
        ui.horizontal(|ui| {
            ui.label(format!("📊 {}", service_name));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("🚀").on_hover_text("Abrir interfaz ").clicked() {
                    self.open_database_interface = Some(service_name.to_string());
                }
            });
        });

        if let Some(database) = database {
            ui.label(format!("💾 {}", database));
        }
    }

    fn render_discovered_projects_section(&mut self, ui: &mut egui::Ui) {
        ui.collapsing(format!("📂 Proyectos Descubiertos ({})", self.projects.len()), |ui| {
            if self.projects.is_empty() {
                self.render_empty_projects_message(ui);
            } else {
                self.render_projects_list(ui);
            }
        });
    }

    fn render_empty_projects_message(&self, ui: &mut egui::Ui) {
        ui.label("💭 No hay proyectos descubiertos ");
        ui.label(r#"Usa el botón "Buscar Proyectos" para encontrarlos "#);
    }

    fn render_projects_list(&mut self, ui: &mut egui::Ui) {
        // 1. Primero recolectar todos los datos necesarios (solo lectura)
        let projects: Vec<_> = self.projects.iter().cloned().collect();
        let previous_selection = self.selected_project_path.clone();

        // 2. Variable para capturar la nueva selección
        let mut new_selection = previous_selection.clone();

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for project_path in &projects {
                    let selected = self.render_project_item_ui(ui, project_path, &new_selection);
                    if selected {
                        new_selection = Some(project_path.clone());
                    }
                }
            });

        // 3. Aplicar los cambios fuera del closure
        if new_selection != previous_selection {
            self.selected_project_path = new_selection.clone();
            self.handle_project_selection_change(previous_selection);
        }
    }

    fn render_project_item_ui(
        &self,  // ¡Note: &self en lugar de &mut self!
        ui: &mut egui::Ui,
        project_path: &std::path::PathBuf,
        current_selection: &Option<std::path::PathBuf>,
    ) -> bool {
        let project_name = project_path.file_name().unwrap_or_default().to_string_lossy();
        let is_selected = current_selection.as_ref() == Some(project_path);

        let mut was_clicked = false;
        let mut copy_clicked = false;

        ui.horizontal(|ui| {
            if ui.selectable_label(is_selected, format!("📁 {}", project_name)).clicked() {
                was_clicked = true;
            }

            if ui.small_button("📄").on_hover_text("Copiar ruta ").clicked() {
                copy_clicked = true;
            }
        });

        // Manejar la copia inmediatamente (no afecta el estado de self)
        if copy_clicked {
            ui.ctx().copy_text(project_path.to_string_lossy().to_string());
        }

        was_clicked
    }
    fn handle_project_selection_change(&mut self, previous_path: Option<std::path::PathBuf>) {
        if self.selected_project_path != previous_path {
            if let Some(path) = &self.selected_project_path {
                self.is_loading.set(true);
                self.services.clear();
                self.db_query_input.clear();
                self.db_query_result = None;
                self.shell_command_input.clear();
                get_project_info(self.sender.clone(), path.clone());
            }
        }
    }

    fn render_running_apps_section(&self, ui: &mut egui::Ui) {
        ui.collapsing(format!("⚙️ Apps en Ejecución ({})", self.apps.len()), |ui| {
            if self.apps.is_empty() {
                ui.label("💭 No hay aplicaciones ejecutándose ");
            } else {
                for app in &self.apps {
                    ui.horizontal(|ui| {
                        ui.label(format!("🚀 {}", &app.name));
                    });
                }
            }
        });
    }

    fn render_selected_project_info(&self, ui: &mut egui::Ui) {
        if let Some(selected_path) = &self.selected_project_path {
            ui.separator();
            ui.strong("🎯 Proyecto Actual:");
            ui.label(format!("📝 {}", selected_path.file_name().unwrap_or_default().to_string_lossy()));
            ui.label(format!("📂 {}", selected_path.display()));

            if !self.services.is_empty() {
                ui.label(format!("⚙️ {} servicios activos ", self.services.len()));
            }
        }
    }

    fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let selected_path = self.selected_project_path.clone();
            if let Some(selected_path) = selected_path {
                self.render_project_interface(ui, &selected_path);
            } else {
                self.render_welcome_screen(ui);
            }

            ui.separator();
        });
    }

    fn render_project_interface(&mut self, ui: &mut egui::Ui, selected_path: &std::path::PathBuf) {
        self.render_project_header(ui, selected_path);
        ui.separator();

        self.render_lando_controls(ui, selected_path);
        ui.separator();

        self.render_database_services_interface(ui, selected_path);

        self.render_open_database_interface(ui, selected_path);

        self.render_services_section(ui, selected_path);

        self.render_query_results_section(ui);
    }

    fn render_project_header(&self, ui: &mut egui::Ui, selected_path: &std::path::PathBuf) {
        ui.horizontal(|ui| {
            ui.heading(format!("🏠 {}", selected_path.file_name().unwrap_or_default().to_string_lossy()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("📂 {}", selected_path.display()));
            });
        });
    }

    fn render_lando_controls(&mut self, ui: &mut egui::Ui, selected_path: &std::path::PathBuf) {
        ui.group(|ui| {
            ui.label("⚙️ Controles de Lando:");
            ui.horizontal_wrapped(|ui| {
                let commands = vec![
                    ("▶️ start ", "start", egui::Color32::GREEN),
                    ("⏹️ stop ", "stop", egui::Color32::RED),
                    ("🔄 restart ", "restart", egui::Color32::YELLOW),
                    ("🔧 rebuild ", "rebuild", egui::Color32::BLUE),
                    ("poweroff ", "poweroff", egui::Color32::DARK_RED),
                ];

                for (label, cmd, color) in commands {
                    let btn = ui.add_enabled(!self.is_loading.get(),
                                             egui::Button::new(label).fill(color.gamma_multiply(0.1))
                    );

                    if btn.clicked() {
                        self.is_loading.set(true);
                        run_lando_command(self.sender.clone(), cmd.to_string(), selected_path.clone());
                    }
                }
            });
        });
    }

    fn render_database_services_interface(
        &mut self,
        ui: &mut egui::Ui,
        selected_path: &std::path::PathBuf,
    ) {
        let database_services: Vec<_> = self.get_database_services().to_vec();
        if database_services.is_empty() {
            return;
        }

        let sender_clone = self.sender.clone();
        let path_clone = selected_path.clone();

        let is_loading = self.is_loading.clone(); // copia (bool implementa Copy)

        let service_ui_manager = &self.service_ui_manager;
        let terminal = &self.terminal;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("🗄️ Servicios de Base de Datos ({})", database_services.len()));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("🔄")
                        .on_hover_text("Refrescar servicios")
                        .clicked()
                        && !is_loading.get()
                    {
                        sender_clone.send(LandoCommandOutcome::CommandSuccess("refresh".to_string())).ok();
                        get_project_info(sender_clone.clone(), path_clone.clone());
                    }
                });
            });

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for service in &database_services {
                        ui.push_id(&format!("db_{}", service.service), |ui| {
                            service_ui_manager.borrow_mut().show_service_details(
                                ui,
                                service,
                                &path_clone,
                                &sender_clone,
                                // Aquí mejor pasar flags por RefCell o Arc<Mutex>
                                &mut self.is_loading.get(),
                                &mut *terminal.borrow_mut(),
                            );
                        });
                        ui.separator();
                    }
                });
        });
        ui.separator();
    }


    fn render_open_database_interface(&mut self, ui: &mut egui::Ui, selected_path: &std::path::PathBuf) {
        if let Some(open_db_service) = &self.open_database_interface {
            if let Some(service) = self.services.iter().find(|s| s.service == *open_db_service) {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(format!("🗄️ Interfaz de Base de Datos: {}", service.service));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("❌ Cerrar ").clicked() {
                                self.open_database_interface = None;
                            }
                        });
                    });

                    ui.separator();

                    let service_key = format!("{}_{}", service.service, service.r#type);
                    if let Some(database_ui) = self.service_ui_manager.borrow_mut().database_uis.get_mut(&service_key) {
                        database_ui.show_full_interface(
                            ui,
                            service,
                            &selected_path.clone(),
                            &self.sender,
                            &mut self.is_loading.get(),
                            &mut self.terminal.borrow_mut()
                        );
                    }
                });
                ui.separator();
            }
        }
    }

    fn render_services_section(&mut self, ui: &mut egui::Ui, selected_path: &std::path::PathBuf) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("⚙️ Servicios ({})", self.services.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🔄").on_hover_text("Refrescar servicios ").clicked() && !self.is_loading.get() {
                        self.is_loading.set(true) ;
                        get_project_info(self.sender.clone(), selected_path.clone());
                    }
                });
            });
        });

        if !self.services.is_empty() {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let services = self.services.clone();
                    let selected_path_clone = selected_path.clone();

                    for service in &services {
                        ui.push_id(&service.service, |ui| {
                            self.service_ui_manager.borrow_mut().show_service_details(
                                ui,
                                service,
                                &selected_path_clone,
                                &self.sender,
                                &mut self.is_loading.get(),
                                &mut self.terminal.borrow_mut(),
                            );
                        });
                        ui.separator();
                    }
                });
        } else if !self.is_loading.get() {
            self.render_no_services_message(ui, selected_path);
        }
    }

    fn render_no_services_message(&mut self, ui: &mut egui::Ui, selected_path: &std::path::PathBuf) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("🔍 No se encontraron servicios ");
            ui.label("Este proyecto no tiene servicios configurados o no se han cargado aún.");
            ui.add_space(20.0);
            if ui.button("🔄 Intentar recargar ").clicked() {
                self.is_loading.set(true);
                get_project_info(self.sender.clone(), selected_path.clone());
            }
            ui.add_space(50.0);
        });
        //df
    }

    fn render_query_results_section(&mut self, ui: &mut egui::Ui) {
        if let Some(result) = &self.db_query_result {
            ui.separator();
            let result_clone = result.clone();
            let mut clear_result = false;
            let mut copy_result = false;

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("📊 Resultado de la Consulta:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("🔄").on_hover_text("Limpiar resultado ").clicked() {
                            clear_result = true;
                        }
                        if ui.small_button("📋").on_hover_text("Copiar resultado ").clicked() {
                            copy_result = true;
                        }
                    });
                });

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        let mut result_str = result_clone.clone();
                        ui.add(
                            egui::TextEdit::multiline(&mut result_str)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    });
            });

            if clear_result {
                self.db_query_result = None;
            }
            if copy_result {
                ui.ctx().copy_text(result_clone);
            }
        }
    }

    fn render_welcome_screen(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("🚀 Bienvenido a Lando GUI ");
            ui.add_space(20.0);
            ui.add_space(30.0);
            ui.add_space(100.0);
        });
    }
}