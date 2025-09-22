use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use eframe::egui;
use egui_term::TerminalBackend;

use crate::core::commands::*;
use crate::models::commands::LandoCommandOutcome;
use crate::models::lando::LandoService;

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub query: String,
    pub result: String,
    pub execution_time: f64,
    pub timestamp: u64,
    pub rows_affected: Option<i32>,
    pub has_error: bool,
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub row_count: Option<i64>,
    pub table_type: String, // table, view, etc.
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseTab {
    QueryEditor,
    SchemaExplorer,
    TableBrowser,
    Connections,
    QueryHistory,
    Tools,
}

pub struct DatabaseUI {
    // Query Editor
    pub query_input: String,
    pub query_results: Vec<QueryResult>,
    pub current_result_index: usize,
    pub query_history: Vec<String>,
    pub selected_history_index: Option<usize>,
    pub saved_queries: HashMap<String, String>,
    pub query_name_input: String,
    
    // Schema Browser
    pub tables: Vec<TableInfo>,
    pub selected_table: Option<String>,
    pub schema_filter: String,
    pub show_views: bool,
    pub show_procedures: bool,
    
    // Table Browser
    pub table_data: String,
    pub current_table: String,
    pub table_page: usize,
    pub table_limit: usize,
    pub table_sort_column: String,
    pub table_sort_desc: bool,
    pub table_filter: String,
    
    // Connection Management
    pub new_user: String,
    pub new_password: String,
    pub new_database: String,
    pub connection_status: ConnectionStatus,
    pub connection_test_result: String,
    
    // UI State
    pub current_tab: DatabaseTab,
    pub split_view: bool,
    pub auto_complete_enabled: bool,
    pub syntax_highlighting: bool,
    pub show_line_numbers: bool,
    pub show_save_query_dialog: bool,
    
    // Performance
    pub query_timeout: u32,
    pub max_rows: usize,
    pub enable_query_cache: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Testing,
    Error(String),
}

impl Default for DatabaseUI {
    fn default() -> Self {
        Self {
            // Query Editor
            query_input: String::new(),
            query_results: Vec::new(),
            current_result_index: 0,
            query_history: Vec::new(),
            selected_history_index: None,
            saved_queries: HashMap::new(),
            query_name_input: String::new(),
            
            // Schema Browser
            tables: Vec::new(),
            selected_table: None,
            schema_filter: String::new(),
            show_views: true,
            show_procedures: true,
            
            // Table Browser
            table_data: String::new(),
            current_table: String::new(),
            table_page: 0,
            table_limit: 50,
            table_sort_column: String::new(),
            table_sort_desc: false,
            table_filter: String::new(),
            
            // Connection Management
            new_user: String::new(),
            new_password: String::new(),
            new_database: String::new(),
            connection_status: ConnectionStatus::Disconnected,
            connection_test_result: String::new(),
            
            // UI State
            current_tab: DatabaseTab::QueryEditor,
            split_view: false,
            auto_complete_enabled: true,
            syntax_highlighting: true,
            show_line_numbers: true,
            show_save_query_dialog: false,
            
            // Performance
            query_timeout: 30,
            max_rows: 1000,
            enable_query_cache: true,
        }
    }
}

impl DatabaseUI {

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
        _terminal: &mut TerminalBackend,
    ) {
        // Botón prominente para abrir la interfaz de base de datos
        ui.horizontal(|ui| {
            ui.heading(format!("🗄️ {} ({})", service.service, service.r#type));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🚀 Abrir Interfaz de BD").clicked() {
                    self.current_tab = DatabaseTab::QueryEditor;
                }
            });
        });
        
        ui.separator();
        
        // Información básica del servicio
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("📊 Tipo: {}", service.r#type));
                ui.label(format!("🏷️ Versión: {}", service.version));
                
                if let Some(creds) = &service.creds {
                    if let Some(database) = &creds.database {
                        ui.label(format!("💾 DB: {}", database));
                    }
                }
                
                if let Some(conn) = &service.external_connection {
                    ui.label(format!("🌐 {}:{}", conn.host, conn.port));
                }
            });
        });
        
        ui.separator();
        
        // Controles rápidos
        ui.group(|ui| {
            ui.label("⚡ Acciones Rápidas:");
            ui.horizontal_wrapped(|ui| {
                if ui.button("📋 Ver Tablas").clicked() && !*is_loading {
                    self.current_tab = DatabaseTab::SchemaExplorer;
                    self.refresh_schema(service, project_path, sender, is_loading);
                }
                
                if ui.button("✏️ Editor SQL").clicked() {
                    self.current_tab = DatabaseTab::QueryEditor;
                }
                
                if ui.button("🔗 Test Conexión").clicked() && !*is_loading {
                    self.test_connection(service, project_path, sender, is_loading);
                }
                
                if ui.button("🔧 Herramientas").clicked() {
                    self.current_tab = DatabaseTab::Tools;
                }
            });
        });
        
        ui.separator();
        
        // Estado de conexión con botón de test
        ui.horizontal(|ui| {
            ui.label("🔗 Estado:");
            let (color, icon, text) = match &self.connection_status {
                ConnectionStatus::Connected => (egui::Color32::GREEN, "✅", "Conectado"),
                ConnectionStatus::Disconnected => (egui::Color32::RED, "❌", "Desconectado"),
                ConnectionStatus::Testing => (egui::Color32::YELLOW, "⏳", "Probando..."),
                ConnectionStatus::Error(err) => (egui::Color32::RED, "🚫", err.as_str()),
            };
            ui.colored_label(color, format!("{} {}", icon, text));
            
            ui.separator();
            
            if ui.button("🔍 Test Conexión").clicked() && !*is_loading {
                self.test_connection(service, project_path, sender, is_loading);
            }
        });
        
        ui.separator();
        
        // Interfaz completa de base de datos (siempre visible)
        ui.separator();
        ui.heading("🔧 Interfaz Completa de Base de Datos");
        
        // Navegación por pestañas
        self.show_tab_navigation(ui);
        
        ui.separator();
        
        // Diálogo para guardar query
        if self.show_save_query_dialog {
            self.show_save_query_dialog(ui);
        }
        
        // Contenido según la pestaña seleccionada
        match self.current_tab {
            DatabaseTab::QueryEditor => {
                if self.split_view {
                    self.show_split_query_editor(ui, service, project_path, sender, is_loading);
                } else {
                    self.show_query_editor(ui, service, project_path, sender, is_loading);
                }
            },
            DatabaseTab::SchemaExplorer => {
                self.show_schema_explorer(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::TableBrowser => {
                self.show_table_browser(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::Connections => {
                self.show_connection_manager(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::QueryHistory => {
                self.show_query_history_panel(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::Tools => {
                self.show_database_tools(ui, service, project_path, sender, is_loading);
            },
        }
    }

    pub fn show_full_interface(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
        terminal: &mut TerminalBackend,
    ) {
        // Navegación por pestañas
        self.show_tab_navigation(ui);
        
        ui.separator();
        
        // Contenido según la pestaña seleccionada
        match self.current_tab {
            DatabaseTab::QueryEditor => {
                if self.split_view {
                    self.show_split_query_editor(ui, service, project_path, sender, is_loading);
                } else {
                    self.show_query_editor(ui, service, project_path, sender, is_loading);
                }
            },
            DatabaseTab::SchemaExplorer => {
                self.show_schema_explorer(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::TableBrowser => {
                self.show_table_browser(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::Connections => {
                self.show_connection_manager(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::QueryHistory => {
                self.show_query_history_panel(ui, service, project_path, sender, is_loading);
            },
            DatabaseTab::Tools => {
                self.show_database_tools(ui, service, project_path, sender, is_loading);
            },
        }
    }

    fn show_database_header(&mut self, ui: &mut egui::Ui, service: &LandoService, is_loading: &bool) {
        ui.horizontal(|ui| {
            // Información básica
            ui.vertical(|ui| {
                ui.heading(format!("🗄️ {}", service.service));
                ui.label(format!("📊 Tipo: {}", service.r#type));
                ui.label(format!("🏷️ Versión: {}", service.version));
            });
            
            ui.separator();
            
            // Estado de conexión
            ui.vertical(|ui| {
                ui.label("🔗 Estado de Conexión:");
                let (color, icon, text) = match &self.connection_status {
                    ConnectionStatus::Connected => (egui::Color32::GREEN, "✅", "Conectado"),
                    ConnectionStatus::Disconnected => (egui::Color32::RED, "❌", "Desconectado"),
                    ConnectionStatus::Testing => (egui::Color32::YELLOW, "⏳", "Probando..."),
                    ConnectionStatus::Error(err) => (egui::Color32::RED, "🚫", err.as_str()),
                };
                ui.colored_label(color, format!("{} {}", icon, text));
                
                if let Some(conn) = &service.external_connection {
                    ui.label(format!("🌐 {}:{}", conn.host, conn.port));
                }
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Configuración rápida
                ui.checkbox(&mut self.split_view, "📱 Vista dividida");
                
                if *is_loading {
                    ui.spinner();
                    ui.label("Ejecutando...");
                }
            });
        });
    }
    
    fn show_tab_navigation(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.current_tab, DatabaseTab::QueryEditor, "✏️ Editor SQL");
            ui.selectable_value(&mut self.current_tab, DatabaseTab::SchemaExplorer, "🗂️ Schema");
            ui.selectable_value(&mut self.current_tab, DatabaseTab::TableBrowser, "📋 Tablas");
            ui.selectable_value(&mut self.current_tab, DatabaseTab::Connections, "🔗 Conexiones");
            ui.selectable_value(&mut self.current_tab, DatabaseTab::QueryHistory, "📜 Historial");
            ui.selectable_value(&mut self.current_tab, DatabaseTab::Tools, "🔧 Herramientas");
        });
    }
    
    fn show_query_editor(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        // Toolbar del editor con templates SQL
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("💻 Editor SQL:");
                ui.separator();
                
                // Templates SQL específicos por tipo de BD
                let templates = self.get_sql_templates(&service.r#type);
                let mut template_to_insert = None;
                for (name, sql) in templates {
                    if ui.small_button(name).clicked() {
                        template_to_insert = Some(sql.clone());
                    }
                }
                if let Some(template) = template_to_insert {
                    self.insert_template(&template);
                }
                
                ui.separator();
                
                // Herramientas del editor
                if ui.button("📝 Formato").on_hover_text("Formatear SQL (Ctrl+Shift+F)").clicked() {
                    self.format_query();
                }
                
                if ui.button("🗑️ Limpiar").on_hover_text("Limpiar editor (Ctrl+L)").clicked() {
                    self.query_input.clear();
                }
                
                if ui.button("💾 Guardar").on_hover_text("Guardar query (Ctrl+S)").clicked() {
                    self.show_save_query_dialog = true;
                }
            });
            
            // Segunda fila con configuración
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.syntax_highlighting, "🎨 Resaltado");
                ui.checkbox(&mut self.show_line_numbers, "🔢 Números");
                ui.checkbox(&mut self.auto_complete_enabled, "💡 Auto-completar");
                ui.separator();
                ui.checkbox(&mut self.split_view, "📱 Vista dividida");
            });
        });
        
        ui.separator();
        
        // Editor de consultas principal
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("📝 Query SQL:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Queries guardadas
                    if !self.saved_queries.is_empty() {
                        egui::ComboBox::new("saved_queries_combo", "💾 Guardadas")
                            .show_ui(ui, |ui| {
                                for (name, query) in &self.saved_queries {
                                    if ui.selectable_label(false, name).clicked() {
                                        self.query_input = query.clone();
                                    }
                                }
                            });
                    }
                    
                    // Historial de queries
                    if !self.query_history.is_empty() {
                        egui::ComboBox::new("history_combo", "📜 Historial")
                            .show_ui(ui, |ui| {
                                for (i, query) in self.query_history.iter().enumerate().rev().take(10) {
                                    let preview = if query.len() > 50 {
                                        format!("{}...", &query[..50])
                                    } else {
                                        query.clone()
                                    };
                                    if ui.selectable_label(false, preview).clicked() {
                                        self.query_input = query.clone();
                                    }
                                }
                            });
                    }
                });
            });
            
            let editor_rows = self.get_editor_rows();
            let text_edit = ui.add(
                egui::TextEdit::multiline(&mut self.query_input)
                    .hint_text("-- Escribe tu consulta SQL aquí\n-- Ejemplos:\nSELECT * FROM users LIMIT 10;\nSHOW TABLES;\nDESCRIBE table_name;")
                    .code_editor()
                    .desired_rows(editor_rows)
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
            );
            
            // Shortcuts de teclado mejorados
            if text_edit.has_focus() {
                ui.ctx().input(|i| {
                    // Ejecutar query
                    if i.key_pressed(egui::Key::F9) || (i.modifiers.ctrl && i.key_pressed(egui::Key::Enter)) {
                        self.execute_query(service, project_path, sender, is_loading);
                    }
                    // Formatear
                    if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::F) {
                        self.format_query();
                    }
                    // Limpiar
                    if i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
                        self.query_input.clear();
                    }
                    // Guardar
                    if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                        self.show_save_query_dialog = true;
                    }
                });
            }
            
            // Información del editor
            ui.horizontal(|ui| {
                let lines = self.query_input.lines().count();
                let chars = self.query_input.len();
                ui.small(format!("Líneas: {} | Caracteres: {}", lines, chars));
                
                if !self.query_input.is_empty() {
                    ui.separator();
                    if self.is_valid_sql(&self.query_input) {
                        ui.colored_label(egui::Color32::GREEN, "✓ SQL válido");
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, "⚠ Revisar sintaxis");
                    }
                }
            });
        });
        
        ui.separator();
        
        // Controles de ejecución mejorados
        ui.horizontal(|ui| {
            let can_execute = !*is_loading && !self.query_input.trim().is_empty();
            let execute_btn = ui.add_enabled(
                can_execute,
                egui::Button::new("▶️ Ejecutar Query")
                    .fill(if can_execute { egui::Color32::from_rgb(34, 139, 34) } else { egui::Color32::GRAY })
            );
            
            if execute_btn.clicked() {
                self.execute_query(service, project_path, sender, is_loading);
            }
            
            // Botones de acción rápida
            if ui.button("⏹️ Explicar").on_hover_text("EXPLAIN query").clicked() {
                self.explain_query(service, project_path, sender, is_loading);
            }
            
            ui.separator();
            
            // Configuración de ejecución
            ui.label("📋 Límite:");
            ui.add(egui::DragValue::new(&mut self.max_rows).range(1..=50000).speed(10));
            
            ui.label("⏰ Timeout:");
            ui.add(egui::DragValue::new(&mut self.query_timeout).range(5..=600).suffix("s"));
            
            if *is_loading {
                ui.separator();
                ui.spinner();
                ui.label("Ejecutando...");
            }
        });
        
        ui.separator();
        
        // Área de resultados mejorada
        self.show_query_results(ui);
    }
    
    fn show_query_results(&mut self, ui: &mut egui::Ui) {
        if !self.query_results.is_empty() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("📊 Resultados ({}):", self.query_results.len()));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("📋").on_hover_text("Copiar resultado").clicked() {
                            if let Some(result) = self.query_results.get(self.current_result_index) {
                                ui.ctx().copy_text(result.result.clone());
                            }
                        }
                        
                        if ui.small_button("💾").on_hover_text("Exportar a CSV").clicked() {
                            self.export_results_to_csv();
                        }
                        
                        if self.query_results.len() > 1 {
                            ui.separator();
                            if ui.small_button("◀️").clicked() && self.current_result_index > 0 {
                                self.current_result_index -= 1;
                            }
                            ui.label(format!("{}/{}", self.current_result_index + 1, self.query_results.len()));
                            if ui.small_button("▶️").clicked() && self.current_result_index < self.query_results.len() - 1 {
                                self.current_result_index += 1;
                            }
                        }
                    });
                });
                
                if let Some(result) = self.query_results.get(self.current_result_index) {
                    // Información de la consulta
                    ui.horizontal(|ui| {
                        ui.label(format!("⏱️ Tiempo: {:.2}ms", result.execution_time));
                        if let Some(rows) = result.rows_affected {
                            ui.label(format!("📋 Filas: {}", rows));
                        }
                        ui.label(format!("🗺️ {}", self.format_timestamp(result.timestamp)));
                        
                        if result.has_error {
                            ui.colored_label(egui::Color32::RED, "❌ Error");
                        } else {
                            ui.colored_label(egui::Color32::GREEN, "✅ Éxito");
                        }
                    });
                    
                    ui.separator();
                    
                    // Contenido del resultado
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut result.result.clone())
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .interactive(false)
                            );
                        });
                }
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("💭 No hay resultados aún");
                ui.label("Ejecuta una consulta para ver los resultados aquí");
                ui.add_space(50.0);
            });
        }
    }
    
    fn show_split_query_editor(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.columns(2, |columns| {
            // Panel izquierdo - Editor
            columns[0].vertical(|ui| {
                ui.strong("✏️ Editor SQL");
                ui.separator();
                
                // Controles del editor
                ui.horizontal_wrapped(|ui| {
                    if ui.button("📋 SELECT").clicked() {
                        self.insert_template("SELECT * FROM table_name LIMIT 10;");
                    }
                    if ui.button("🔍 DESCRIBE").clicked() {
                        self.insert_template(&self.get_describe_template(&service.r#type));
                    }
                    if ui.button("📊 COUNT").clicked() {
                        self.insert_template("SELECT COUNT(*) FROM table_name;");
                    }
                });
                
                ui.separator();
                
                // Editor principal
                ui.add(
                    egui::TextEdit::multiline(&mut self.query_input)
                        .hint_text("-- Tu consulta SQL")
                        .code_editor()
                        .desired_rows(15)
                        .desired_width(f32::INFINITY)
                );
                
                ui.horizontal(|ui| {
                    let execute_btn = ui.add_enabled(
                        !*is_loading && !self.query_input.trim().is_empty(),
                        egui::Button::new("▶️ Ejecutar")
                    );
                    
                    if execute_btn.clicked() {
                        self.execute_query(service, project_path, sender, is_loading);
                    }
                    
                    if ui.button("🗑️").clicked() {
                        self.query_input.clear();
                    }
                });
            });
            
            // Panel derecho - Resultados
            columns[1].vertical(|ui| {
                ui.strong("📊 Resultados");
                ui.separator();
                self.show_query_results(ui);
            });
        });
    }
    
    fn show_schema_explorer(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.heading("🗂️ Explorador de Schema");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Actualizar").clicked() && !*is_loading {
                    self.refresh_schema(service, project_path, sender, is_loading);
                }
            });
        });
        
        ui.separator();
        
        // Filtros
        ui.horizontal(|ui| {
            ui.label("🔍 Filtro:");
            ui.text_edit_singleline(&mut self.schema_filter);
            
            ui.separator();
            ui.checkbox(&mut self.show_views, "Vistas");
            ui.checkbox(&mut self.show_procedures, "Procedimientos");
        });
        
        ui.separator();
        
        // Lista de tablas
        egui::ScrollArea::vertical()
            .max_height(500.0)
            .show(ui, |ui| {
                if self.tables.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label("💭 No se han cargado tablas");
                        ui.label("Usa el botón 'Actualizar' para cargar el schema");
                        ui.add_space(50.0);
                    });
                } else {
                    for table in &self.tables.clone() {
                        if !self.schema_filter.is_empty() && !table.name.to_lowercase().contains(&self.schema_filter.to_lowercase()) {
                            continue;
                        }
                        
                        ui.collapsing(format!("📋 {}", table.name), |ui| {
                            ui.label(format!("Tipo: {}", table.table_type));
                            if let Some(count) = table.row_count {
                                ui.label(format!("Filas: {}", count));
                            }
                            
                            ui.separator();
                            ui.strong("Columnas:");
                            
                            for column in &table.columns {
                                ui.horizontal(|ui| {
                                    let icon = if column.is_primary_key { "🔑" } else { "📜" };
                                    ui.label(format!("{} {}", icon, column.name));
                                    ui.label(format!("({})", column.data_type));
                                    
                                    if !column.nullable {
                                        ui.colored_label(egui::Color32::RED, "NOT NULL");
                                    }
                                    
                                    if let Some(default) = &column.default_value {
                                        ui.label(format!("= {}", default));
                                    }
                                });
                            }
                            
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("📋 SELECT").clicked() {
                                    self.query_input = format!("SELECT * FROM {} LIMIT 10;", table.name);
                                    self.current_tab = DatabaseTab::QueryEditor;
                                }
                                if ui.button("🔍 DESCRIBE").clicked() {
                                    self.query_input = format!("DESCRIBE {};", table.name);
                                    self.current_tab = DatabaseTab::QueryEditor;
                                }
                                if ui.button("📊 COUNT").clicked() {
                                    self.query_input = format!("SELECT COUNT(*) FROM {};", table.name);
                                    self.current_tab = DatabaseTab::QueryEditor;
                                }
                            });
                        });
                    }
                }
            });
    }
    
    fn show_table_browser(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.heading("📋 Navegador de Tablas");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Actualizar").clicked() && !*is_loading {
                    self.refresh_schema(service, project_path, sender, is_loading);
                }
            });
        });
        
        ui.separator();
        
        // Selector de tabla
        ui.horizontal(|ui| {
            ui.label("📋 Tabla:");
            
            if self.tables.is_empty() {
                ui.label("No hay tablas cargadas");
                if ui.button("🔄 Cargar Tablas").clicked() && !*is_loading {
                    self.refresh_schema(service, project_path, sender, is_loading);
                }
            } else {
                egui::ComboBox::new("table_selector", self.current_table.as_str())
                    .show_ui(ui, |ui| {
                        let tables_clone = self.tables.clone();
                        for table in &tables_clone {
                            if ui.selectable_label(false, &table.name).clicked() {
                                self.current_table = table.name.clone();
                                self.table_page = 0;
                                self.table_filter.clear();
                                self.load_table_data(service, project_path, sender, is_loading);
                            }
                        }
                    });
            }
        });
        
        if !self.current_table.is_empty() {
            ui.separator();
            
            // Controles de navegación
            ui.horizontal(|ui| {
                ui.label("🔍 Filtro:");
                ui.text_edit_singleline(&mut self.table_filter);
                
                ui.separator();
                
                ui.label("📄 Filas por página:");
                ui.add(egui::DragValue::new(&mut self.table_limit).range(10..=1000).speed(10));
                
                ui.separator();
                
                if ui.button("🔄 Actualizar").clicked() && !*is_loading {
                    self.load_table_data(service, project_path, sender, is_loading);
                }
            });
            
            ui.separator();
            
            // Paginación
            ui.horizontal(|ui| {
                if ui.button("◀️ Anterior").clicked() && self.table_page > 0 {
                    self.table_page -= 1;
                    self.load_table_data(service, project_path, sender, is_loading);
                }
                
                ui.label(format!("Página {}", self.table_page + 1));
                
                if ui.button("▶️ Siguiente").clicked() {
                    self.table_page += 1;
                    self.load_table_data(service, project_path, sender, is_loading);
                }
                
                ui.separator();
                
                ui.label(format!("Límite: {}", self.table_limit));
            });
            
            ui.separator();
            
            // Datos de la tabla
            if *is_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Cargando datos de la tabla...");
                });
            } else if !self.table_data.is_empty() {
                egui::ScrollArea::both()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.table_data.clone())
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .interactive(false)
                        );
                    });
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("💭 No hay datos para mostrar");
                    ui.label("Selecciona una tabla y haz clic en 'Actualizar'");
                    ui.add_space(50.0);
                });
            }
        }
    }
    
    fn show_connection_manager(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("🔗 Gestor de Conexiones");
        
        // Información de conexión actual
        ui.group(|ui| {
            ui.strong("Conexión Actual:");
            
            if let Some(creds) = &service.creds {
                ui.horizontal(|ui| {
                    ui.label("👤 Usuario:");
                    ui.label(creds.user.as_ref().unwrap_or(&"N/A".to_string()));
                });
                
                if let Some(database) = &creds.database {
                    ui.horizontal(|ui| {
                        ui.label("💾 Base de datos:");
                        ui.label(database);
                    });
                }
            }
            
            if let Some(conn) = &service.external_connection {
                ui.horizontal(|ui| {
                    ui.label("🌐 Host:");
                    ui.label(format!("{}:{}", conn.host, conn.port));
                });
            }
        });
        
        ui.separator();
        
        // Nuevas credenciales
        ui.group(|ui| {
            ui.strong("Actualizar Credenciales:");
            
            ui.horizontal(|ui| {
                ui.label("👤 Usuario:");
                ui.text_edit_singleline(&mut self.new_user);
            });
            
            ui.horizontal(|ui| {
                ui.label("🔐 Contraseña:");
                ui.add(egui::TextEdit::singleline(&mut self.new_password).password(true));
            });
            
            ui.horizontal(|ui| {
                ui.label("💾 Base de datos:");
                ui.text_edit_singleline(&mut self.new_database);
            });
            
            ui.horizontal(|ui| {
                if ui.button("🔄 Test Connection").clicked() && !*is_loading {
                    self.test_connection(service, project_path, sender, is_loading);
                }
                
                if ui.button("💾 Aplicar Cambios").clicked() && !*is_loading {
                    self.update_credentials(service, project_path, sender, is_loading);
                }
            });
        });
        
        if !self.connection_test_result.is_empty() {
            ui.separator();
            ui.group(|ui| {
                ui.strong("Resultado del Test:");
                ui.label(&self.connection_test_result);
            });
        }
    }
    
    fn show_query_history_panel(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.heading("📜 Historial de Consultas");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} consultas", self.query_history.len()));
                
                if ui.button("🗑️ Limpiar").clicked() {
                    self.query_history.clear();
                    self.query_results.clear();
                }
            });
        });
        
        ui.separator();
        
        // Filtro de búsqueda
        ui.horizontal(|ui| {
            ui.label("🔍 Buscar:");
            ui.text_edit_singleline(&mut self.schema_filter); // Reutilizamos este campo para búsqueda
        });
        
        ui.separator();
        
        if self.query_history.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("💭 No hay consultas en el historial");
                ui.label("Las consultas ejecutadas aparecerán aquí");
                ui.add_space(50.0);
            });
        } else {
            let queries = self.query_history.clone(); // Clone para evitar borrowing issues
            let mut execute_query_request = None;
            let mut copy_text = None;
            let mut edit_query_request = None;
            
            // Filtrar queries si hay texto de búsqueda
            let filtered_queries: Vec<_> = if !self.schema_filter.is_empty() {
                queries.iter()
                    .filter(|query| query.to_lowercase().contains(&self.schema_filter.to_lowercase()))
                    .collect()
            } else {
                queries.iter().collect()
            };
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, query) in filtered_queries.iter().enumerate().rev() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", i + 1));
                            
                            let query_preview = if query.len() > 100 {
                                format!("{}...", &query[..100])
                            } else {
                                query.to_string()
                            };
                            
                            ui.label(query_preview);
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("▶️").on_hover_text("Ejecutar de nuevo").clicked() {
                                    execute_query_request = Some(query.to_string());
                                }
                                
                                if ui.small_button("📋").on_hover_text("Copiar").clicked() {
                                    copy_text = Some(query.to_string());
                                }
                                
                                if ui.small_button("✏️").on_hover_text("Editar").clicked() {
                                    edit_query_request = Some(query.to_string());
                                }
                                
                                if ui.small_button("💾").on_hover_text("Guardar").clicked() {
                                    self.query_input = query.to_string();
                                    self.show_save_query_dialog = true;
                                }
                            });
                        });
                    });
                    ui.add_space(5.0);
                }
            });
            
            // Procesar requests fuera del loop de borrowing
            if let Some(query) = execute_query_request {
                self.query_input = query.to_string();
                self.current_tab = DatabaseTab::QueryEditor;
                self.execute_query(service, project_path, sender, is_loading);
            }
            
            if let Some(text) = copy_text {
                ui.ctx().copy_text(text.to_string());
            }
            
            if let Some(query) = edit_query_request {
                self.query_input = query.to_string();
                self.current_tab = DatabaseTab::QueryEditor;
            }
        }
    }
    
    fn show_database_tools(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.heading("🔧 Herramientas de Base de Datos");
        
        // Herramientas de administración
        ui.group(|ui| {
            ui.strong("🛠️ Administración:");
            
            ui.horizontal_wrapped(|ui| {
                if ui.button("📊 Optimizar").clicked() && !*is_loading {
                    self.optimize_database(service, project_path, sender, is_loading);
                }
                
                if ui.button("📝 Backup").clicked() && !*is_loading {
                    self.backup_database(service, project_path, sender, is_loading);
                }
                
                if ui.button("🔄 Repair").clicked() && !*is_loading {
                    self.repair_database(service, project_path, sender, is_loading);
                }
                
                if ui.button("📊 Analyze").clicked() && !*is_loading {
                    self.analyze_database(service, project_path, sender, is_loading);
                }
            });
        });
        
        ui.separator();
        
        // Herramientas de desarrollo
        ui.group(|ui| {
            ui.strong("💻 Desarrollo:");
            
            ui.horizontal_wrapped(|ui| {
                if ui.button("📜 Generate Schema").clicked() {
                    self.generate_schema_documentation();
                }
                
                if ui.button("📦 Export Data").clicked() {
                    self.export_data();
                }
                
                if ui.button("📥 Import Data").clicked() {
                    self.import_data();
                }
            });
        });
        
        ui.separator();
        
        // Gestión de queries guardadas
        ui.group(|ui| {
            ui.strong("💾 Queries Guardadas:");
            
            if self.saved_queries.is_empty() {
                ui.label("No hay queries guardadas");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        let mut queries_to_remove = Vec::new();
                        
                        for (name, query) in &self.saved_queries {
                            ui.horizontal(|ui| {
                                ui.label(format!("📝 {}", name));
                                
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("▶️").on_hover_text("Ejecutar").clicked() {
                                        self.query_input = query.clone();
                                        self.current_tab = DatabaseTab::QueryEditor;
                                    }
                                    
                                    if ui.small_button("✏️").on_hover_text("Editar").clicked() {
                                        self.query_input = query.clone();
                                        self.current_tab = DatabaseTab::QueryEditor;
                                    }
                                    
                                    if ui.small_button("🗑️").on_hover_text("Eliminar").clicked() {
                                        queries_to_remove.push(name.clone());
                                    }
                                });
                            });
                        }
                        
                        // Eliminar queries marcadas para eliminación
                        for name in queries_to_remove {
                            self.saved_queries.remove(&name);
                        }
                    });
            }
        });
        
        ui.separator();
        
        // Configuración de rendimiento
        ui.group(|ui| {
            ui.strong("⚙️ Configuración:");
            
            ui.horizontal(|ui| {
                ui.label("Máx filas por consulta:");
                ui.add(egui::DragValue::new(&mut self.max_rows).range(1..=10000));
            });
            
            ui.horizontal(|ui| {
                ui.label("Timeout (segundos):");
                ui.add(egui::DragValue::new(&mut self.query_timeout).range(5..=300));
            });
            
            ui.checkbox(&mut self.enable_query_cache, "Habilitar caché de consultas");
        });
    }

    fn show_save_query_dialog(&mut self, ui: &mut egui::Ui) {
        let mut query_name = self.query_name_input.clone();
        let mut query_content = self.query_input.clone();
        let mut saved_queries_clone = self.saved_queries.clone();
        let mut should_close = false;
        
        egui::Window::new("💾 Guardar Query")
            .open(&mut self.show_save_query_dialog)
            .show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    ui.label("Nombre de la query:");
                    ui.text_edit_singleline(&mut query_name);
                    
                    ui.separator();
                    
                    ui.label("Query a guardar:");
                    ui.add(
                        egui::TextEdit::multiline(&mut query_content)
                            .code_editor()
                            .desired_rows(8)
                            .interactive(false)
                    );
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("💾 Guardar").clicked() {
                            if !query_name.is_empty() && !query_content.is_empty() {
                                saved_queries_clone.insert(query_name.clone(), query_content.clone());
                                query_name.clear();
                                should_close = true;
                            }
                        }
                        
                        if ui.button("❌ Cancelar").clicked() {
                            query_name.clear();
                            should_close = true;
                        }
                    });
                });
            });
        
        if should_close {
            self.show_save_query_dialog = false;
        }
        self.query_name_input = query_name;
        self.saved_queries = saved_queries_clone;
    }

}
