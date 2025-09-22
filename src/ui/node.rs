use std::path::PathBuf;
use std::sync::mpsc::Sender;

use eframe::egui;
use egui_term::TerminalBackend;

use crate::models::commands::LandoCommandOutcome;
use crate::core::commands::*;
use crate::models::lando::LandoService;

pub struct NodeUI {
    pub command_input: String,
    pub command_history: Vec<String>,
    pub npm_command_input: String,
    pub package_name: String,
    pub package_version: String,
    pub script_name: String,
    pub installed_packages: Vec<PackageInfo>,
    pub available_scripts: Vec<String>,
    pub logs_output: String,
    pub debug_port: String,
    pub current_tab: NodeTab,
    pub node_version: String,
    pub npm_version: String,
    pub package_json_content: String,
    pub dependency_type: DependencyType,
    pub show_dev_dependencies: bool,
    pub show_global_packages: bool,
    pub environment_mode: EnvironmentMode,
    pub pm2_processes: Vec<PM2Process>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub is_dev_dependency: bool,
    pub is_outdated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PM2Process {
    pub name: String,
    pub id: u32,
    pub status: String,
    pub cpu: String,
    pub memory: String,
    pub uptime: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeTab {
    Scripts,
    Packages,
    Debug,
    Environment,
    PM2,
    Logs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DependencyType {
    Production,
    Development,
    Peer,
    Optional,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvironmentMode {
    Development,
    Production,
    Test,
    Custom(String),
}

impl Default for NodeUI {
    fn default() -> Self {
        Self {
            command_input: String::new(),
            command_history: Vec::new(),
            npm_command_input: String::new(),
            package_name: String::new(),
            package_version: String::new(),
            script_name: String::new(),
            installed_packages: Vec::new(),
            available_scripts: vec![
                "start".to_string(),
                "dev".to_string(),
                "build".to_string(),
                "test".to_string(),
                "lint".to_string(),
            ],
            logs_output: String::new(),
            debug_port: "9229".to_string(),
            current_tab: NodeTab::Scripts,
            node_version: "N/A".to_string(),
            npm_version: "N/A".to_string(),
            package_json_content: String::new(),
            dependency_type: DependencyType::Production,
            show_dev_dependencies: true,
            show_global_packages: false,
            environment_mode: EnvironmentMode::Development,
            pm2_processes: Vec::new(),
        }
    }
}

impl NodeUI {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
        terminal: &mut TerminalBackend,
    ) {
        ui.collapsing(format!("️ Node.js: {} ({})", service.service, service.r#type), |ui| {
            // Información del servicio
            self.show_service_header(ui, service);
            
            ui.separator();
            
            // Navegación por pestañas
            self.show_tab_navigation(ui);
            
            ui.separator();

            // Contenido según la pestaña seleccionada
            match self.current_tab {
                NodeTab::Scripts => {
                    self.show_scripts_panel(ui, service, project_path, sender, is_loading);
                }
                NodeTab::Packages => {
                    self.show_packages_panel(ui, service, project_path, sender, is_loading);
                }
                NodeTab::Debug => {
                    self.show_debug_panel(ui, service, project_path, sender, is_loading);
                }
                NodeTab::Environment => {
                    self.show_environment_panel(ui, service, project_path, sender, is_loading);
                }
                NodeTab::PM2 => {
                    self.show_pm2_panel(ui, service, project_path, sender, is_loading);
                }
                NodeTab::Logs => {
                    self.show_logs_panel(ui, service, project_path, sender, is_loading);
                }
            }

            // Terminal embebido
            self.show_terminal_section(ui, terminal);
        });
    }

    fn show_service_header(&mut self, ui: &mut egui::Ui, service: &LandoService) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(format!("🏷️ Tipo: {}", service.r#type));
                ui.label(format!("📦 Versión: {}", service.version));
                ui.label(format!("🟢 Node: {}", self.node_version));
                ui.label(format!("📦 NPM: {}", self.npm_version));
            });

            ui.separator();

            if let Some(conn) = &service.external_connection {
                ui.vertical(|ui| {
                    ui.label("Conexión Externa:");
                    ui.label(format!("🌐 {}:{}", conn.host, conn.port));
                });
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Actualizar Info").clicked() {
                    // Implementación pendiente
                }
            });
        });
    }

    fn show_tab_navigation(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.current_tab, NodeTab::Scripts, "🚀 Scripts");
            ui.selectable_value(&mut self.current_tab, NodeTab::Packages, "📦 Packages");
            ui.selectable_value(&mut self.current_tab, NodeTab::Debug, "🐛 Debug");
            ui.selectable_value(&mut self.current_tab, NodeTab::Environment, "🌍 Env");
            ui.selectable_value(&mut self.current_tab, NodeTab::PM2, "⚡ PM2");
            ui.selectable_value(&mut self.current_tab, NodeTab::Logs, "📜 Logs");
        });
    }

    fn show_scripts_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("🚀 Scripts de NPM");

        // Scripts predefinidos
        ui.group(|ui| {
            ui.label("Scripts Disponibles:");
            
            ui.horizontal_wrapped(|ui| {
                for script in &self.available_scripts.clone() {
                    let btn = ui.add_enabled(!*is_loading, egui::Button::new(format!("▶️ {}", script)));
                    if btn.clicked() {
                        self.run_npm_script(service, project_path, sender, is_loading, script);
                    }
                }
            });
        });

        ui.separator();

        // Script personalizado
        ui.group(|ui| {
            ui.label("Ejecutar Script Personalizado:");
            
            ui.horizontal(|ui| {
                ui.label("npm run");
                ui.text_edit_singleline(&mut self.script_name);
                
                let btn = ui.add_enabled(!*is_loading, egui::Button::new("▶️ Ejecutar"));
                if btn.clicked() {
                    let script_name = self.script_name.clone();
                    self.run_npm_script(service, project_path, sender, is_loading, &script_name);
                }
            });
        });

        ui.separator();

        // Comandos NPM comunes
        ui.group(|ui| {
            ui.label("Comandos NPM:");
            
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.npm_command_input);
                
                let btn = ui.add_enabled(!*is_loading, egui::Button::new("▶️ Ejecutar"));
                if btn.clicked() {
                    self.execute_npm_command(service, project_path, sender, is_loading);
                }
            });

            ui.horizontal_wrapped(|ui| {
                if ui.button("📋 npm list").clicked() {
                    self.npm_command_input = "list".to_string();
                }
                if ui.button("🔍 npm audit").clicked() {
                    self.npm_command_input = "audit".to_string();
                }
                if ui.button("🔧 npm doctor").clicked() {
                    self.npm_command_input = "doctor".to_string();
                }
                if ui.button("🧹 npm cache clean").clicked() {
                    self.npm_command_input = "cache clean --force".to_string();
                }
                if ui.button("🔄 npm update").clicked() {
                    self.npm_command_input = "update".to_string();
                }
            });
        });

        ui.separator();

        // Package.json viewer/editor
        ui.collapsing("📄 package.json", |ui| {
            if ui.button("🔄 Cargar package.json").clicked() {
                self.load_package_json(service, project_path, sender, is_loading);
            }

            ui.add(
                egui::TextEdit::multiline(&mut self.package_json_content)
                    .code_editor()
                    .desired_rows(10)
                    .desired_width(f32::INFINITY)
            );

            if ui.button("💾 Guardar package.json").clicked() {
                self.save_package_json(service, project_path, sender, is_loading);
            }
        });
    }

    fn show_packages_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("📦 Gestión de Paquetes");

        // Instalar nuevo paquete
        ui.group(|ui| {
            ui.label("Instalar Paquete:");
            
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.package_name);
                ui.label("@");
                ui.text_edit_singleline(&mut self.package_version);
                
                ui.label("Tipo:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", self.dependency_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.dependency_type, DependencyType::Production, "Production");
                        ui.selectable_value(&mut self.dependency_type, DependencyType::Development, "Development");
                        ui.selectable_value(&mut self.dependency_type, DependencyType::Peer, "Peer");
                        ui.selectable_value(&mut self.dependency_type, DependencyType::Optional, "Optional");
                    });
            });

            ui.horizontal(|ui| {
                let install_btn = ui.add_enabled(!*is_loading, egui::Button::new("📥 Instalar"));
                if install_btn.clicked() {
                    self.install_package(service, project_path, sender, is_loading);
                }
                
                if ui.button("🔍 Buscar en NPM").clicked() {
                    self.search_package(service, project_path, sender, is_loading);
                }
            });
        });

        ui.separator();

        // Filtros y opciones
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_dev_dependencies, "Mostrar dependencias de desarrollo");
            ui.checkbox(&mut self.show_global_packages, "Mostrar paquetes globales");
            
            if ui.button("🔄 Actualizar Lista").clicked() {
                self.refresh_packages_list(service, project_path, sender, is_loading);
            }
        });

        ui.separator();

        // Lista de paquetes instalados
        ui.group(|ui| {
            ui.label("Paquetes Instalados:");
            
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for package in &self.installed_packages.clone() {
                        ui.horizontal(|ui| {
                            let color = if package.is_outdated {
                                egui::Color32::YELLOW
                            } else if package.is_dev_dependency {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                egui::Color32::WHITE
                            };
                            
                            ui.colored_label(color, format!("📦 {}", package.name));
                            ui.label(format!("v{}", package.version));
                            
                            if let Some(desc) = &package.description {
                                ui.label(desc);
                            }
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("🗑️").on_hover_text("Desinstalar").clicked() {
                                    self.uninstall_package(service, project_path, sender, is_loading, &package.name);
                                }
                                
                                if package.is_outdated && ui.small_button("⬆️").on_hover_text("Actualizar").clicked() {
                                    self.update_package(service, project_path, sender, is_loading, &package.name);
                                }
                            });
                        });
                        ui.separator();
                    }
                });
        });
    }

    fn show_debug_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("🐛 Debugging de Node.js");

        // Configuración de debug
        ui.group(|ui| {
            ui.label("Configuración de Debug:");
            
            ui.horizontal(|ui| {
                ui.label("Puerto de debug:");
                ui.text_edit_singleline(&mut self.debug_port);
                
                ui.label("Modo:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", self.environment_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.environment_mode, EnvironmentMode::Development, "Development");
                        ui.selectable_value(&mut self.environment_mode, EnvironmentMode::Production, "Production");
                        ui.selectable_value(&mut self.environment_mode, EnvironmentMode::Test, "Test");
                    });
            });
        });

        ui.separator();

        // Comandos de debug
        ui.group(|ui| {
            ui.label("Comandos de Debug:");
            
            ui.horizontal(|ui| {
                let debug_btn = ui.add_enabled(!*is_loading, egui::Button::new("🐛 Iniciar Debug"));
                if debug_btn.clicked() {
                    self.start_debug_session(service, project_path, sender, is_loading);
                }
                
                if ui.button("🔍 Inspect").clicked() {
                    self.start_inspector(service, project_path, sender, is_loading);
                }
                
                if ui.button("📊 Profiling").clicked() {
                    self.start_profiling(service, project_path, sender, is_loading);
                }
            });
        });

        ui.separator();

        // Herramientas de desarrollo
        ui.group(|ui| {
            ui.label("Herramientas de Desarrollo:");
            
            ui.horizontal_wrapped(|ui| {
                if ui.button("🔧 ESLint").clicked() {
                    self.run_eslint(service, project_path, sender, is_loading);
                }
                
                if ui.button("🎨 Prettier").clicked() {
                    self.run_prettier(service, project_path, sender, is_loading);
                }
                
                if ui.button("🧪 Jest").clicked() {
                    self.run_tests(service, project_path, sender, is_loading);
                }
                
                if ui.button("📈 Coverage").clicked() {
                    self.run_coverage(service, project_path, sender, is_loading);
                }
            });
        });

        ui.separator();

        // Información de debug
        ui.collapsing("🔍 Información de Debug", |ui| {
            ui.label("URL de Inspector:");
            ui.code(format!("chrome://inspect/#devices"));
            ui.label(format!("Puerto: {}", self.debug_port));
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
        ui.heading("🌍 Variables de Entorno Node.js");

        // Modo de entorno
        ui.group(|ui| {
            ui.label("Modo de Entorno:");
            
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.environment_mode, EnvironmentMode::Development, "Development");
                ui.selectable_value(&mut self.environment_mode, EnvironmentMode::Production, "Production");
                ui.selectable_value(&mut self.environment_mode, EnvironmentMode::Test, "Test");
            });
        });

        ui.separator();

        // Variables de entorno específicas de Node.js
        ui.group(|ui| {
            ui.label("Variables Node.js:");
            
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.label("NODE_ENV:");
                    ui.label("NODE_PATH:");
                    ui.label("NODE_OPTIONS:");
                    ui.label("NPM_CONFIG_PREFIX:");
                });
                
                columns[1].vertical(|ui| {
                    ui.code(match self.environment_mode {
                        EnvironmentMode::Development => "development",
                        EnvironmentMode::Production => "production",
                        EnvironmentMode::Test => "test",
                        EnvironmentMode::Custom(ref s) => s,
                    });
                    ui.code("./node_modules");
                    ui.code("--max-old-space-size=4096");
                    ui.code("~/.npm-global");
                });
            });
        });

        ui.separator();

        // Configuración de NPM
        ui.group(|ui| {
            ui.label("Configuración NPM:");
            
            ui.horizontal(|ui| {
                if ui.button("📋 npm config list").clicked() {
                    self.show_npm_config(service, project_path, sender, is_loading);
                }
                
                if ui.button("🔧 npm config edit").clicked() {
                    self.edit_npm_config(service, project_path, sender, is_loading);
                }
            });
        });
    }

    fn show_pm2_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("⚡ Gestión PM2");

        // Controles PM2
        ui.horizontal(|ui| {
            if ui.button("🔄 Actualizar Lista").clicked() {
                self.refresh_pm2_processes(service, project_path, sender, is_loading);
            }
            
            if ui.button("🚀 PM2 Start").clicked() {
                self.pm2_start(service, project_path, sender, is_loading);
            }
            
            if ui.button("⏹️ PM2 Stop All").clicked() {
                self.pm2_stop_all(service, project_path, sender, is_loading);
            }
            
            if ui.button("🔄 PM2 Restart All").clicked() {
                self.pm2_restart_all(service, project_path, sender, is_loading);
            }
        });

        ui.separator();

        // Lista de procesos PM2
        if !self.pm2_processes.is_empty() {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for process in &self.pm2_processes.clone() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let status_color = match process.status.as_str() {
                                    "online" => egui::Color32::GREEN,
                                    "stopped" => egui::Color32::RED,
                                    "error" => egui::Color32::RED,
                                    _ => egui::Color32::GRAY,
                                };
                                
                                ui.colored_label(status_color, format!("● {}", process.name));
                                ui.label(format!("ID: {}", process.id));
                                ui.label(format!("CPU: {}", process.cpu));
                                ui.label(format!("Mem: {}", process.memory));
                                ui.label(format!("Uptime: {}", process.uptime));
                                
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("🗑️").clicked() {
                                        self.pm2_delete_process(service, project_path, sender, is_loading, &process.name);
                                    }
                                    if ui.small_button("⏹️").clicked() {
                                        self.pm2_stop_process(service, project_path, sender, is_loading, &process.name);
                                    }
                                    if ui.small_button("🔄").clicked() {
                                        self.pm2_restart_process(service, project_path, sender, is_loading, &process.name);
                                    }
                                });
                            });
                        });
                    }
                });
        } else {
            ui.label("No hay procesos PM2 ejecutándose");
        }
    }

    fn show_logs_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("📜 Logs de Node.js");

        // Controles de logs
        ui.horizontal(|ui| {
            if ui.button("🔄 Actualizar Logs").clicked() {
                self.refresh_logs(service, project_path, sender, is_loading);
            }
            
            if ui.button("📜 NPM Logs").clicked() {
                self.show_npm_logs(service, project_path, sender, is_loading);
            }
            
            if ui.button("⚡ PM2 Logs").clicked() {
                self.show_pm2_logs(service, project_path, sender, is_loading);
            }
            
            if ui.button("🗑️ Limpiar").clicked() {
                self.logs_output.clear();
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

    fn show_terminal_section(&mut self, ui: &mut egui::Ui, terminal: &mut TerminalBackend) {
        ui.collapsing("💻 Terminal Node.js", |ui| {
            ui.label("Terminal integrado para Node.js:");
            // Placeholder para el terminal
            ui.add_space(100.0);
        });
    }

    // Métodos auxiliares (implementaciones básicas - placeholders)
    fn refresh_node_info(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}

    
}