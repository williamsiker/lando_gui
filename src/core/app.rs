use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use egui_term::{BackendSettings, PtyEvent, TerminalBackend};
use crate::core::commands::list_apps;
use crate::models::app::LandoGui;
use crate::ui::service::ServiceUIManager;

impl LandoGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = mpsc::channel();

        // Channel for the terminal, as required by the constructor.
        // The receiver is not used because we don't process PTY events.
        let (pty_sender, _pty_receiver) = mpsc::channel::<(u64, PtyEvent)>();

        // Al iniciar, pedimos la lista de apps
        list_apps(sender.clone());

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
            is_loading: Cell::new(true), // Empezamos cargando
            sender,
            receiver,
            terminal: Rc::new(RefCell::new(
                TerminalBackend::new(
                    0,
                    cc.egui_ctx.clone(),
                    pty_sender,
                    BackendSettings::default(),
                )
                    .expect("Failed to create TerminalBackend"),
            )),
            service_ui_manager: Rc::new(RefCell::new(ServiceUIManager::default())),
            open_database_interface: None,
            show_terminal_popup: false,
            terminal_filter: String::new(),
            log_buffer: Vec::new(),
        }
    }
}