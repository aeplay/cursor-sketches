use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use wasm_bindgen::prelude::*;

/// Parse a SQL query and return a pretty-printed AST
#[wasm_bindgen]
pub fn parse_sql(sql: &str) -> String {
    let dialect = GenericDialect {};
    
    match Parser::parse_sql(&dialect, sql) {
        Ok(statements) => {
            if statements.is_empty() {
                return "No SQL statements found".to_string();
            }
            
            let mut result = format!("SQL: {}\n\nParsed {} statement(s):\n", sql, statements.len());
            
            for (i, stmt) in statements.iter().enumerate() {
                result.push_str(&format!("\n=== Statement {} ===\n{:#?}\n", i + 1, stmt));
            }
            
            result
        }
        Err(e) => format!("Parse error: {}", e),
    }
}
