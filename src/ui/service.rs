use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::collections::HashMap;

use eframe::egui;
use egui_term::TerminalBackend;

use crate::models::commands::LandoCommandOutcome;
use crate::models::lando::LandoService;
use crate::core::commands::*;
use crate::ui::database::DatabaseUI;
use crate::ui::appserver::AppServerUI;
use crate::ui::node::NodeUI;

// Gestor de estado para las diferentes UIs especializadas
pub struct ServiceUIManager {
    pub database_uis: HashMap<String, DatabaseUI>,
    pub appserver_uis: HashMap<String, AppServerUI>,
    pub node_uis: HashMap<String, NodeUI>,
}

impl Default for ServiceUIManager {
    fn default() -> Self {
        Self {
            database_uis: HashMap::new(),
            appserver_uis: HashMap::new(),
            node_uis: HashMap::new(),
        }
    }
}

impl ServiceUIManager {
    pub fn show_service_details(
        &mut self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
        terminal: &mut TerminalBackend,
    ) {
        let service_key = format!("{}_{}", service.service, service.r#type);
        
        // Determinar el tipo de servicio y mostrar la UI apropiada
        match self.classify_service(service) {
            ServiceType::Database => {
                let database_ui = self.database_uis
                    .entry(service_key)
                    .or_insert_with(DatabaseUI::default);
                
                database_ui.show(ui, service, project_path, sender, is_loading, terminal);
            },
            ServiceType::AppServer => {
                let appserver_ui = self.appserver_uis
                    .entry(service_key)
                    .or_insert_with(AppServerUI::default);
                
                appserver_ui.show(ui, service, project_path, sender, is_loading, terminal);
            },
            ServiceType::Node => {
                let node_ui = self.node_uis
                    .entry(service_key)
                    .or_insert_with(NodeUI::default);
                
                node_ui.show(ui, service, project_path, sender, is_loading, terminal);
            },
            ServiceType::Generic => {
                // Fallback a la UI genérica original para servicios no clasificados
                self.show_generic_service_ui(ui, service, project_path, sender, is_loading);
            },
        }
    }

    fn classify_service(&self, service: &LandoService) -> ServiceType {
        let service_type = service.r#type.to_lowercase();
        let service_name = service.service.to_lowercase();

        // Clasificar por nombre de servicio primero (más confiable)
        let result = if service_name == "database" {
            ServiceType::Database
        } else if self.is_database_service(&service_name) {
            ServiceType::Database
        } else if service_name == "appserver" {
            ServiceType::AppServer
        } else if self.is_appserver_service(&service_name) {
            ServiceType::AppServer
        } else if service_name == "node" {
            ServiceType::Node
        } else if self.is_node_service(&service_name) {
            ServiceType::Node
        } else {
            // Clasificar por tipo de servicio como fallback
            match service_type.as_str() {
                "database" => ServiceType::Database,
                "appserver" => ServiceType::AppServer,
                "node" => ServiceType::Node,
                _ => ServiceType::Generic
            }
        };
        
        result
    }

    pub fn is_database_service(&self, service_name: &str) -> bool {
        matches!(service_name, 
            "mysql" | "mariadb" | "postgres" | "postgresql" | 
            "mongodb" | "redis" | "sqlite" | "cassandra" | 
            "elasticsearch" | "memcached"
        )
    }

    fn is_appserver_service(&self, service_name: &str) -> bool {
        matches!(service_name, 
            "apache" | "nginx" | "httpd" | "php" | "python" | 
            "ruby" | "java" | "tomcat" | "jetty"
        )
    }

    fn is_node_service(&self, service_name: &str) -> bool {
        matches!(service_name, "node" | "nodejs" | "npm" | "yarn")
    }

    fn show_generic_service_ui(
        &self,
        ui: &mut egui::Ui,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        ui.collapsing(&service.service, |ui| {
            ui.label(format!("🏷️ Tipo: {}", service.r#type));
            ui.label(format!("📦 Versión: {}", service.version));

            if let Some(creds) = &service.creds {
                ui.separator();
                ui.strong("Credenciales:");
                if let Some(user) = &creds.user {
                    ui.label(format!("👤 Usuario: {}", user));
                }
                if let Some(password) = &creds.password {
                    ui.add(egui::Label::new(format!("🔐 Contraseña: {}", "••••••••")).sense(egui::Sense::click()))
                        .on_hover_text("Click para copiar");
                }
                if let Some(database) = &creds.database {
                    ui.label(format!("💾 Base de datos: {}", database));
                }
            }

            if let Some(conn) = &service.external_connection {
                ui.separator();
                ui.strong("🌐 Conexión Externa:");
                ui.label(format!("Host: {}", conn.host));
                ui.label(format!("Port: {}", conn.port));
            }

            ui.separator();
            ui.label("⚠️ Servicio genérico - Funcionalidad limitada");
            ui.label("Considera configurar una interfaz especializada para este tipo de servicio.");
            
            // Comando shell básico
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("📊 Status").clicked() && !*is_loading {
                    *is_loading = true;
                    run_shell_command(
                        sender.clone(),
                        project_path.clone(),
                        service.service.clone(),
                        "status".to_string(),
                    );
                }
                
                if ui.button("🔄 Restart").clicked() && !*is_loading {
                    *is_loading = true;
                    run_shell_command(
                        sender.clone(),
                        project_path.clone(),
                        service.service.clone(),
                        "restart".to_string(),
                    );
               }
            });
        });
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ServiceType {
    Database,
    AppServer,
    Node,
    Generic,
}
