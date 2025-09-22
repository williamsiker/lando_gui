use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use eframe::egui;
use crate::lando::{self, LandoCommandOutcome};
use crate::models::{LandoApp, LandoService};
use egui_term::{TerminalBackend, TerminalView, PtyEvent, BackendSettings, BackendCommand};
use crate::service_ui::ServiceUIManager;

pub struct LandoGui {
    // Estado de la UI
    apps: Vec<LandoApp>,
    projects: Vec<PathBuf>,
    selected_project_path: Option<PathBuf>,
    services: Vec<LandoService>,
    db_query_input: String,
    db_query_result: Option<String>,
    shell_command_input: String,
    error_message: Option<String>,
    success_message: Option<String>,
    is_loading: bool,

    sender: Sender<LandoCommandOutcome>,
    receiver: Receiver<LandoCommandOutcome>,

    // Terminal
    terminal: TerminalBackend,
    
    // Gestor de UIs especializadas
    service_ui_manager: ServiceUIManager,
    
    // Estado para controlar la interfaz de base de datos
    open_database_interface: Option<String>, // Nombre del servicio de BD abierto
}

impl LandoGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = mpsc::channel();

        // Channel for the terminal, as required by the constructor.
        // The receiver is not used because we don't process PTY events.
        let (pty_sender, _pty_receiver) = mpsc::channel::<(u64, PtyEvent)>();

        // Al iniciar, pedimos la lista de apps
        lando::list_apps(sender.clone());

        Self {
            apps: vec![],
            projects: vec![],
            selected_project_path: None,
            services: vec![],
            db_query_input: String::new(),
            db_query_result: None,
            shell_command_input: String::new(),
            error_message: None,
            success_message: None,
            is_loading: true, // Empezamos cargando
            sender,
            receiver,
            terminal: TerminalBackend::new(
                0, // A unique ID for the terminal
                cc.egui_ctx.clone(),
                pty_sender,
                BackendSettings::default(),
            )
            .expect("Failed to create TerminalBackend"),
            service_ui_manager: ServiceUIManager::default(),
            open_database_interface: None,
        }
    }
}

impl eframe::App for LandoGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Comprobar si hay nuevos resultados del hilo de trabajo (sin bloquear)
        if let Ok(outcome) = self.receiver.try_recv() {
            self.is_loading = false; // La mayoría de los mensajes implican que la carga ha terminado
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
                    self.db_query_result = Some(result.clone());
                    // Actualizar también las UIs especializadas de bases de datos
                    for database_ui in self.service_ui_manager.database_uis.values_mut() {
                        database_ui.process_query_result(result.clone(), false);
                    }
                },
                LandoCommandOutcome::Error(msg) => {
                    self.error_message = Some(msg.clone());
                    // Si el error es de una consulta, también lo mostramos en el resultado
                    if self.db_query_result.is_some() || !self.db_query_input.is_empty() {
                        self.db_query_result = self.error_message.clone();
                        // Actualizar también las UIs especializadas de bases de datos con el error
                        for database_ui in self.service_ui_manager.database_uis.values_mut() {
                            database_ui.process_query_result(msg.clone(), true);
                        }
                    }
                }
                LandoCommandOutcome::CommandSuccess(msg) => self.success_message = Some(msg),
                LandoCommandOutcome::FinishedLoading => { /* No hacer nada, solo parar el spinner */ }
                LandoCommandOutcome::LogOutput(output) => {
                    self.terminal.process_command(BackendCommand::Write(output));
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🚀 Lando GUI");
                ui.separator();
                
                // Estadísticas rápidas
                ui.label(format!("📦 Apps: {}", self.apps.len()));
                ui.label(format!("📂 Proyectos: {}", self.projects.len()));
                ui.label(format!("⚙️ Servicios: {}", self.services.len()));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_loading {
                        ui.spinner();
                        ui.label("Cargando...");
                    }
                    
                    if ui.button("🔄 Refrescar Todo").clicked() && !self.is_loading {
                        self.is_loading = true;
                        lando::list_apps(self.sender.clone());
                        // Refrescar proyecto actual si está seleccionado
                        if let Some(path) = &self.selected_project_path {
                            lando::get_project_info(self.sender.clone(), path.clone());
                        }
                    }
                    
                    if ui.button("🏠 Home").clicked() {
                        self.selected_project_path = None;
                        self.services.clear();
                        self.db_query_result = None;
                        self.error_message = None;
                        self.success_message = None;
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").resizable(true).default_width(280.0).show(ctx, |ui| {
            ui.heading("📁 Proyectos Lando");
            ui.separator();

            // Sección de búsqueda
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("🔍 Buscar Proyectos").clicked() && !self.is_loading {
                        self.is_loading = true;
                        let sender = self.sender.clone();

                        thread::spawn(move || {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                lando::scan_for_projects(sender, path);
                            } else {
                                let _ = sender.send(LandoCommandOutcome::FinishedLoading);
                            }
                        });
                    }
                    
                    if ui.small_button("🗑️").on_hover_text("Limpiar lista").clicked() {
                        self.projects.clear();
                        if self.selected_project_path.is_some() {
                            self.selected_project_path = None;
                            self.services.clear();
                        }
                    }
                });
            });

            ui.separator();
            
            // Sección específica para servicios de base de datos
            let database_services: Vec<_> = self.services.iter()
                .filter(|s| self.service_ui_manager.is_database_service(&s.service) || s.r#type.to_lowercase() == "database")
                .collect();
            
            if !database_services.is_empty() {
                ui.collapsing(format!("🗄️ Bases de Datos ({})", database_services.len()), |ui| {
                    for service in &database_services {
                        ui.horizontal(|ui| {
                            ui.label(format!("📊 {}", service.service));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("🚀").on_hover_text("Abrir interfaz").clicked() {
                                    self.open_database_interface = Some(service.service.clone());
                                }
                            });
                        });
                        
                        if let Some(creds) = &service.creds {
                            if let Some(database) = &creds.database {
                                ui.label(format!("💾 {}", database));
                            }
                        }
                        
                        ui.separator();
                    }
                });
                ui.separator();
            }
            
            // Sección de proyectos descubiertos
            ui.collapsing(format!("📂 Proyectos Descubiertos ({})", self.projects.len()), |ui| {
                if self.projects.is_empty() {
                    ui.label("💭 No hay proyectos descubiertos");
                    ui.label("Usa el botón 'Buscar Proyectos' para encontrarlos");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            let previous_path = self.selected_project_path.clone();
                            let mut selected_path = self.selected_project_path.clone();
                            
                            for project_path in &self.projects {
                                let project_name = project_path.file_name().unwrap_or_default().to_string_lossy();
                                let is_selected = self.selected_project_path.as_ref() == Some(project_path);
                                
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(is_selected, format!("📁 {}", project_name)).clicked() {
                                        selected_path = Some(project_path.clone());
                                    }
                                    
                                if ui.small_button("📄").on_hover_text("Copiar ruta").clicked() {
                                    ctx.copy_text(project_path.to_string_lossy().to_string());
                                }
                                });
                            }
                            
                            self.selected_project_path = selected_path;
                            
                            if self.selected_project_path != previous_path {
                                if let Some(path) = &self.selected_project_path {
                                    self.is_loading = true;
                                    self.services.clear();
                                    self.db_query_input.clear();
                                    self.db_query_result = None;
                                    self.shell_command_input.clear();
                                    lando::get_project_info(self.sender.clone(), path.clone());
                                }
                            }
                        });
                }
            });

            ui.separator();
            
            // Sección de aplicaciones en ejecución
            ui.collapsing(format!("⚙️ Apps en Ejecución ({})", self.apps.len()), |ui| {
                if self.apps.is_empty() {
                    ui.label("💭 No hay aplicaciones ejecutándose");
                } else {
                    for app in &self.apps {
                        ui.horizontal(|ui| {
                            ui.label(format!("🚀 {}", &app.name));
                        });
                    }
                }
            });
            
            // Información del proyecto seleccionado
            if let Some(selected_path) = &self.selected_project_path {
                ui.separator();
                ui.strong("🎯 Proyecto Actual:");
                ui.label(format!("📝 {}", selected_path.file_name().unwrap_or_default().to_string_lossy()));
                ui.label(format!("📂 {}", selected_path.display()));
                
                if !self.services.is_empty() {
                    ui.label(format!("⚙️ {} servicios activos", self.services.len()));
                }
            }
        });

        egui::TopBottomPanel::bottom("terminal_panel")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                TerminalView::new(ui, &mut self.terminal);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(selected_path) = &self.selected_project_path {
                // Cabecera del proyecto
                ui.horizontal(|ui| {
                    ui.heading(format!("🏠 {}", selected_path.file_name().unwrap_or_default().to_string_lossy()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("📂 {}", selected_path.display()));
                    });
                });
                ui.separator();

                // Controles de Lando
                ui.group(|ui| {
                    ui.label("⚙️ Controles de Lando:");
                    ui.horizontal_wrapped(|ui| {
                        let commands = vec![
                            ("▶️ start", "start", egui::Color32::GREEN),
                            ("⏹️ stop", "stop", egui::Color32::RED),
                            ("🔄 restart", "restart", egui::Color32::YELLOW),
                            ("🔧 rebuild", "rebuild", egui::Color32::BLUE),
                            ("⏻ poweroff", "poweroff", egui::Color32::DARK_RED),
                        ];
                        
                        for (label, cmd, color) in commands {
                            let btn = ui.add_enabled(!self.is_loading, 
                                egui::Button::new(label).fill(color.gamma_multiply(0.1))
                            );
                            
                            if btn.clicked() {
                                self.is_loading = true;
                                lando::run_lando_command(self.sender.clone(), cmd.to_string(), selected_path.clone());
                            }
                        }
                    });
                });
                ui.separator();

                // Sección específica para servicios de base de datos
                let database_services: Vec<_> = self.services.iter()
                    .filter(|s| self.service_ui_manager.is_database_service(&s.service) || s.r#type.to_lowercase() == "database")
                    .collect();
                
                if !database_services.is_empty() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(format!("🗄️ Servicios de Base de Datos ({})", database_services.len()));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("🔄").on_hover_text("Refrescar servicios").clicked() && !self.is_loading {
                                    self.is_loading = true;
                                    lando::get_project_info(self.sender.clone(), selected_path.clone());
                                }
                            });
                        });
                        
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                for service in &database_services {
                                    ui.push_id(&format!("db_{}", service.service), |ui| {
                                        self.service_ui_manager.show_service_details(
                                            ui,
                                            service,
                                            &selected_path.clone(),
                                            &self.sender,
                                            &mut self.is_loading,
                                            &mut self.terminal,
                                        );
                                    });
                                    ui.separator();
                                }
                            });
                    });
                    ui.separator();
                }

                // Interfaz de base de datos abierta
                if let Some(open_db_service) = &self.open_database_interface {
                    if let Some(service) = self.services.iter().find(|s| s.service == *open_db_service) {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading(format!("🗄️ Interfaz de Base de Datos: {}", service.service));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("❌ Cerrar").clicked() {
                                        self.open_database_interface = None;
                                    }
                                });
                            });
                            
                            ui.separator();
                            
                            // Mostrar la interfaz completa de base de datos
                            let service_key = format!("{}_{}", service.service, service.r#type);
                            if let Some(database_ui) = self.service_ui_manager.database_uis.get_mut(&service_key) {
                                database_ui.show_full_interface(ui, service, &selected_path.clone(), &self.sender, &mut self.is_loading, &mut self.terminal);
                            }
                        });
                        ui.separator();
                    }
                }

                // Sección de servicios
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(format!("⚙️ Servicios ({})", self.services.len()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("🔄").on_hover_text("Refrescar servicios").clicked() && !self.is_loading {
                                self.is_loading = true;
                                lando::get_project_info(self.sender.clone(), selected_path.clone());
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
                                    self.service_ui_manager.show_service_details(
                                        ui,
                                        service,
                                        &selected_path_clone,
                                        &self.sender,
                                        &mut self.is_loading,
                                        &mut self.terminal,
                                    );
                                });
                                ui.separator();
                            }
                        });

                    // Sección de resultados de consultas
                    if let Some(result) = &self.db_query_result {
                        ui.separator();
                        let result_clone = result.clone();
                        let mut clear_result = false;
                        let mut copy_result = false;
                        
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong("📊 Resultado de la Consulta:");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("🔄").on_hover_text("Limpiar resultado").clicked() {
                                        clear_result = true;
                                    }
                                    if ui.small_button("📋").on_hover_text("Copiar resultado").clicked() {
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
                        
                        // Procesar acciones fuera del closure
                        if clear_result {
                            self.db_query_result = None;
                        }
                        if copy_result {
                            ctx.copy_text(result_clone);
                        }
                    }
                } else if !self.is_loading {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("🔍 No se encontraron servicios");
                        ui.label("Este proyecto no tiene servicios configurados o no se han cargado aún.");
                        ui.add_space(20.0);
                        if ui.button("🔄 Intentar recargar").clicked() {
                            self.is_loading = true;
                            lando::get_project_info(self.sender.clone(), selected_path.clone());
                        }
                        ui.add_space(50.0);
                    });
                }

            } else {
                // Pantalla de bienvenida
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("🚀 Bienvenido a Lando GUI");
                    ui.add_space(20.0);
                    ui.label("📁 Selecciona un proyecto de la lista lateral para comenzar");
                    ui.label("🔍 Si no tienes proyectos, usa el botón 'Buscar Proyectos'");
                    ui.add_space(30.0);
                    
                    ui.group(|ui| {
                        ui.label("💡 Características:");
                        ui.label("• 📊 Gestiona bases de datos con interfaz SQL avanzada");
                        ui.label("• 🌐 Controla servidores web con logs y configuración");
                        ui.label("• 🟢 Administra servicios Node.js con NPM y PM2");
                        ui.label("• 💻 Terminal integrado para comandos avanzados");
                    });
                    
                    ui.add_space(100.0);
                });
            }

            // Sección de mensajes (siempre al final)
            ui.separator();
            let mut clear_error = false;
            let mut clear_success = false;
            
            if let Some(err) = &self.error_message {
                if self.db_query_result.is_none() { // Solo mostrar si no es un error de consulta
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::RED, format!("❌ Error: {}", err));
                        if ui.small_button("✖").on_hover_text("Cerrar").clicked() {
                            clear_error = true;
                        }
                    });
                }
            } else if let Some(success) = &self.success_message {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::GREEN, format!("✅ Éxito: {}", success));
                    if ui.small_button("✖").on_hover_text("Cerrar").clicked() {
                        clear_success = true;
                    }
                });
            }
            
            // Procesar acciones fuera de los closures
            if clear_error {
                self.error_message = None;
            }
            if clear_success {
                self.success_message = None;
            }
        });
    }
}
