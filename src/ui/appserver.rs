use std::path::PathBuf;
use std::sync::mpsc::Sender;

use eframe::egui;
use egui_term::TerminalBackend;

use crate::models::commands::LandoCommandOutcome;
use crate::models::lando::LandoService;

pub struct AppServerUI {
    pub command_input: String,
    pub command_history: Vec<String>,
    pub logs_output: String,
    pub config_content: String,
    pub selected_config_file: String,
    pub available_configs: Vec<String>,
    pub service_status: ServiceStatus,
    pub auto_refresh_logs: bool,
    pub log_level_filter: LogLevel,
    pub current_tab: AppServerTab,
    pub restart_in_progress: bool,
    pub environment_vars: Vec<(String, String)>,
    pub new_env_key: String,
    pub new_env_value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Error(String),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    All,
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppServerTab {
    Control,
    Logs,
    Configuration,
    Environment,
    Monitoring,
}

impl Default for AppServerUI {
    fn default() -> Self {
        Self {
            command_input: String::new(),
            command_history: Vec::new(),
            logs_output: String::new(),
            config_content: String::new(),
            selected_config_file: String::new(),
            available_configs: vec![
                "apache.conf".to_string(),
                "nginx.conf".to_string(),
                "php.ini".to_string(),
                ".htaccess".to_string(),
            ],
            service_status: ServiceStatus::Unknown,
            auto_refresh_logs: false,
            log_level_filter: LogLevel::All,
            current_tab: AppServerTab::Control,
            restart_in_progress: false,
            environment_vars: Vec::new(),
            new_env_key: String::new(),
            new_env_value: String::new(),
        }
    }
}

impl AppServerUI {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
        terminal: &mut TerminalBackend,
    ) {
        ui.collapsing(format!("🔥️ App Server: {} ({})", service.service, service.r#type), |ui| {
            // Información del servicio y estado
            self.show_service_header(ui, service);
            
            ui.separator();
            
            // Pestañas de navegación
            self.show_tab_navigation(ui);
            
            ui.separator();

            // Contenido según la pestaña seleccionada
            match self.current_tab {
                AppServerTab::Control => {
                    self.show_control_panel(ui, service, project_path, sender, is_loading);
                }
                AppServerTab::Logs => {
                    self.show_logs_panel(ui, service, project_path, sender, is_loading);
                }
                AppServerTab::Configuration => {
                    self.show_configuration_panel(ui, service, project_path, sender, is_loading);
                }
                AppServerTab::Environment => {
                    self.show_environment_panel(ui, service, project_path, sender, is_loading);
                }
                AppServerTab::Monitoring => {
                    self.show_monitoring_panel(ui, service, project_path, sender, is_loading);
                }
            }

            // Terminal embebido
            self.show_terminal_section(ui, terminal);
        });
    }

    fn show_service_header(&mut self, ui: &mut egui::Ui, service: &LandoService) {
        ui.horizontal(|ui| {
            // Información básica
            ui.vertical(|ui| {
                ui.label(format!("🏷️ Tipo: {}", service.r#type));
                ui.label(format!("📦 Versión: {}", service.version));
                
                if let Some(conn) = &service.external_connection {
                    ui.label(format!("🌐 {}:{}", conn.host, conn.port));
                }
            });

            ui.separator();

            // Estado del servicio
            ui.vertical(|ui| {
                ui.label("Estado del Servicio:");
                let (color, icon, text) = match &self.service_status {
                    ServiceStatus::Running => (egui::Color32::GREEN, "✅", "Ejecutándose"),
                    ServiceStatus::Stopped => (egui::Color32::RED, "⏹️", "Detenido"),
                    ServiceStatus::Error(err) => (egui::Color32::RED, "❌", err.as_str()),
                    ServiceStatus::Unknown => (egui::Color32::GRAY, "❓", "Desconocido"),
                };
                
                ui.colored_label(color, format!("{} {}", icon, text));
                
                if ui.small_button("🔄 Actualizar Estado").clicked() {
                    self.refresh_service_status();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Botones de acción rápida
                if ui.button("🔄 Restart").clicked() {
                    self.restart_service();
                }
                if ui.button("⏸️ Stop").clicked() {
                    self.stop_service();
                }
                if ui.button("▶️ Start").clicked() {
                    self.start_service();
                }
            });
        });
    }

    fn show_tab_navigation(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.current_tab, AppServerTab::Control, "🎛️ Control");
            ui.selectable_value(&mut self.current_tab, AppServerTab::Logs, "📜 Logs");
            ui.selectable_value(&mut self.current_tab, AppServerTab::Configuration, "⚙️ Config");
            ui.selectable_value(&mut self.current_tab, AppServerTab::Environment, "🌍 Env");
            ui.selectable_value(&mut self.current_tab, AppServerTab::Monitoring, "📊 Monitor");
        });
    }

    fn show_control_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("🎛️ Panel de Control");

        // Controles del servicio
        ui.group(|ui| {
            ui.label("Gestión del Servicio:");
            
            ui.horizontal(|ui| {
                let restart_btn = ui.add_enabled(
                    !self.restart_in_progress && !*is_loading,
                    egui::Button::new("🔄 Reiniciar Servicio")
                );
                if restart_btn.clicked() {
                    self.restart_service_with_feedback(service, project_path, sender, is_loading);
                }

                if ui.button("⏹️ Detener").clicked() {
                    self.stop_service_with_feedback(service, project_path, sender, is_loading);
                }

                if ui.button("▶️ Iniciar").clicked() {
                    self.start_service_with_feedback(service, project_path, sender, is_loading);
                }
            });

            ui.horizontal(|ui| {
                if ui.button("🔄 Reload Config").clicked() {
                    self.reload_configuration(service, project_path, sender, is_loading);
                }

                if ui.button("🧹 Clear Cache").clicked() {
                    self.clear_cache(service, project_path, sender, is_loading);
                }

                if ui.button("🔍 Test Connection").clicked() {
                    self.test_connection(service, project_path, sender, is_loading);
                }
            });
        });

        ui.separator();

        // Comandos personalizados
        ui.group(|ui| {
            ui.label("Comandos Personalizados:");
            
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.command_input);
                
                let execute_btn = ui.add_enabled(!*is_loading, egui::Button::new("▶️ Ejecutar"));
                if execute_btn.clicked() {
                    self.execute_custom_command(service, project_path, sender, is_loading);
                }
            });

            // Comandos predefinidos
            ui.horizontal(|ui| {
                if ui.button("📊 Status").clicked() {
                    self.command_input = "status".to_string();
                }
                if ui.button("📋 Info").clicked() {
                    self.command_input = "info".to_string();
                }
                if ui.button("🔧 List Modules").clicked() {
                    self.command_input = self.get_list_modules_command(&service.r#type);
                }
            });

            // Historial de comandos
            if !self.command_history.is_empty() {
                ui.collapsing("📜 Historial", |ui| {
                    for cmd in &self.command_history {
                        if ui.small_button(cmd).clicked() {
                            self.command_input = cmd.clone();
                        }
                    }
                });
            }
        });
    }

    fn show_logs_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("📜 Logs del Servidor");

        // Controles de logs
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.auto_refresh_logs, "🔄 Auto-refresh");
            
            ui.label("Nivel:");
            egui::ComboBox::from_label("")
                .selected_text(format!("{:?}", self.log_level_filter))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.log_level_filter, LogLevel::All, "Todos");
                    ui.selectable_value(&mut self.log_level_filter, LogLevel::Error, "Error");
                    ui.selectable_value(&mut self.log_level_filter, LogLevel::Warning, "Warning");
                    ui.selectable_value(&mut self.log_level_filter, LogLevel::Info, "Info");
                    ui.selectable_value(&mut self.log_level_filter, LogLevel::Debug, "Debug");
                });

            if ui.button("🔄 Actualizar").clicked() {
                self.refresh_logs(service, project_path, sender, is_loading);
            }

            if ui.button("🗑️ Limpiar").clicked() {
                self.logs_output.clear();
            }

            if ui.button("💾 Exportar").clicked() {
                self.export_logs();
            }
        });

        ui.separator();

        // Botones para diferentes tipos de logs
        ui.horizontal(|ui| {
            if ui.button("📝 Access Log").clicked() {
                self.show_access_logs(service, project_path, sender, is_loading);
            }
            if ui.button("❌ Error Log").clicked() {
                self.show_error_logs(service, project_path, sender, is_loading);
            }
            if ui.button("🔍 Debug Log").clicked() {
                self.show_debug_logs(service, project_path, sender, is_loading);
            }
        });

        ui.separator();

        // Área de logs
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(400.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.logs_output)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                );
            });
    }

    fn show_configuration_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("⚙️ Configuración del Servidor");

        // Selector de archivo de configuración
        ui.horizontal(|ui| {
            ui.label("Archivo de configuración:");
            egui::ComboBox::from_label("")
                .selected_text(&self.selected_config_file)
                .show_ui(ui, |ui| {
                    for config in &self.available_configs {
                        ui.selectable_value(&mut self.selected_config_file, config.clone(), config);
                    }
                });

            if ui.button("🔄 Cargar").clicked() {
                self.load_config_file(service, project_path, sender, is_loading);
            }

            if ui.button("💾 Guardar").clicked() {
                self.save_config_file(service, project_path, sender, is_loading);
            }

            if ui.button("🔙 Backup").clicked() {
                self.backup_config_file(service, project_path, sender, is_loading);
            }
        });

        ui.separator();

        // Editor de configuración
        ui.label("Contenido del archivo:");
        egui::ScrollArea::vertical()
            .max_height(500.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.config_content)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                );
            });

        ui.separator();

        // Validación y herramientas
        ui.horizontal(|ui| {
            if ui.button("✅ Validar Sintaxis").clicked() {
                self.validate_config(service, project_path, sender, is_loading);
            }

            if ui.button("🔧 Test Config").clicked() {
                self.test_config(service, project_path, sender, is_loading);
            }

            if ui.button("📋 Mostrar Diferencias").clicked() {
                self.show_config_diff(service, project_path, sender, is_loading);
            }
        });
    }

    fn show_environment_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("🌍 Variables de Entorno");

        // Agregar nueva variable
        ui.group(|ui| {
            ui.label("Agregar nueva variable:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_env_key);
                ui.label("=");
                ui.text_edit_singleline(&mut self.new_env_value);
                
                if ui.button("➕ Agregar").clicked() {
                    self.add_environment_variable();
                }
            });
        });

        ui.separator();

        // Lista de variables existentes
        ui.label("Variables actuales:");
        
        let mut to_remove = None;
        for (i, (key, value)) in self.environment_vars.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(key);
                ui.label("=");
                ui.text_edit_singleline(value);
                
                if ui.button("🗑️").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        if let Some(index) = to_remove {
            self.environment_vars.remove(index);
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("💾 Aplicar Cambios").clicked() {
                self.apply_environment_changes(service, project_path, sender, is_loading);
            }

            if ui.button("🔄 Recargar").clicked() {
                self.reload_environment_variables(service, project_path, sender, is_loading);
            }
        });
    }

    fn show_monitoring_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("📊 Monitoreo del Servidor");

        // Métricas básicas (placeholder)
        ui.columns(3, |columns| {
            columns[0].group(|ui| {
                ui.label("CPU Usage");
                ui.label("0%"); // Placeholder
            });

            columns[1].group(|ui| {
                ui.label("Memory Usage");
                ui.label("0 MB"); // Placeholder
            });

            columns[2].group(|ui| {
                ui.label("Active Connections");
                ui.label("0"); // Placeholder
            });
        });

        ui.separator();

        // Botones de monitoreo
        ui.horizontal(|ui| {
            if ui.button("📊 Server Status").clicked() {
                self.get_server_stats(service, project_path, sender, is_loading);
            }

            if ui.button("🔗 Active Connections").clicked() {
                self.get_active_connections(service, project_path, sender, is_loading);
            }

            if ui.button("⚡ Performance").clicked() {
                self.get_performance_metrics(service, project_path, sender, is_loading);
            }
        });
    }

    fn show_terminal_section(&mut self, ui: &mut egui::Ui, terminal: &mut TerminalBackend) {
        ui.collapsing("💻 Terminal del Servidor", |ui| {
            ui.label("Terminal integrado para comandos avanzados:");
            // Placeholder para el terminal
            ui.add_space(100.0);
        });
    }
    fn show_access_logs(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    fn show_error_logs(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    fn show_debug_logs(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    fn show_config_diff(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}


}