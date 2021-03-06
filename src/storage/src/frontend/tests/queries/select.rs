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

#[rstest::fixture]
fn with_small_ints_table(default_schema_name: &str, mut storage_with_schema: PersistentStorage) -> PersistentStorage {
    create_table(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![
            column_definition("column_1", SqlType::SmallInt(i16::min_value())),
            column_definition("column_2", SqlType::SmallInt(i16::min_value())),
            column_definition("column_3", SqlType::SmallInt(i16::min_value())),
        ],
    );
    storage_with_schema
}

#[rstest::rstest]
fn select_from_table_from_non_existent_schema(mut storage: PersistentStorage) {
    assert_eq!(
        storage
            .select_all_from("non_existent", "table_name", vec![])
            .expect("no system errors"),
        Err(OperationOnTableError::SchemaDoesNotExist)
    );
}

#[rstest::rstest]
fn select_from_table_that_does_not_exist(default_schema_name: &str, mut storage_with_schema: PersistentStorage) {
    let table_columns = storage_with_schema
        .table_columns(default_schema_name, "not_existed")
        .expect("no system errors")
        .into_iter()
        .map(|column_definition| column_definition.name())
        .collect();

    assert_eq!(
        storage_with_schema
            .select_all_from(default_schema_name, "not_existed", table_columns)
            .expect("no system errors"),
        Err(OperationOnTableError::TableDoesNotExist)
    );
}

#[rstest::rstest]
fn select_all_from_table_with_many_columns(default_schema_name: &str, mut with_small_ints_table: PersistentStorage) {
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["1", "2", "3"],
    );

    let table_columns = with_small_ints_table
        .table_columns(default_schema_name, "table_name")
        .expect("no system errors")
        .into_iter()
        .map(|column_definition| column_definition.name())
        .collect();

    assert_eq!(
        with_small_ints_table
            .select_all_from(default_schema_name, "table_name", table_columns)
            .expect("no system errors"),
        Ok((
            vec![
                column_definition("column_1", SqlType::SmallInt(i16::min_value())),
                column_definition("column_2", SqlType::SmallInt(i16::min_value())),
                column_definition("column_3", SqlType::SmallInt(i16::min_value()))
            ],
            vec![vec!["1".to_owned(), "2".to_owned(), "3".to_owned()]]
        ))
    );
}

#[rstest::rstest]
fn select_first_and_last_columns_from_table_with_multiple_columns(
    default_schema_name: &str,
    mut with_small_ints_table: PersistentStorage,
) {
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["1", "2", "3"],
    );
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["4", "5", "6"],
    );
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["7", "8", "9"],
    );

    assert_eq!(
        with_small_ints_table
            .select_all_from(
                default_schema_name,
                "table_name",
                vec!["column_1".to_owned(), "column_3".to_owned()]
            )
            .expect("no system errors"),
        Ok((
            vec![
                column_definition("column_1", SqlType::SmallInt(i16::min_value())),
                column_definition("column_3", SqlType::SmallInt(i16::min_value()))
            ],
            vec![
                vec!["1".to_owned(), "3".to_owned()],
                vec!["4".to_owned(), "6".to_owned()],
                vec!["7".to_owned(), "9".to_owned()],
            ],
        ))
    );
}

#[rstest::rstest]
fn select_all_columns_reordered_from_table_with_multiple_columns(
    default_schema_name: &str,
    mut with_small_ints_table: PersistentStorage,
) {
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["1", "2", "3"],
    );
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["4", "5", "6"],
    );
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["7", "8", "9"],
    );

    assert_eq!(
        with_small_ints_table
            .select_all_from(
                default_schema_name,
                "table_name",
                vec!["column_3".to_owned(), "column_1".to_owned(), "column_2".to_owned()]
            )
            .expect("no system errors"),
        Ok((
            vec![
                column_definition("column_3", SqlType::SmallInt(i16::min_value())),
                column_definition("column_1", SqlType::SmallInt(i16::min_value())),
                column_definition("column_2", SqlType::SmallInt(i16::min_value()))
            ],
            vec![
                vec!["3".to_owned(), "1".to_owned(), "2".to_owned()],
                vec!["6".to_owned(), "4".to_owned(), "5".to_owned()],
                vec!["9".to_owned(), "7".to_owned(), "8".to_owned()],
            ],
        ))
    );
}

#[rstest::rstest]
fn select_with_column_name_duplication(default_schema_name: &str, mut with_small_ints_table: PersistentStorage) {
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["1", "2", "3"],
    );
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["4", "5", "6"],
    );
    insert_into(
        &mut with_small_ints_table,
        default_schema_name,
        "table_name",
        vec![],
        vec!["7", "8", "9"],
    );

    assert_eq!(
        with_small_ints_table
            .select_all_from(
                default_schema_name,
                "table_name",
                vec![
                    "column_3".to_owned(),
                    "column_2".to_owned(),
                    "column_1".to_owned(),
                    "column_3".to_owned(),
                    "column_2".to_owned()
                ]
            )
            .expect("no system errors"),
        Ok((
            vec![
                column_definition("column_3", SqlType::SmallInt(i16::min_value())),
                column_definition("column_2", SqlType::SmallInt(i16::min_value())),
                column_definition("column_1", SqlType::SmallInt(i16::min_value())),
                column_definition("column_3", SqlType::SmallInt(i16::min_value())),
                column_definition("column_2", SqlType::SmallInt(i16::min_value()))
            ],
            vec![
                vec![
                    "3".to_owned(),
                    "2".to_owned(),
                    "1".to_owned(),
                    "3".to_owned(),
                    "2".to_owned()
                ],
                vec![
                    "6".to_owned(),
                    "5".to_owned(),
                    "4".to_owned(),
                    "6".to_owned(),
                    "5".to_owned()
                ],
                vec![
                    "9".to_owned(),
                    "8".to_owned(),
                    "7".to_owned(),
                    "9".to_owned(),
                    "8".to_owned()
                ],
            ],
        ))
    );
}

#[rstest::rstest]
fn select_different_integer_types(default_schema_name: &str, mut storage_with_schema: PersistentStorage) {
    create_table(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![
            column_definition("small_int", SqlType::SmallInt(i16::min_value())),
            column_definition("integer", SqlType::Integer(i32::min_value())),
            column_definition("big_int", SqlType::BigInt(i64::min_value())),
        ],
    );

    insert_into(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![],
        vec!["1000", "2000000", "3000000000"],
    );
    insert_into(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![],
        vec!["4000", "5000000", "6000000000"],
    );
    insert_into(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![],
        vec!["7000", "8000000", "9000000000"],
    );

    assert_eq!(
        storage_with_schema
            .select_all_from(
                default_schema_name,
                "table_name",
                vec!["small_int".to_owned(), "integer".to_owned(), "big_int".to_owned()]
            )
            .expect("no system errors"),
        Ok((
            vec![
                column_definition("small_int", SqlType::SmallInt(i16::min_value())),
                column_definition("integer", SqlType::Integer(i32::min_value())),
                column_definition("big_int", SqlType::BigInt(i64::min_value())),
            ],
            vec![
                vec!["1000".to_owned(), "2000000".to_owned(), "3000000000".to_owned()],
                vec!["4000".to_owned(), "5000000".to_owned(), "6000000000".to_owned()],
                vec!["7000".to_owned(), "8000000".to_owned(), "9000000000".to_owned()],
            ],
        ))
    );
}

#[rstest::rstest]
fn select_different_character_strings_types(default_schema_name: &str, mut storage_with_schema: PersistentStorage) {
    create_table(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![
            column_definition("char_10", SqlType::Char(10)),
            column_definition("var_char_20", SqlType::VarChar(20)),
        ],
    );

    insert_into(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![],
        vec!["1234567890", "12345678901234567890"],
    );
    insert_into(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![],
        vec!["12345", "1234567890"],
    );
    insert_into(
        &mut storage_with_schema,
        default_schema_name,
        "table_name",
        vec![],
        vec!["12345", "1234567890     "],
    );

    assert_eq!(
        storage_with_schema
            .select_all_from(
                default_schema_name,
                "table_name",
                vec!["char_10".to_owned(), "var_char_20".to_owned()]
            )
            .expect("no system errors"),
        Ok((
            vec![
                column_definition("char_10", SqlType::Char(10)),
                column_definition("var_char_20", SqlType::VarChar(20)),
            ],
            vec![
                vec!["1234567890".to_owned(), "12345678901234567890".to_owned()],
                vec!["12345".to_owned(), "1234567890".to_owned()],
                vec!["12345".to_owned(), "1234567890".to_owned()],
            ],
        ))
    );
}
