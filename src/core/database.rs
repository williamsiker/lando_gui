use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::commands::LandoCommandOutcome;
use crate::core::commands::*;
use crate::models::lando::LandoService;
use crate::ui::database::{ConnectionStatus, DatabaseUI, QueryResult, TableInfo};

impl DatabaseUI {
    pub fn update_query_result(&mut self, result_text: String, has_error: bool) {
        let rows_affected = self.extract_rows_affected(&result_text);
        let execution_time = if let Some(last_result) = self.query_results.last_mut() {
            let start_time = last_result.timestamp;
            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let exec_time = (current_time - start_time) as f64 * 1000.0; // en ms

            last_result.result = result_text.clone();
            last_result.execution_time = exec_time;
            last_result.has_error = has_error;
            last_result.rows_affected = rows_affected;

            exec_time
        } else {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let result = QueryResult {
                query: self.query_input.clone(),
                result: result_text.clone(),
                execution_time: 0.0,
                timestamp,
                rows_affected: self.extract_rows_affected(&result_text),
                has_error,
            };
            self.query_results.push(result);
            self.current_result_index = self.query_results.len() - 1;
            0.0
        };

        // Limitar el número de resultados guardados
        if self.query_results.len() > 20 {
            self.query_results.remove(0);
            if self.current_result_index > 0 {
                self.current_result_index -= 1;
            }
        }
    }

    pub fn extract_rows_affected(&self, result: &str) -> Option<i32> {
        if result.contains("row") {
            for line in result.lines() {
                if let Some(num_str) = line.split_whitespace().next() {
                    if let Ok(num) = num_str.parse::<i32>() {
                        return Some(num);
                    }
                }
            }
        }
        None
    }

    // Métodos auxiliares mejorados
    pub fn insert_template(&mut self, template: &str) {
        if !self.query_input.is_empty() {
            self.query_input.push_str("\n\n");
        }
        self.query_input.push_str(template);
    }

    pub fn get_sql_templates(&self, db_type: &str) -> Vec<(&str, String)> {
        let mut templates = vec![
            ("📋 SELECT", "SELECT * FROM table_name LIMIT 10;".to_string()),
            ("🔍 COUNT", "SELECT COUNT(*) FROM table_name;".to_string()),
            ("📊 TABLES", self.get_show_tables_query(db_type)),
            ("🏗️ DESCRIBE", self.get_describe_template(db_type)),
            ("🔍 WHERE", "SELECT * FROM table_name WHERE column = 'value';".to_string()),
            ("📈 ORDER BY", "SELECT * FROM table_name ORDER BY column DESC;".to_string()),
            ("📊 GROUP BY", "SELECT column, COUNT(*) FROM table_name GROUP BY column;".to_string()),
            ("🔗 JOIN", "SELECT * FROM table1 t1 JOIN table2 t2 ON t1.id = t2.table1_id;".to_string()),
        ];

        // Templates específicos por tipo de BD
        match db_type.to_lowercase().as_str() {
            "mysql" | "mariadb" => {
                templates.extend(vec![
                    ("📈 STATUS", "SHOW STATUS;".to_string()),
                    ("🔧 PROCESSES", "SHOW PROCESSLIST;".to_string()),
                    ("💾 DATABASES", "SHOW DATABASES;".to_string()),
                    ("🔍 INDEX", "SHOW INDEX FROM table_name;".to_string()),
                    ("📊 VARIABLES", "SHOW VARIABLES LIKE '%buffer%';".to_string()),
                    ("🔧 ENGINES", "SHOW ENGINES;".to_string()),
                    ("📈 PERFORMANCE", "SELECT * FROM performance_schema.events_statements_summary_by_digest LIMIT 10;".to_string()),
                    ("🔍 USERS", "SELECT User, Host FROM mysql.user;".to_string()),
                    ("📊 TABLES STATUS", "SHOW TABLE STATUS;".to_string()),
                    ("🔧 CREATE TABLE", "CREATE TABLE example_table (\n    id INT AUTO_INCREMENT PRIMARY KEY,\n    name VARCHAR(255) NOT NULL,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);".to_string()),
                ]);
            },
            "postgresql" | "postgres" => {
                templates.extend(vec![
                    ("📈 STATS", "SELECT * FROM pg_stat_database;".to_string()),
                    ("🔧 ACTIVITY", "SELECT * FROM pg_stat_activity;".to_string()),
                    ("💾 SCHEMAS", "SELECT schema_name FROM information_schema.schemata;".to_string()),
                    ("🔍 INDEXES", "SELECT * FROM pg_indexes WHERE tablename = 'table_name';".to_string()),
                    ("📊 TABLES INFO", "SELECT schemaname, tablename, tableowner FROM pg_tables;".to_string()),
                    ("🔧 LOCKS", "SELECT * FROM pg_locks;".to_string()),
                    ("📈 QUERY STATS", "SELECT query, calls, total_time FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10;".to_string()),
                    ("🔍 USERS", "SELECT usename, usesuper FROM pg_user;".to_string()),
                    ("📊 SIZE", "SELECT pg_size_pretty(pg_total_relation_size('table_name'));".to_string()),
                    ("🔧 CREATE TABLE", "CREATE TABLE example_table (\n    id SERIAL PRIMARY KEY,\n    name VARCHAR(255) NOT NULL,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);".to_string()),
                ]);
            },
            "sqlite" => {
                templates.extend(vec![
                    ("📈 PRAGMA", "PRAGMA database_list;".to_string()),
                    ("🔧 INFO", "PRAGMA table_info(table_name);".to_string()),
                    ("🔍 INDEX", "PRAGMA index_list(table_name);".to_string()),
                    ("📊 SCHEMA", "SELECT sql FROM sqlite_master WHERE type='table';".to_string()),
                    ("🔧 VERSION", "SELECT sqlite_version();".to_string()),
                    ("📈 STATS", "PRAGMA stats;".to_string()),
                    ("🔍 FOREIGN KEYS", "PRAGMA foreign_key_list(table_name);".to_string()),
                    ("📊 SIZE", "PRAGMA page_count; PRAGMA page_size;".to_string()),
                    ("🔧 CREATE TABLE", "CREATE TABLE example_table (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    name TEXT NOT NULL,\n    created_at DATETIME DEFAULT CURRENT_TIMESTAMP\n);".to_string()),
                ]);
            },
            _ => {
                // Templates genéricos para otros tipos de BD
                templates.extend(vec![
                    ("📊 INFO", "SELECT * FROM information_schema.tables;".to_string()),
                    ("🔍 COLUMNS", "SELECT * FROM information_schema.columns WHERE table_name = 'table_name';".to_string()),
                    ("📈 STATS", "SELECT * FROM information_schema.table_statistics;".to_string()),
                ]);
            }
        }

        templates
    }

    pub fn get_editor_rows(&self) -> usize {
        if self.split_view { 8 } else { 12 }
    }

    pub fn is_valid_sql(&self, sql: &str) -> bool {
        let sql = sql.trim().to_lowercase();
        if sql.is_empty() { return false; }

        // Validación básica de SQL
        let sql_keywords = ["select", "insert", "update", "delete", "show", "describe", "explain", "pragma", "create", "drop", "alter"];
        sql_keywords.iter().any(|&keyword| sql.starts_with(keyword))
    }

    pub fn explain_query(
        &mut self,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        if !self.query_input.trim().is_empty() {
            let explain_query = format!("EXPLAIN {}", self.query_input.trim());
            let original_query = self.query_input.clone();
            self.query_input = explain_query;
            self.execute_query(service, project_path, sender, is_loading);
            self.query_input = original_query; // Restaurar query original
        }
    }

    pub fn get_show_tables_query(&self, db_type: &str) -> String {
        match db_type.to_lowercase().as_str() {
            "mysql" | "mariadb" => "SHOW TABLES;".to_string(),
            "postgresql" | "postgres" => "SELECT tablename FROM pg_tables WHERE schemaname = 'public';".to_string(),
            "sqlite" => "SELECT name FROM sqlite_master WHERE type='table';".to_string(),
            _ => "SHOW TABLES;".to_string(),
        }
    }

    pub fn format_query(&mut self) {
        // Formateo básico de SQL
        self.query_input = self.query_input
            .replace(",", ",\n    ")
            .replace(" FROM ", "\nFROM ")
            .replace(" WHERE ", "\nWHERE ")
            .replace(" ORDER BY ", "\nORDER BY ")
            .replace(" GROUP BY ", "\nGROUP BY ");
    }

    pub fn get_describe_template(&self, db_type: &str) -> String {
        match db_type.to_lowercase().as_str() {
            "mysql" | "mariadb" => "DESCRIBE table_name;".to_string(),
            "postgresql" | "postgres" => "\\d table_name".to_string(),
            "sqlite" => "PRAGMA table_info(table_name);".to_string(),
            _ => "DESCRIBE table_name;".to_string(),
        }
    }

    pub fn format_timestamp(&self, timestamp: u64) -> String {
        let datetime = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(timestamp);
        // Formateo básico - en una implementación real usarías chrono
        format!("{:?}", datetime)
    }

    pub fn execute_query(
        &mut self,
        service: &LandoService,
        project_path: &PathBuf,
        sender: &Sender<LandoCommandOutcome>,
        is_loading: &mut bool,
    ) {
        if !self.query_input.trim().is_empty() {
            *is_loading = true;

            // Agregar al historial si no existe
            if !self.query_history.contains(&self.query_input) {
                self.query_history.push(self.query_input.clone());
                // Mantener solo los últimos 50 queries
                if self.query_history.len() > 50 {
                    self.query_history.remove(0);
                }
            }

            // Crear resultado placeholder
            let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let result = QueryResult {
                query: self.query_input.clone(),
                result: "Ejecutando consulta...".to_string(),
                execution_time: 0.0,
                timestamp: start_time,
                rows_affected: None,
                has_error: false,
            };

            self.query_results.push(result);
            self.current_result_index = self.query_results.len() - 1;

            run_db_query(
                sender.clone(),
                project_path.clone(),
                service.service.clone(),
                self.query_input.clone(),
            );
        }
    }

    // Placeholder methods - implementar según necesidades
    pub fn export_results_to_csv(&self) {
        if let Some(result) = self.query_results.get(self.current_result_index) {
            // En una implementación real, aquí se implementaría la exportación a CSV
            // Por ahora, simplemente copiamos el resultado al portapapeles
            println!("Exportando resultado a CSV: {}", result.result);
        }
    }
    pub fn refresh_schema(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading { return; }

        *is_loading = true;

        // Crear placeholder para el resultado
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let result = QueryResult {
            query: "Schema refresh".to_string(),
            result: "Cargando schema...".to_string(),
            execution_time: 0.0,
            timestamp: start_time,
            rows_affected: None,
            has_error: false,
        };
        self.query_results.push(result);
        self.current_result_index = self.query_results.len() - 1;

        // Ejecutar comando para obtener tablas
        let tables_query = self.get_show_tables_query(&service.r#type);
        run_db_query(
            sender.clone(),
            project_path.clone(),
            service.service.clone(),
            tables_query,
        );
    }
    pub fn load_table_data(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading || self.current_table.is_empty() { return; }

        *is_loading = true;

        // Crear query con paginación y filtros
        let mut query = format!("SELECT * FROM {}", self.current_table);

        if !self.table_filter.is_empty() {
            // Filtro básico - en una implementación real se haría más sofisticado
            query.push_str(&format!(" WHERE {}", self.table_filter));
        }

        query.push_str(&format!(" LIMIT {} OFFSET {}", self.table_limit, self.table_page * self.table_limit));

        // Crear placeholder para el resultado
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let result = QueryResult {
            query: query.clone(),
            result: "Cargando datos de la tabla...".to_string(),
            execution_time: 0.0,
            timestamp: start_time,
            rows_affected: None,
            has_error: false,
        };
        self.query_results.push(result);
        self.current_result_index = self.query_results.len() - 1;

        run_db_query(
            sender.clone(),
            project_path.clone(),
            service.service.clone(),
            query,
        );
    }

    pub fn test_connection(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading { return; }

        *is_loading = true;
        self.connection_status = ConnectionStatus::Testing;

        println!("🔍 Probando conexión a BD usando lando ssh...");

        // Usar la nueva función de test de conexión que usa lando ssh
        test_db_connection(
            sender.clone(),
            project_path.clone(),
            service.service.clone(),
        );
    }

    pub fn update_credentials(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading { return; }

        *is_loading = true;

        // Comando para actualizar credenciales usando lando
        let command = format!("config --set database.creds.user={} --set database.creds.password={} --set database.creds.database={}",
                              self.new_user, self.new_password, self.new_database);

        run_lando_command(
            sender.clone(),
            command,
            project_path.clone(),
        );
    }
    pub fn optimize_database(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading { return; }

        *is_loading = true;

        let optimize_query = match service.r#type.to_lowercase().as_str() {
            "mysql" | "mariadb" => "OPTIMIZE TABLE;",
            "postgresql" | "postgres" => "VACUUM ANALYZE;",
            "sqlite" => "VACUUM;",
            _ => "OPTIMIZE TABLE;",
        };

        run_db_query(
            sender.clone(),
            project_path.clone(),
            service.service.clone(),
            optimize_query.to_string(),
        );
    }

    pub fn backup_database(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading { return; }

        *is_loading = true;

        // Comando de backup usando lando
        let backup_command = match service.r#type.to_lowercase().as_str() {
            "mysql" | "mariadb" => format!("db-export -s {}", service.service),
            "postgresql" | "postgres" => format!("db-export -s {}", service.service),
            "sqlite" => format!("db-export -s {}", service.service),
            _ => format!("db-export -s {}", service.service),
        };

        run_lando_command(
            sender.clone(),
            backup_command,
            project_path.clone(),
        );
    }

    pub fn repair_database(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading { return; }

        *is_loading = true;

        let repair_query = match service.r#type.to_lowercase().as_str() {
            "mysql" | "mariadb" => "REPAIR TABLE;",
            "postgresql" | "postgres" => "REINDEX DATABASE;",
            "sqlite" => "REINDEX;",
            _ => "REPAIR TABLE;",
        };

        run_db_query(
            sender.clone(),
            project_path.clone(),
            service.service.clone(),
            repair_query.to_string(),
        );
    }

    pub fn analyze_database(&mut self, service: &LandoService, project_path: &PathBuf, sender: &Sender<LandoCommandOutcome>, is_loading: &mut bool) {
        if *is_loading { return; }

        *is_loading = true;

        let analyze_query = match service.r#type.to_lowercase().as_str() {
            "mysql" | "mariadb" => "ANALYZE TABLE;",
            "postgresql" | "postgres" => "ANALYZE;",
            "sqlite" => "ANALYZE;",
            _ => "ANALYZE TABLE;",
        };

        run_db_query(
            sender.clone(),
            project_path.clone(),
            service.service.clone(),
            analyze_query.to_string(),
        );
    }
    pub fn generate_schema_documentation(&self) {
        // Generar documentación del schema
        println!("Generando documentación del schema...");
    }

    pub fn export_data(&self) {
        // Exportar datos de la base de datos
        println!("Exportando datos...");
    }

    pub fn import_data(&self) {
        // Importar datos a la base de datos
        println!("Importando datos...");
    }

    // Método para procesar resultados de queries y actualizar el estado
    pub fn process_query_result(&mut self, result_text: String, has_error: bool) {
        // Actualizar el último resultado
        self.update_query_result(result_text.clone(), has_error);

        // Si es un resultado de schema refresh, procesar las tablas
        if let Some(result) = self.query_results.get(self.current_result_index) {
            if result.query.contains("SHOW TABLES") || result.query.contains("SELECT tablename") || result.query.contains("SELECT name") {
                self.parse_tables_from_result(&result_text);
            }
        }

        // Actualizar estado de conexión basado en el resultado
        if has_error {
            println!("❌ Error en consulta: {}", result_text);
            self.connection_status = ConnectionStatus::Error(format!("Error en la consulta: {}", result_text));
        } else {
            println!("✅ Consulta exitosa: {}", result_text);
            self.connection_status = ConnectionStatus::Connected;
        }
    }

    pub fn parse_tables_from_result(&mut self, result: &str) {
        self.tables.clear();

        // Parsear resultado de SHOW TABLES o similar
        for line in result.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('+') && !line.starts_with('|') && !line.starts_with('-') {
                // Limpiar el nombre de la tabla
                let table_name = line.split_whitespace().next().unwrap_or("").to_string();
                if !table_name.is_empty() {
                    let table_info = TableInfo {
                        name: table_name,
                        columns: Vec::new(), // Se cargarían con DESCRIBE
                        row_count: None,
                        table_type: "table".to_string(),
                    };
                    self.tables.push(table_info);
                }
            }
        }
    }
}