use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct LandoApp {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub urls: Vec<String>,
    pub running: bool,
}

// Representa un servicio individual de Lando (ej. appserver, database)
#[derive(Deserialize, Clone, Debug, Default)]
pub struct LandoService {
    pub service: String,
    pub r#type: String,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub internal_connection: Option<ServiceConnectionInfo>,
    #[serde(default)]
    pub external_connection: Option<ServiceConnectionInfo>,
    #[serde(default)]
    pub creds: Option<ServiceCreds>,
}

// Información de conexión para un servicio
#[derive(Deserialize, Clone, Debug, Default)]
pub struct ServiceConnectionInfo {
    pub host: String,
    pub port: String,
}

// Credenciales para un servicio
#[derive(Deserialize, Clone, Debug, Default)]
pub struct ServiceCreds {
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
}
