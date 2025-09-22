use crate::models::commands::LandoCommandOutcome;
use crate::models::lando::{LandoApp, LandoService};
use crate::ui::service::ServiceUIManager;
use egui_term::TerminalBackend;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct LandoGui {
    // Estado de la UI
    pub(crate) apps: Vec<LandoApp>,
    pub(crate) projects: Vec<PathBuf>,
    pub(crate) selected_project_path: Option<PathBuf>,
    pub(crate) services: Vec<LandoService>,
    pub(crate) db_query_input: String,
    pub(crate) db_query_result: Option<String>,
    pub(crate) shell_command_input: String,
    pub(crate) error_message: Option<String>,
    pub(crate) success_message: Option<String>,
    pub(crate) is_loading: Cell<bool>,

    pub(crate) sender: Sender<LandoCommandOutcome>,
    pub(crate) receiver: Receiver<LandoCommandOutcome>,

    // Terminal
    pub(crate) terminal: Rc<RefCell<TerminalBackend>>,
    pub(crate) show_terminal_popup: bool,
    pub(crate) terminal_filter: String,
    pub(crate) log_buffer: Vec<String>,

    // Gestor de UIs especializadas
    pub(crate) service_ui_manager: Rc<RefCell<ServiceUIManager>>,

    // Estado para controlar la interfaz de base de datos
    pub(crate) open_database_interface: Option<String>, // Nombre del servicio de BD abierto
}