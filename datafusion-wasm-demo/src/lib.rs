use std::collections::HashMap;
use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};
use datafusion_common::config::ConfigOptions;
use datafusion_common::TableReference;
use datafusion_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use datafusion_sql::planner::{ContextProvider, SqlToRel};
use datafusion_sql::sqlparser::dialect::GenericDialect;
use datafusion_sql::sqlparser::parser::Parser;
use wasm_bindgen::prelude::*;

/// A minimal context provider for SQL planning
struct MinimalContextProvider {
    tables: HashMap<String, Arc<dyn TableSource>>,
    options: ConfigOptions,
}

impl MinimalContextProvider {
    fn new() -> Self {
        let mut tables = HashMap::new();
        
        // Add a sample "users" table for demo purposes
        let users_schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("email", DataType::Utf8, true),
            Field::new("age", DataType::Int32, true),
        ]);
        tables.insert(
            "users".to_string(),
            Arc::new(MinimalTableSource::new(users_schema)) as Arc<dyn TableSource>,
        );

        // Add an "orders" table
        let orders_schema = Schema::new(vec![
            Field::new("order_id", DataType::Int64, false),
            Field::new("user_id", DataType::Int64, false),
            Field::new("amount", DataType::Float64, true),
            Field::new("created_at", DataType::Date32, true),
        ]);
        tables.insert(
            "orders".to_string(),
            Arc::new(MinimalTableSource::new(orders_schema)) as Arc<dyn TableSource>,
        );

        Self {
            tables,
            options: ConfigOptions::default(),
        }
    }
}

/// Minimal table source implementation
struct MinimalTableSource {
    schema: Arc<Schema>,
}

impl MinimalTableSource {
    fn new(schema: Schema) -> Self {
        Self {
            schema: Arc::new(schema),
        }
    }
}

impl TableSource for MinimalTableSource {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
}

impl ContextProvider for MinimalContextProvider {
    fn get_table_source(
        &self,
        name: TableReference,
    ) -> datafusion_common::Result<Arc<dyn TableSource>> {
        let table_name = name.table();
        self.tables
            .get(table_name)
            .cloned()
            .ok_or_else(|| datafusion_common::DataFusionError::Plan(format!("Table not found: {}", table_name)))
    }

    fn get_function_meta(&self, _name: &str) -> Option<Arc<ScalarUDF>> {
        None
    }

    fn get_aggregate_meta(&self, _name: &str) -> Option<Arc<AggregateUDF>> {
        None
    }

    fn get_window_meta(&self, _name: &str) -> Option<Arc<WindowUDF>> {
        None
    }

    fn get_variable_type(&self, _variable_names: &[String]) -> Option<DataType> {
        None
    }

    fn options(&self) -> &ConfigOptions {
        &self.options
    }

    fn udf_names(&self) -> Vec<String> {
        vec![]
    }

    fn udaf_names(&self) -> Vec<String> {
        vec![]
    }

    fn udwf_names(&self) -> Vec<String> {
        vec![]
    }
}

/// Parse a SQL query and return a pretty-printed logical plan
#[wasm_bindgen]
pub fn parse_sql_to_plan(sql: &str) -> String {
    let dialect = GenericDialect {};
    
    // Parse SQL
    let statements = match Parser::parse_sql(&dialect, sql) {
        Ok(stmts) => stmts,
        Err(e) => return format!("Parse error: {}", e),
    };

    if statements.is_empty() {
        return "No SQL statements found".to_string();
    }

    let statement = &statements[0];
    
    // Create context and planner
    let context = MinimalContextProvider::new();
    let planner = SqlToRel::new(&context);

    // Convert to logical plan
    match planner.sql_statement_to_plan(statement.clone()) {
        Ok(plan) => {
            format!(
                "SQL: {}\n\nLogical Plan:\n{}",
                sql,
                plan.display_indent()
            )
        }
        Err(e) => format!("Planning error: {}", e),
    }
}

/// Get available tables in the demo context
#[wasm_bindgen]
pub fn get_available_tables() -> String {
    "Available tables:\n\
     - users (id: Int64, name: Utf8, email: Utf8, age: Int32)\n\
     - orders (order_id: Int64, user_id: Int64, amount: Float64, created_at: Date32)"
        .to_string()
}
