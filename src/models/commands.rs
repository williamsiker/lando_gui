use crate::models::lando::{LandoApp, LandoService};
use std::path::PathBuf;

// Mensajes que los hilos de trabajo envían a la UI.
#[derive(Debug)]
pub enum LandoCommandOutcome {
    List(Vec<LandoApp>),
    Projects(Vec<PathBuf>),
    Info(Vec<LandoService>),
    DbQueryResult(String),
    Error(String),
    CommandSuccess(String),
    FinishedLoading, // Para indicar que una tarea en segundo plano ha terminado
    LogOutput(Vec<u8>), // Para enviar la salida del comando en tiempo real
}
