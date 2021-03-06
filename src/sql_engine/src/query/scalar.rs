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

///! Module for representing scalar level operations. Implementation of
///! theses operators will be defined in a sperate module.
use super::{ColumnType, RelationType, Row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {}

/// Operation performed on the table
/// influenced by Materialized's ScalarExpr
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarOp {
    /// column access
    Column(usize),
    /// literal value (owned) and expected type.
    Literal(Vec<Row>, RelationType),
    /// binary operator
    Binary(BinaryOp, Box<ScalarOp>, Box<ScalarOp>),
    /// uanry operator
    Unary(UnaryOp, Box<ScalarOp>),
}
