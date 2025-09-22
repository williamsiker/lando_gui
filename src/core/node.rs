use std::path::PathBuf;
use std::sync::mpsc::Sender;
use crate::models::commands::LandoCommandOutcome;
use crate::models::lando::LandoService;
use crate::core::commands::*;
use crate::ui::node::{DependencyType, NodeUI};

impl NodeUI {

    pub fn run_npm_script(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool, script: &str) {
        *is_loading = true;
        let command = format!("npm run {}", script);
        run_shell_command(
            sender.clone(),
            project_path.clone(),
            service.service.clone(),
            command,
        );
    }

    pub fn execute_npm_command(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if !self.npm_command_input.trim().is_empty() {
            *is_loading = true;
            let command = format!("npm {}", self.npm_command_input);
            run_shell_command(
                sender.clone(),
                project_path.clone(),
                service.service.clone(),
                command,
            );
        }
    }

    pub fn install_package(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if !self.package_name.trim().is_empty() {
            *is_loading = true;
            let version_part = if !self.package_version.is_empty() {
                format!("@{}", self.package_version)
            } else {
                String::new()
            };

            let flag = match self.dependency_type {
                DependencyType::Development => " --save-dev",
                DependencyType::Peer => " --save-peer",
                DependencyType::Optional => " --save-optional",
                _ => " --save",
            };

            let command = format!("npm install {}{}{}", self.package_name, version_part, flag);
            run_shell_command(
                sender.clone(),
                project_path.clone(),
                service.service.clone(),
                command,
            );
        }
    }

    // Implementaciones básicas para otros métodos (placeholders)
    pub fn load_package_json(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn save_package_json(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn search_package(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn refresh_packages_list(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn uninstall_package(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool, _package: &str) {}
    pub fn update_package(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool, _package: &str) {}
    pub fn start_debug_session(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn start_inspector(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn start_profiling(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn run_eslint(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn run_prettier(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn run_tests(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn run_coverage(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn show_npm_config(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn edit_npm_config(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn refresh_pm2_processes(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn pm2_start(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn pm2_stop_all(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn pm2_restart_all(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn pm2_delete_process(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool, _name: &str) {}
    pub fn pm2_stop_process(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool, _name: &str) {}
    pub fn pm2_restart_process(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool, _name: &str) {}
    pub fn refresh_logs(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn show_npm_logs(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}
    pub fn show_pm2_logs(&mut self, _service: &LandoService, _project_path: &PathBuf, _sender: &Sender<LandoCommandOutcome>, _is_loading: &mut bool) {}

}