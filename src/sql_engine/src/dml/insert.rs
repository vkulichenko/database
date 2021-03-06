// Copyright 2020 Alex Dukhno
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::dml::ExpressionEvaluation;
use kernel::SystemResult;
use protocol::{
    results::{QueryErrorBuilder, QueryEvent},
    Sender,
};
use sql_types::ConstraintError;
use sqlparser::ast::{DataType, Expr, Ident, ObjectName, Query, SetExpr, UnaryOperator, Value};
use std::sync::{Arc, Mutex};
use storage::{backend::BackendStorage, frontend::FrontendStorage, ColumnDefinition, OperationOnTableError};

pub(crate) struct InsertCommand<'ic, P: BackendStorage> {
    raw_sql_query: &'ic str,
    name: ObjectName,
    columns: Vec<Ident>,
    source: Box<Query>,
    storage: Arc<Mutex<FrontendStorage<P>>>,
    session: Arc<dyn Sender>,
}

impl<'ic, P: BackendStorage> InsertCommand<'ic, P> {
    pub(crate) fn new(
        raw_sql_query: &'ic str,
        name: ObjectName,
        columns: Vec<Ident>,
        source: Box<Query>,
        storage: Arc<Mutex<FrontendStorage<P>>>,
        session: Arc<dyn Sender>,
    ) -> InsertCommand<'ic, P> {
        InsertCommand {
            raw_sql_query,
            name,
            columns,
            source,
            storage,
            session,
        }
    }

    pub(crate) fn execute(&mut self) -> SystemResult<()> {
        let table_name = self.name.0.pop().unwrap().to_string();
        let schema_name = self.name.0.pop().unwrap().to_string();
        let Query { body, .. } = &*self.source;
        if let SetExpr::Values(values) = &body {
            let values = &values.0;

            let columns = if self.columns.is_empty() {
                vec![]
            } else {
                self.columns
                    .clone()
                    .into_iter()
                    .map(|id| {
                        let sqlparser::ast::Ident { value, .. } = id;
                        value
                    })
                    .collect()
            };

            let mut rows = vec![];
            for line in values {
                let mut row = vec![];
                for col in line {
                    let v = match col {
                        Expr::Value(Value::Number(v)) => v.to_string(),
                        Expr::Value(Value::SingleQuotedString(v)) => v.to_string(),
                        Expr::Value(Value::Boolean(v)) => v.to_string(),
                        Expr::Cast { expr, data_type } => match (&**expr, data_type) {
                            (Expr::Value(Value::Boolean(v)), DataType::Boolean) => v.to_string(),
                            (Expr::Value(Value::SingleQuotedString(v)), DataType::Boolean) => v.to_string(),
                            _ => {
                                self.session
                                    .send(Err(QueryErrorBuilder::new()
                                        .syntax_error(format!(
                                            "Cast from {:?} to {:?} is not currently supported",
                                            expr, data_type
                                        ))
                                        .build()))
                                    .expect("To Send Query Result to Client");
                                return Ok(());
                            }
                        },
                        Expr::UnaryOp { op, expr } => match (op, &**expr) {
                            (UnaryOperator::Minus, Expr::Value(Value::Number(v))) => {
                                "-".to_owned() + v.to_string().as_str()
                            }
                            (op, expr) => {
                                self.session
                                    .send(Err(QueryErrorBuilder::new()
                                        .syntax_error(op.to_string() + expr.to_string().as_str())
                                        .build()))
                                    .expect("To Send Query Result to Client");
                                return Ok(());
                            }
                        },
                        expr @ Expr::BinaryOp { .. } => {
                            match ExpressionEvaluation::new(self.session.clone()).eval(expr) {
                                Ok(expr_result) => expr_result.value(),
                                Err(()) => return Ok(()),
                            }
                        }
                        expr => {
                            self.session
                                .send(Err(QueryErrorBuilder::new().syntax_error(expr.to_string()).build()))
                                .expect("To Send Query Result to Client");
                            return Ok(());
                        }
                    };
                    row.push(v);
                }
                rows.push(row);
            }

            let len = rows.len();
            match (self.storage.lock().unwrap()).insert_into(&schema_name, &table_name, columns, rows)? {
                Ok(_) => {
                    self.session
                        .send(Ok(QueryEvent::RecordsInserted(len)))
                        .expect("To Send Query Result to Client");
                    Ok(())
                }
                Err(OperationOnTableError::SchemaDoesNotExist) => {
                    self.session
                        .send(Err(QueryErrorBuilder::new().schema_does_not_exist(schema_name).build()))
                        .expect("To Send Query Result to Client");
                    Ok(())
                }
                Err(OperationOnTableError::TableDoesNotExist) => {
                    self.session
                        .send(Err(QueryErrorBuilder::new()
                            .table_does_not_exist(schema_name + "." + table_name.as_str())
                            .build()))
                        .expect("To Send Query Result to Client");
                    Ok(())
                }
                Err(OperationOnTableError::ColumnDoesNotExist(non_existing_columns)) => {
                    self.session
                        .send(Err(QueryErrorBuilder::new()
                            .column_does_not_exist(non_existing_columns)
                            .build()))
                        .expect("To Send Query Result to Client");
                    Ok(())
                }
                Err(OperationOnTableError::ConstraintViolations(constraint_errors, row_index)) => {
                    let mut builder = QueryErrorBuilder::new();
                    let mut constraint_error_mapper =
                        |err: &ConstraintError, column_definition: &ColumnDefinition, row_index: usize| match err {
                            ConstraintError::OutOfRange => {
                                builder.out_of_range(
                                    column_definition.sql_type().to_pg_types(),
                                    column_definition.name(),
                                    row_index,
                                );
                            }
                            ConstraintError::TypeMismatch(value) => {
                                builder.type_mismatch(
                                    value,
                                    column_definition.sql_type().to_pg_types(),
                                    column_definition.name(),
                                    row_index,
                                );
                            }
                            ConstraintError::ValueTooLong(len) => {
                                builder.string_length_mismatch(
                                    column_definition.sql_type().to_pg_types(),
                                    *len,
                                    column_definition.name(),
                                    row_index,
                                );
                            }
                        };

                    constraint_errors.iter().for_each(|(err, column_definition)| {
                        constraint_error_mapper(err, column_definition, row_index)
                    });
                    self.session
                        .send(Err(builder.build()))
                        .expect("To Send Query Result to Client");
                    Ok(())
                }
                Err(OperationOnTableError::InsertTooManyExpressions) => {
                    self.session
                        .send(Err(QueryErrorBuilder::new().too_many_insert_expressions().build()))
                        .expect("To Send Query Result to Client");
                    Ok(())
                }
            }
        } else {
            self.session
                .send(Err(QueryErrorBuilder::new()
                    .feature_not_supported(self.raw_sql_query.to_owned())
                    .build()))
                .expect("To Send Query Result to Client");
            Ok(())
        }
    }
}
