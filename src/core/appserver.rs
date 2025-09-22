use std::path::PathBuf;
use std::sync::mpsc::Sender;
use crate::core::commands::run_shell_command;
use crate::models::lando::LandoService;
use crate::ui::appserver::AppServerUI;
use crate::models::commands::LandoCommandOutcome;

impl AppServerUI {
    pub fn get_list_modules_command(&self, server_type: &str) -> String {
        match server_type.to_lowercase().as_str() {
            "apache" => "apache2ctl -M".to_string(),
            "nginx" => "nginx -V".to_string(),
            "php" => "php -m".to_string(),
            _ => "status".to_string(),
        }
    }
    pub fn execute_custom_command(
        &mut self,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        if !self.command_input.trim().is_empty() {
            *is_loading = true;
            self.command_history.push(self.command_input.clone());

            run_shell_command(
                sender.clone(),
                project_path.clone(),
                service.service.clone(),
                self.command_input.clone(),
            );
        }
    }

    pub fn refresh_service_status(&mut self) {}
    pub fn restart_service(&mut self) {}
    pub fn start_service(&mut self) {}
    pub fn stop_service(&mut self) {}
    pub fn restart_service_with_feedback(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn stop_service_with_feedback(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn start_service_with_feedback(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn reload_configuration(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn clear_cache(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn test_connection(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn refresh_logs(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn export_logs(&mut self) {}

    pub fn load_config_file(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn save_config_file(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn backup_config_file(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn validate_config(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn test_config(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn add_environment_variable(&mut self) {
        if !self.new_env_key.is_empty() && !self.new_env_value.is_empty() {
            self.environment_vars.push((self.new_env_key.clone(), self.new_env_value.clone()));
            self.new_env_key.clear();
            self.new_env_value.clear();
        }
    }
    pub fn apply_environment_changes(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn reload_environment_variables(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn get_server_stats(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn get_active_connections(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn get_performance_metrics(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
}