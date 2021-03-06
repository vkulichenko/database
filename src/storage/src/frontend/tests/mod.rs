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

use super::*;
use sql_types::SqlType;

#[cfg(test)]
mod queries;
#[cfg(test)]
mod schema;
#[cfg(test)]
mod table;

type PersistentStorage = FrontendStorage<SledBackendStorage>;

#[rstest::fixture]
fn default_schema_name() -> &'static str {
    "schema_name"
}

#[rstest::fixture]
fn storage() -> PersistentStorage {
    FrontendStorage::default().expect("no system errors")
}

#[rstest::fixture]
fn storage_with_schema(mut storage: PersistentStorage, default_schema_name: &str) -> PersistentStorage {
    create_schema(&mut storage, default_schema_name);
    storage
}

fn create_schema<P: backend::BackendStorage>(storage: &mut FrontendStorage<P>, schema_name: &str) {
    storage
        .create_schema(schema_name)
        .expect("no system errors")
        .expect("schema is created");
}

fn create_table<P: backend::BackendStorage>(
    storage: &mut FrontendStorage<P>,
    schema_name: &str,
    table_name: &str,
    column_names: Vec<ColumnDefinition>,
) {
    storage
        .create_table(schema_name, table_name, column_names.as_slice())
        .expect("no system errors")
        .expect("table is created");
}

fn column_definition(name: &'static str, sql_type: SqlType) -> ColumnDefinition {
    ColumnDefinition {
        name: name.to_owned(),
        sql_type,
    }
}

fn insert_into<P: backend::BackendStorage>(
    storage: &mut FrontendStorage<P>,
    schema_name: &str,
    table_name: &str,
    columns: Vec<&str>,
    values: Vec<&str>,
) {
    storage
        .insert_into(
            schema_name,
            table_name,
            columns.into_iter().map(ToOwned::to_owned).collect(),
            vec![values.into_iter().map(ToOwned::to_owned).collect()],
        )
        .expect("no system errors")
        .expect("values are inserted");
}
