use crate::models::{LandoApp, LandoService};
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;
use walkdir::WalkDir;

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

// Lanza un comando `lando list` en un hilo separado.
pub fn list_apps(sender: Sender<LandoCommandOutcome>) {
    thread::spawn(move || {
        let output = Command::new("lando")
            .args(["list", "--format", "json"])
            .output();

        let outcome = match output {
            Ok(output) => {
                if output.status.success() {
                    match serde_json::from_slice::<Vec<LandoApp>>(&output.stdout) {
                        Ok(apps) => LandoCommandOutcome::List(apps),
                        Err(e) => LandoCommandOutcome::Error(format!("Error al parsear JSON: {}", e)),
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    LandoCommandOutcome::Error(format!("Error de Lando: {}", stderr))
                }
            }
            Err(e) => LandoCommandOutcome::Error(format!("No se pudo ejecutar Lando: {}", e)),
        };

        let _ = sender.send(outcome);
    });
}

// Escanea un directorio en busca de proyectos Lando (`.lando.yml`)
pub fn scan_for_projects(sender: Sender<LandoCommandOutcome>, path_to_scan: PathBuf) {
    thread::spawn(move || {
        let mut projects = vec![];
        // Limita la profundidad para no tardar demasiado
        let walker = WalkDir::new(path_to_scan).max_depth(3);

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() == ".lando.yml" {
                if let Some(parent) = entry.path().parent() {
                    projects.push(parent.to_path_buf());
                }
            }
        }

        let _ = sender.send(LandoCommandOutcome::Projects(projects));
    });
}

// Ejecuta un comando de lando en el directorio de un proyecto y transmite la salida.
pub fn run_lando_command(sender: Sender<LandoCommandOutcome>, command: String, project_path: PathBuf) {
    thread::spawn(move || {
        let mut child = match Command::new("lando")
            .arg(command.clone())
            .current_dir(project_path.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                let _ = sender.send(LandoCommandOutcome::Error(format!(
                    "No se pudo ejecutar Lando: {}",
                    e
                )));
                return;
            }
        };

        // Hilo para leer stdout
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let sender_stdout = sender.clone();
        let stdout_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut buffer = [0; 1024];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 { break; }
                let _ = sender_stdout.send(LandoCommandOutcome::LogOutput(buffer[..n].to_vec()));
            }
        });

        // Hilo para leer stderr
        let stderr = child.stderr.take().expect("Failed to open stderr");
        let sender_stderr = sender.clone();
        let stderr_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut buffer = [0; 1024];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 { break; }
                let _ = sender_stderr.send(LandoCommandOutcome::LogOutput(buffer[..n].to_vec()));
            }
        });

        // Esperar a que los hilos de lectura terminen
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        // Esperar a que el comando termine y enviar el estado final
        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => {
                let _ = sender.send(LandoCommandOutcome::Error(format!(
                    "Error esperando el comando '{}': {}",
                    command, e
                )));
                return;
            }
        };

        let outcome = if status.success() {
            LandoCommandOutcome::CommandSuccess(format!(
                "Comando '{}' finalizado con éxito.",
                command
            ))
        } else {
            LandoCommandOutcome::Error(format!(
                "El comando '{}' terminó con un error.",
                command
            ))
        };

        let _ = sender.send(outcome);
    });
}

pub fn get_project_info(sender: Sender<LandoCommandOutcome>, project_path: PathBuf) {
    thread::spawn(move || {
        let output = Command::new("lando")
            .args(["info", "--format", "json"])
            .current_dir(project_path)
            .output();

        let outcome = match output {
            Ok(output) => {
                if output.status.success() {
                    match serde_json::from_slice::<Vec<LandoService>>(&output.stdout) {
                        Ok(services) => LandoCommandOutcome::Info(services),
                        Err(e) => LandoCommandOutcome::Error(format!("Error al parsear JSON de lando info: {}", e)),
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    LandoCommandOutcome::Error(format!("Error de Lando info: {}", stderr))
                }
            }
            Err(e) => LandoCommandOutcome::Error(format!("No se pudo ejecutar Lando info: {}", e)),
        };

        let _ = sender.send(outcome);
    });
}

pub fn run_db_query(sender: Sender<LandoCommandOutcome>, project_path: PathBuf, service: String, query: String) {
    thread::spawn(move || {
        // Intentar primero con credenciales por defecto (root sin contraseña)
        let output = Command::new("lando")
            .args(["db-cli", "-s", &service, "-u", "root", "-e", &query])
            .current_dir(project_path.clone())
            .output();

        let outcome = match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    LandoCommandOutcome::DbQueryResult(stdout)
                } else {
                    // Si falla con root, intentar sin especificar usuario
                    let output2 = Command::new("lando")
                        .args(["db-cli", "-s", &service, "-e", &query])
                        .current_dir(project_path)
                        .output();
                    
                    match output2 {
                        Ok(output2) => {
                            if output2.status.success() {
                                let stdout = String::from_utf8_lossy(&output2.stdout).to_string();
                                LandoCommandOutcome::DbQueryResult(stdout)
                            } else {
                                let stderr = String::from_utf8_lossy(&output2.stderr).to_string();
                                LandoCommandOutcome::Error(format!("Error ejecutando la consulta: {}", stderr))
                            }
                        }
                        Err(e) => LandoCommandOutcome::Error(format!("No se pudo ejecutar lando db-cli: {}", e)),
                    }
                }
            }
            Err(e) => LandoCommandOutcome::Error(format!("No se pudo ejecutar lando db-cli: {}", e)),
        };

        let _ = sender.send(outcome);
    });
}

pub fn test_db_connection(
    sender: Sender<LandoCommandOutcome>,
    project_path: PathBuf,
    service: String,
) {
    thread::spawn(move || {
        // Usar mysqladmin para verificar si el servidor está vivo
        let test_command = "mysqladmin -u root ping";

        let output = Command::new("lando")
            .args(["ssh", "-s", &service, "-c", test_command])
            .current_dir(project_path)
            .output();

        let outcome = match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    if stdout.contains("alive") {
                        LandoCommandOutcome::DbQueryResult("✅ Conexión exitosa".to_string())
                    } else {
                        LandoCommandOutcome::Error(format!(
                            "Error de conexión (salida inesperada): {}",
                            stdout
                        ))
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    LandoCommandOutcome::Error(format!("Error probando conexión: {}", stderr))
                }
            }
            Err(e) => LandoCommandOutcome::Error(format!(
                "No se pudo ejecutar test de conexión: {}",
                e
            )),
        };

        let _ = sender.send(outcome);
    });
}

pub fn run_shell_command(sender: Sender<LandoCommandOutcome>, project_path: PathBuf, service: String, command: String) {
    thread::spawn(move || {
        let mut child = match Command::new("lando")
            .args(["ssh", "-s", &service, "-c", &command])
            .current_dir(project_path.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                let _ = sender.send(LandoCommandOutcome::Error(format!(
                    "No se pudo ejecutar Lando ssh: {}",
                    e
                )));
                return;
            }
        };

        // Hilo para leer stdout
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let sender_stdout = sender.clone();
        let stdout_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut buffer = [0; 1024];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 { break; }
                let _ = sender_stdout.send(LandoCommandOutcome::LogOutput(buffer[..n].to_vec()));
            }
        });

        // Hilo para leer stderr
        let stderr = child.stderr.take().expect("Failed to open stderr");
        let sender_stderr = sender.clone();
        let stderr_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut buffer = [0; 1024];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 { break; }
                let _ = sender_stderr.send(LandoCommandOutcome::LogOutput(buffer[..n].to_vec()));
            }
        });

        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => {
                let _ = sender.send(LandoCommandOutcome::Error(format!(
                    "Error esperando el comando ssh '{}': {}",
                    command, e
                )));
                return;
            }
        };

        let outcome = if status.success() {
            LandoCommandOutcome::CommandSuccess(format!(
                "Comando shell '{}' finalizado con éxito.",
                command
            ))
        } else {
            LandoCommandOutcome::Error(format!(
                "El comando shell '{}' terminó con un error.",
                command
            ))
        };

        let _ = sender.send(outcome);
    });
}
