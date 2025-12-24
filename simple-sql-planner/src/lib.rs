use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::fmt;
use wasm_bindgen::prelude::*;

// ============================================================================
// Simple Schema Definitions (no Arrow dependency!)
// ============================================================================

#[derive(Debug, Clone)]
pub enum DataType {
    Int32,
    Int64,
    Float64,
    Utf8,
    Boolean,
    Date,
    Timestamp,
    Unknown,
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Int32 => write!(f, "Int32"),
            DataType::Int64 => write!(f, "Int64"),
            DataType::Float64 => write!(f, "Float64"),
            DataType::Utf8 => write!(f, "Utf8"),
            DataType::Boolean => write!(f, "Boolean"),
            DataType::Date => write!(f, "Date"),
            DataType::Timestamp => write!(f, "Timestamp"),
            DataType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub fields: Vec<Field>,
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }
}

// ============================================================================
// Logical Plan Nodes
// ============================================================================

#[derive(Debug, Clone)]
pub enum LogicalPlan {
    /// Scan a table
    TableScan {
        table_name: String,
        schema: Schema,
        projection: Option<Vec<usize>>,
    },
    /// Project specific expressions
    Projection {
        exprs: Vec<LogicalExpr>,
        input: Box<LogicalPlan>,
    },
    /// Filter rows
    Filter {
        predicate: LogicalExpr,
        input: Box<LogicalPlan>,
    },
    /// Join two inputs
    Join {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        on: LogicalExpr,
        join_type: JoinType,
    },
    /// Aggregate
    Aggregate {
        input: Box<LogicalPlan>,
        group_by: Vec<LogicalExpr>,
        aggregates: Vec<LogicalExpr>,
    },
    /// Sort
    Sort {
        input: Box<LogicalPlan>,
        order_by: Vec<(LogicalExpr, bool)>, // (expr, ascending)
    },
    /// Limit
    Limit {
        input: Box<LogicalPlan>,
        limit: usize,
    },
    /// Subquery
    Subquery {
        input: Box<LogicalPlan>,
        alias: String,
    },
}

#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone)]
pub enum LogicalExpr {
    Column { name: String, table: Option<String> },
    Literal(LiteralValue),
    BinaryOp { left: Box<LogicalExpr>, op: String, right: Box<LogicalExpr> },
    Function { name: String, args: Vec<LogicalExpr> },
    Alias { expr: Box<LogicalExpr>, alias: String },
    Wildcard,
    IsNull { expr: Box<LogicalExpr>, negated: bool },
    Between { expr: Box<LogicalExpr>, low: Box<LogicalExpr>, high: Box<LogicalExpr>, negated: bool },
    InList { expr: Box<LogicalExpr>, list: Vec<LogicalExpr>, negated: bool },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Int64(i64),
    Float64(f64),
    Utf8(String),
    Boolean(bool),
    Null,
}

// ============================================================================
// Pretty Printing
// ============================================================================

impl LogicalPlan {
    pub fn display_indent(&self) -> String {
        self.display_indent_impl(0)
    }

    fn display_indent_impl(&self, indent: usize) -> String {
        let prefix = "  ".repeat(indent);
        match self {
            LogicalPlan::TableScan { table_name, projection, .. } => {
                let proj_str = match projection {
                    Some(cols) => format!(", projection=[{:?}]", cols),
                    None => String::new(),
                };
                format!("{}TableScan: {}{}\n", prefix, table_name, proj_str)
            }
            LogicalPlan::Projection { exprs, input } => {
                let exprs_str: Vec<String> = exprs.iter().map(|e| format!("{}", e)).collect();
                format!(
                    "{}Projection: {}\n{}",
                    prefix,
                    exprs_str.join(", "),
                    input.display_indent_impl(indent + 1)
                )
            }
            LogicalPlan::Filter { predicate, input } => {
                format!(
                    "{}Filter: {}\n{}",
                    prefix,
                    predicate,
                    input.display_indent_impl(indent + 1)
                )
            }
            LogicalPlan::Join { left, right, on, join_type } => {
                format!(
                    "{}{:?} Join: {}\n{}{}",
                    prefix,
                    join_type,
                    on,
                    left.display_indent_impl(indent + 1),
                    right.display_indent_impl(indent + 1)
                )
            }
            LogicalPlan::Aggregate { input, group_by, aggregates } => {
                let group_str: Vec<String> = group_by.iter().map(|e| format!("{}", e)).collect();
                let agg_str: Vec<String> = aggregates.iter().map(|e| format!("{}", e)).collect();
                format!(
                    "{}Aggregate: groupBy=[{}], aggr=[{}]\n{}",
                    prefix,
                    group_str.join(", "),
                    agg_str.join(", "),
                    input.display_indent_impl(indent + 1)
                )
            }
            LogicalPlan::Sort { input, order_by } => {
                let order_str: Vec<String> = order_by
                    .iter()
                    .map(|(e, asc)| format!("{} {}", e, if *asc { "ASC" } else { "DESC" }))
                    .collect();
                format!(
                    "{}Sort: {}\n{}",
                    prefix,
                    order_str.join(", "),
                    input.display_indent_impl(indent + 1)
                )
            }
            LogicalPlan::Limit { input, limit } => {
                format!(
                    "{}Limit: {}\n{}",
                    prefix,
                    limit,
                    input.display_indent_impl(indent + 1)
                )
            }
            LogicalPlan::Subquery { input, alias } => {
                format!(
                    "{}Subquery: {}\n{}",
                    prefix,
                    alias,
                    input.display_indent_impl(indent + 1)
                )
            }
        }
    }
}

impl fmt::Display for LogicalExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalExpr::Column { name, table } => {
                if let Some(t) = table {
                    write!(f, "{}.{}", t, name)
                } else {
                    write!(f, "{}", name)
                }
            }
            LogicalExpr::Literal(v) => write!(f, "{:?}", v),
            LogicalExpr::BinaryOp { left, op, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
            LogicalExpr::Function { name, args } => {
                let args_str: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
                write!(f, "{}({})", name, args_str.join(", "))
            }
            LogicalExpr::Alias { expr, alias } => write!(f, "{} AS {}", expr, alias),
            LogicalExpr::Wildcard => write!(f, "*"),
            LogicalExpr::IsNull { expr, negated } => {
                if *negated {
                    write!(f, "{} IS NOT NULL", expr)
                } else {
                    write!(f, "{} IS NULL", expr)
                }
            }
            LogicalExpr::Between { expr, low, high, negated } => {
                if *negated {
                    write!(f, "{} NOT BETWEEN {} AND {}", expr, low, high)
                } else {
                    write!(f, "{} BETWEEN {} AND {}", expr, low, high)
                }
            }
            LogicalExpr::InList { expr, list, negated } => {
                let list_str: Vec<String> = list.iter().map(|e| format!("{}", e)).collect();
                if *negated {
                    write!(f, "{} NOT IN ({})", expr, list_str.join(", "))
                } else {
                    write!(f, "{} IN ({})", expr, list_str.join(", "))
                }
            }
        }
    }
}

// ============================================================================
// Schema Catalog (simple in-memory)
// ============================================================================

pub struct Catalog {
    tables: HashMap<String, Schema>,
}

impl Catalog {
    pub fn new() -> Self {
        let mut tables = HashMap::new();
        
        // Demo tables
        tables.insert("users".to_string(), Schema::new(vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false },
            Field { name: "name".to_string(), data_type: DataType::Utf8, nullable: true },
            Field { name: "email".to_string(), data_type: DataType::Utf8, nullable: true },
            Field { name: "age".to_string(), data_type: DataType::Int32, nullable: true },
        ]));
        
        tables.insert("orders".to_string(), Schema::new(vec![
            Field { name: "order_id".to_string(), data_type: DataType::Int64, nullable: false },
            Field { name: "user_id".to_string(), data_type: DataType::Int64, nullable: false },
            Field { name: "amount".to_string(), data_type: DataType::Float64, nullable: true },
            Field { name: "created_at".to_string(), data_type: DataType::Date, nullable: true },
        ]));
        
        Self { tables }
    }
    
    pub fn get_table(&self, name: &str) -> Option<&Schema> {
        self.tables.get(name)
    }
}

// ============================================================================
// SQL to Logical Plan Converter
// ============================================================================

pub struct Planner {
    catalog: Catalog,
}

impl Planner {
    pub fn new() -> Self {
        Self { catalog: Catalog::new() }
    }
    
    pub fn plan(&self, stmt: &Statement) -> Result<LogicalPlan, String> {
        match stmt {
            Statement::Query(query) => self.plan_query(query),
            _ => Err(format!("Unsupported statement type: {:?}", stmt)),
        }
    }
    
    fn plan_query(&self, query: &Query) -> Result<LogicalPlan, String> {
        let plan = self.plan_set_expr(&query.body)?;
        
        // Handle ORDER BY
        let plan = if let Some(order_by) = &query.order_by {
            if !order_by.exprs.is_empty() {
                let order_exprs: Result<Vec<(LogicalExpr, bool)>, String> = order_by.exprs.iter().map(|o| {
                    let expr = self.plan_expr(&o.expr)?;
                    let asc = o.asc.unwrap_or(true);
                    Ok((expr, asc))
                }).collect();
                LogicalPlan::Sort { input: Box::new(plan), order_by: order_exprs? }
            } else {
                plan
            }
        } else {
            plan
        };
        
        // Handle LIMIT
        let plan = if let Some(limit) = &query.limit {
            if let Expr::Value(Value::Number(n, _)) = limit {
                let limit_val: usize = n.parse().map_err(|_| "Invalid LIMIT value")?;
                LogicalPlan::Limit { input: Box::new(plan), limit: limit_val }
            } else {
                plan
            }
        } else {
            plan
        };
        
        Ok(plan)
    }
    
    fn plan_set_expr(&self, set_expr: &SetExpr) -> Result<LogicalPlan, String> {
        match set_expr {
            SetExpr::Select(select) => self.plan_select(select),
            SetExpr::Query(query) => self.plan_query(query),
            _ => Err(format!("Unsupported set expression: {:?}", set_expr)),
        }
    }
    
    fn plan_select(&self, select: &Select) -> Result<LogicalPlan, String> {
        // Step 1: Plan FROM clause
        let from_plan = self.plan_from(&select.from)?;
        
        // Step 2: Plan WHERE clause
        let filtered_plan = if let Some(selection) = &select.selection {
            let predicate = self.plan_expr(selection)?;
            LogicalPlan::Filter {
                predicate,
                input: Box::new(from_plan),
            }
        } else {
            from_plan
        };
        
        // Step 3: Check for aggregation
        let has_aggregates = select.projection.iter().any(|p| self.has_aggregate(p));
        let group_by_exprs = match &select.group_by {
            GroupByExpr::Expressions(exprs, _) => exprs.clone(),
            GroupByExpr::All(_) => vec![],
        };
        let has_group_by = !group_by_exprs.is_empty();
        
        let plan = if has_aggregates || has_group_by {
            // Extract group by expressions
            let group_by: Result<Vec<_>, _> = group_by_exprs.iter().map(|e| self.plan_expr(e)).collect();
            
            // Extract aggregate expressions
            let mut aggregates = Vec::new();
            for item in &select.projection {
                self.extract_aggregates(item, &mut aggregates)?;
            }
            
            LogicalPlan::Aggregate {
                input: Box::new(filtered_plan),
                group_by: group_by?,
                aggregates,
            }
        } else {
            filtered_plan
        };
        
        // Step 4: Plan projection
        let projection_exprs: Result<Vec<_>, _> = select
            .projection
            .iter()
            .map(|item| self.plan_select_item(item))
            .collect();
        
        Ok(LogicalPlan::Projection {
            exprs: projection_exprs?,
            input: Box::new(plan),
        })
    }
    
    fn plan_from(&self, from: &[TableWithJoins]) -> Result<LogicalPlan, String> {
        if from.is_empty() {
            return Err("FROM clause is required".to_string());
        }
        
        let mut plan = self.plan_table_factor(&from[0].relation)?;
        
        // Handle JOINs in first table
        for join in &from[0].joins {
            let right = self.plan_table_factor(&join.relation)?;
            let (join_type, on_expr) = self.plan_join_constraint(&join.join_operator)?;
            plan = LogicalPlan::Join {
                left: Box::new(plan),
                right: Box::new(right),
                on: on_expr,
                join_type,
            };
        }
        
        // Handle multiple tables in FROM (implicit cross join)
        for table_with_joins in from.iter().skip(1) {
            let right = self.plan_table_factor(&table_with_joins.relation)?;
            plan = LogicalPlan::Join {
                left: Box::new(plan),
                right: Box::new(right),
                on: LogicalExpr::Literal(LiteralValue::Boolean(true)),
                join_type: JoinType::Cross,
            };
        }
        
        Ok(plan)
    }
    
    fn plan_table_factor(&self, factor: &TableFactor) -> Result<LogicalPlan, String> {
        match factor {
            TableFactor::Table { name, alias, .. } => {
                let table_name = name.0.iter().map(|i| i.value.clone()).collect::<Vec<_>>().join(".");
                let schema = self.catalog.get_table(&table_name)
                    .ok_or_else(|| format!("Table not found: {}", table_name))?
                    .clone();
                
                let plan = LogicalPlan::TableScan {
                    table_name: table_name.clone(),
                    schema,
                    projection: None,
                };
                
                if let Some(alias) = alias {
                    Ok(LogicalPlan::Subquery {
                        input: Box::new(plan),
                        alias: alias.name.value.clone(),
                    })
                } else {
                    Ok(plan)
                }
            }
            TableFactor::Derived { subquery, alias, .. } => {
                let plan = self.plan_query(subquery)?;
                let alias_name = alias.as_ref().map(|a| a.name.value.clone()).unwrap_or_default();
                Ok(LogicalPlan::Subquery {
                    input: Box::new(plan),
                    alias: alias_name,
                })
            }
            _ => Err(format!("Unsupported table factor: {:?}", factor)),
        }
    }
    
    fn plan_join_constraint(&self, op: &JoinOperator) -> Result<(JoinType, LogicalExpr), String> {
        match op {
            JoinOperator::Inner(constraint) => {
                Ok((JoinType::Inner, self.plan_join_condition(constraint)?))
            }
            JoinOperator::LeftOuter(constraint) => {
                Ok((JoinType::Left, self.plan_join_condition(constraint)?))
            }
            JoinOperator::RightOuter(constraint) => {
                Ok((JoinType::Right, self.plan_join_condition(constraint)?))
            }
            JoinOperator::FullOuter(constraint) => {
                Ok((JoinType::Full, self.plan_join_condition(constraint)?))
            }
            JoinOperator::CrossJoin => {
                Ok((JoinType::Cross, LogicalExpr::Literal(LiteralValue::Boolean(true))))
            }
            _ => Err(format!("Unsupported join operator: {:?}", op)),
        }
    }
    
    fn plan_join_condition(&self, constraint: &JoinConstraint) -> Result<LogicalExpr, String> {
        match constraint {
            JoinConstraint::On(expr) => self.plan_expr(expr),
            JoinConstraint::Using(cols) => {
                // Convert USING to ON condition
                let conditions: Vec<LogicalExpr> = cols.iter().map(|col| {
                    LogicalExpr::BinaryOp {
                        left: Box::new(LogicalExpr::Column { name: col.value.clone(), table: None }),
                        op: "=".to_string(),
                        right: Box::new(LogicalExpr::Column { name: col.value.clone(), table: None }),
                    }
                }).collect();
                
                conditions.into_iter().reduce(|a, b| {
                    LogicalExpr::BinaryOp {
                        left: Box::new(a),
                        op: "AND".to_string(),
                        right: Box::new(b),
                    }
                }).ok_or_else(|| "Empty USING clause".to_string())
            }
            JoinConstraint::Natural => {
                Ok(LogicalExpr::Literal(LiteralValue::Boolean(true))) // Simplified
            }
            JoinConstraint::None => {
                Ok(LogicalExpr::Literal(LiteralValue::Boolean(true)))
            }
        }
    }
    
    fn plan_select_item(&self, item: &SelectItem) -> Result<LogicalExpr, String> {
        match item {
            SelectItem::UnnamedExpr(expr) => self.plan_expr(expr),
            SelectItem::ExprWithAlias { expr, alias } => {
                Ok(LogicalExpr::Alias {
                    expr: Box::new(self.plan_expr(expr)?),
                    alias: alias.value.clone(),
                })
            }
            SelectItem::Wildcard(_) => Ok(LogicalExpr::Wildcard),
            SelectItem::QualifiedWildcard(name, _) => {
                Ok(LogicalExpr::Column {
                    name: "*".to_string(),
                    table: Some(name.0.iter().map(|i| i.value.clone()).collect::<Vec<_>>().join(".")),
                })
            }
        }
    }
    
    fn plan_expr(&self, expr: &Expr) -> Result<LogicalExpr, String> {
        match expr {
            Expr::Identifier(ident) => {
                Ok(LogicalExpr::Column { name: ident.value.clone(), table: None })
            }
            Expr::CompoundIdentifier(idents) => {
                let parts: Vec<_> = idents.iter().map(|i| i.value.clone()).collect();
                if parts.len() == 2 {
                    Ok(LogicalExpr::Column { name: parts[1].clone(), table: Some(parts[0].clone()) })
                } else {
                    Ok(LogicalExpr::Column { name: parts.join("."), table: None })
                }
            }
            Expr::Value(value) => self.plan_value(value),
            Expr::BinaryOp { left, op, right } => {
                Ok(LogicalExpr::BinaryOp {
                    left: Box::new(self.plan_expr(left)?),
                    op: format!("{}", op),
                    right: Box::new(self.plan_expr(right)?),
                })
            }
            Expr::Function(func) => {
                let args: Result<Vec<_>, _> = match &func.args {
                    FunctionArguments::List(arg_list) => {
                        arg_list.args.iter().filter_map(|arg| {
                            match arg {
                                FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => Some(self.plan_expr(e)),
                                FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => Some(Ok(LogicalExpr::Wildcard)),
                                _ => None,
                            }
                        }).collect()
                    }
                    FunctionArguments::None => Ok(vec![]),
                    FunctionArguments::Subquery(_) => Err("Subquery args not supported".to_string()),
                };
                
                Ok(LogicalExpr::Function {
                    name: func.name.0.iter().map(|i| i.value.clone()).collect::<Vec<_>>().join(".").to_uppercase(),
                    args: args?,
                })
            }
            Expr::Nested(inner) => self.plan_expr(inner),
            Expr::IsNull(inner) => {
                Ok(LogicalExpr::IsNull { expr: Box::new(self.plan_expr(inner)?), negated: false })
            }
            Expr::IsNotNull(inner) => {
                Ok(LogicalExpr::IsNull { expr: Box::new(self.plan_expr(inner)?), negated: true })
            }
            Expr::Between { expr, low, high, negated } => {
                Ok(LogicalExpr::Between {
                    expr: Box::new(self.plan_expr(expr)?),
                    low: Box::new(self.plan_expr(low)?),
                    high: Box::new(self.plan_expr(high)?),
                    negated: *negated,
                })
            }
            Expr::InList { expr, list, negated } => {
                let list_exprs: Result<Vec<_>, _> = list.iter().map(|e| self.plan_expr(e)).collect();
                Ok(LogicalExpr::InList {
                    expr: Box::new(self.plan_expr(expr)?),
                    list: list_exprs?,
                    negated: *negated,
                })
            }
            _ => Err(format!("Unsupported expression: {:?}", expr)),
        }
    }
    
    fn plan_value(&self, value: &Value) -> Result<LogicalExpr, String> {
        match value {
            Value::Number(n, _) => {
                if n.contains('.') {
                    Ok(LogicalExpr::Literal(LiteralValue::Float64(n.parse().unwrap_or(0.0))))
                } else {
                    Ok(LogicalExpr::Literal(LiteralValue::Int64(n.parse().unwrap_or(0))))
                }
            }
            Value::SingleQuotedString(s) | Value::DoubleQuotedString(s) => {
                Ok(LogicalExpr::Literal(LiteralValue::Utf8(s.clone())))
            }
            Value::Boolean(b) => Ok(LogicalExpr::Literal(LiteralValue::Boolean(*b))),
            Value::Null => Ok(LogicalExpr::Literal(LiteralValue::Null)),
            _ => Err(format!("Unsupported value: {:?}", value)),
        }
    }
    
    fn has_aggregate(&self, item: &SelectItem) -> bool {
        match item {
            SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => {
                self.expr_has_aggregate(expr)
            }
            _ => false,
        }
    }
    
    fn expr_has_aggregate(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Function(func) => {
                let name = func.name.0.iter().map(|i| i.value.to_uppercase()).collect::<Vec<_>>().join(".");
                matches!(name.as_str(), "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "FIRST" | "LAST")
            }
            Expr::BinaryOp { left, right, .. } => {
                self.expr_has_aggregate(left) || self.expr_has_aggregate(right)
            }
            Expr::Nested(inner) => self.expr_has_aggregate(inner),
            _ => false,
        }
    }
    
    fn extract_aggregates(&self, item: &SelectItem, aggregates: &mut Vec<LogicalExpr>) -> Result<(), String> {
        match item {
            SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => {
                self.extract_aggregates_from_expr(expr, aggregates)
            }
            _ => Ok(()),
        }
    }
    
    fn extract_aggregates_from_expr(&self, expr: &Expr, aggregates: &mut Vec<LogicalExpr>) -> Result<(), String> {
        match expr {
            Expr::Function(func) => {
                let name = func.name.0.iter().map(|i| i.value.to_uppercase()).collect::<Vec<_>>().join(".");
                if matches!(name.as_str(), "COUNT" | "SUM" | "AVG" | "MIN" | "MAX") {
                    aggregates.push(self.plan_expr(expr)?);
                }
                Ok(())
            }
            Expr::BinaryOp { left, right, .. } => {
                self.extract_aggregates_from_expr(left, aggregates)?;
                self.extract_aggregates_from_expr(right, aggregates)
            }
            Expr::Nested(inner) => self.extract_aggregates_from_expr(inner, aggregates),
            _ => Ok(()),
        }
    }
}

// ============================================================================
// WASM Exports
// ============================================================================

#[wasm_bindgen]
pub fn parse_sql_to_plan(sql: &str) -> String {
    let dialect = GenericDialect {};
    
    let statements = match Parser::parse_sql(&dialect, sql) {
        Ok(stmts) => stmts,
        Err(e) => return format!("Parse error: {}", e),
    };

    if statements.is_empty() {
        return "No SQL statements found".to_string();
    }

    let planner = Planner::new();
    
    match planner.plan(&statements[0]) {
        Ok(plan) => {
            format!("SQL: {}\n\nLogical Plan:\n{}", sql, plan.display_indent())
        }
        Err(e) => format!("Planning error: {}", e),
    }
}

#[wasm_bindgen]
pub fn get_available_tables() -> String {
    "Available tables:\n\
     - users (id: Int64, name: Utf8, email: Utf8, age: Int32)\n\
     - orders (order_id: Int64, user_id: Int64, amount: Float64, created_at: Date)"
        .to_string()
}
