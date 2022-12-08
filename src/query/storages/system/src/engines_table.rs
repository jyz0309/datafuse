// Copyright 2021 Datafuse Labs.
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

use std::sync::Arc;

use common_catalog::catalog_kind::CATALOG_DEFAULT;
use common_catalog::table::Table;
use common_catalog::table_context::TableContext;
use common_exception::Result;
use common_expression::types::DataType;
use common_expression::utils::ColumnFrom;
use common_expression::Chunk;
use common_expression::Column;
use common_expression::DataField;
use common_expression::DataSchemaRefExt;
use common_expression::SchemaDataType;
use common_expression::Value;
use common_meta_app::schema::TableIdent;
use common_meta_app::schema::TableInfo;
use common_meta_app::schema::TableMeta;

use crate::table::AsyncOneBlockSystemTable;
use crate::table::AsyncSystemTable;

pub struct EnginesTable {
    table_info: TableInfo,
}

#[async_trait::async_trait]
impl AsyncSystemTable for EnginesTable {
    const NAME: &'static str = "system.engines";

    fn get_table_info(&self) -> &TableInfo {
        &self.table_info
    }

    async fn get_full_data(&self, ctx: Arc<dyn TableContext>) -> Result<Chunk> {
        // TODO passin catalog name
        let table_engine_descriptors = ctx.get_catalog(CATALOG_DEFAULT)?.get_table_engines();
        let mut engine_name = Vec::with_capacity(table_engine_descriptors.len());
        let mut engine_comment = Vec::with_capacity(table_engine_descriptors.len());
        for descriptor in &table_engine_descriptors {
            engine_name.push(descriptor.engine_name.as_bytes().to_vec());
            engine_comment.push(descriptor.comment.as_bytes().to_vec());
        }

        let rows_len = table_engine_descriptors.len();
        Ok(Chunk::new_from_sequence(
            vec![
                (
                    Value::Column(Column::from_data(engine_name)),
                    DataType::String,
                ),
                (
                    Value::Column(Column::from_data(engine_comment)),
                    DataType::String,
                ),
            ],
            rows_len,
        ))
    }
}

impl EnginesTable {
    pub fn create(table_id: u64) -> Arc<dyn Table> {
        let schema = DataSchemaRefExt::create(vec![
            DataField::new("Engine", SchemaDataType::String),
            DataField::new("Comment", SchemaDataType::String),
        ]);

        let table_info = TableInfo {
            desc: "'system'.'engines'".to_string(),
            name: "engines".to_string(),
            ident: TableIdent::new(table_id, 0),
            meta: TableMeta {
                schema,
                engine: "SystemEngines".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        AsyncOneBlockSystemTable::create(EnginesTable { table_info })
    }
}
