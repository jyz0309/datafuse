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

use common_base::base::mask_string;
use common_catalog::table::Table;
use common_catalog::table_context::TableContext;
use common_config::GlobalConfig;
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
use itertools::Itertools;
use serde_json::Value as JsonValue;

use crate::SyncOneBlockSystemTable;
use crate::SyncSystemTable;

pub struct ConfigsTable {
    table_info: TableInfo,
}

impl SyncSystemTable for ConfigsTable {
    const NAME: &'static str = "system.config";

    fn get_table_info(&self) -> &TableInfo {
        &self.table_info
    }

    fn get_full_data(&self, _ctx: Arc<dyn TableContext>) -> Result<Chunk> {
        let config = GlobalConfig::instance().as_ref().clone().into_outer();

        let mut names: Vec<String> = vec![];
        let mut values: Vec<String> = vec![];
        let mut groups: Vec<String> = vec![];
        let mut descs: Vec<String> = vec![];

        let query_config = config.query;
        let query_config_value = serde_json::to_value(query_config)?;
        ConfigsTable::extract_config(
            &mut names,
            &mut values,
            &mut groups,
            &mut descs,
            "query".to_string(),
            query_config_value,
        );

        let log_config = config.log;
        let log_config_value = serde_json::to_value(log_config)?;
        ConfigsTable::extract_config(
            &mut names,
            &mut values,
            &mut groups,
            &mut descs,
            "log".to_string(),
            log_config_value,
        );

        let meta_config = config.meta;
        let meta_config_value = serde_json::to_value(meta_config)?;
        ConfigsTable::extract_config(
            &mut names,
            &mut values,
            &mut groups,
            &mut descs,
            "meta".to_string(),
            meta_config_value,
        );

        // Clone storage config to avoid change it's value.
        //
        // TODO(xuanwo):
        // Refactor into config so that config can  decide which value needs mask.
        let mut storage_config = config.storage;
        storage_config.s3.access_key_id = mask_string(&storage_config.s3.access_key_id, 3);
        storage_config.s3.secret_access_key = mask_string(&storage_config.s3.secret_access_key, 3);
        storage_config.gcs.credential = mask_string(&storage_config.gcs.credential, 3);
        storage_config.azblob.account_name = mask_string(&storage_config.azblob.account_name, 3);
        storage_config.azblob.account_key = mask_string(&storage_config.azblob.account_key, 3);
        let storage_config_value = serde_json::to_value(storage_config)?;
        ConfigsTable::extract_config(
            &mut names,
            &mut values,
            &mut groups,
            &mut descs,
            "storage".to_string(),
            storage_config_value,
        );

        let names: Vec<Vec<u8>> = names.iter().map(|x| x.as_bytes().to_vec()).collect();
        let values: Vec<Vec<u8>> = values.iter().map(|x| x.as_bytes().to_vec()).collect();
        let groups: Vec<Vec<u8>> = groups.iter().map(|x| x.as_bytes().to_vec()).collect();
        let descs: Vec<Vec<u8>> = descs.iter().map(|x| x.as_bytes().to_vec()).collect();

        let rows_len = names.len();
        Ok(Chunk::new_from_sequence(
            vec![
                (Value::Column(Column::from_data(groups)), DataType::String),
                (Value::Column(Column::from_data(names)), DataType::String),
                (Value::Column(Column::from_data(values)), DataType::String),
                (Value::Column(Column::from_data(descs)), DataType::String),
            ],
            rows_len,
        ))
    }
}

impl ConfigsTable {
    pub fn create(table_id: u64) -> Arc<dyn Table> {
        let schema = DataSchemaRefExt::create(vec![
            DataField::new("group", SchemaDataType::String),
            DataField::new("name", SchemaDataType::String),
            DataField::new("value", SchemaDataType::String),
            DataField::new("description", SchemaDataType::String),
        ]);

        let table_info = TableInfo {
            desc: "'system'.'configs'".to_string(),
            name: "configs".to_string(),
            ident: TableIdent::new(table_id, 0),
            meta: TableMeta {
                schema,
                engine: "SystemConfigs".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        SyncOneBlockSystemTable::create(ConfigsTable { table_info })
    }

    fn extract_config(
        names: &mut Vec<String>,
        values: &mut Vec<String>,
        groups: &mut Vec<String>,
        descs: &mut Vec<String>,
        group: String,
        config_value: JsonValue,
    ) {
        ConfigsTable::extract_config_with_name_prefix(
            names,
            values,
            groups,
            descs,
            group,
            config_value,
            None,
        );
    }

    fn extract_config_with_name_prefix(
        names: &mut Vec<String>,
        values: &mut Vec<String>,
        groups: &mut Vec<String>,
        descs: &mut Vec<String>,
        group: String,
        config_value: JsonValue,
        name_prefix: Option<String>,
    ) {
        for (k, v) in config_value.as_object().unwrap().into_iter() {
            match v {
                JsonValue::String(s) => ConfigsTable::push_config(
                    names,
                    values,
                    groups,
                    descs,
                    k.to_string(),
                    s.to_string(),
                    group.clone(),
                    "".to_string(),
                    name_prefix.clone(),
                ),
                JsonValue::Number(n) => ConfigsTable::push_config(
                    names,
                    values,
                    groups,
                    descs,
                    k.to_string(),
                    n.to_string(),
                    group.clone(),
                    "".to_string(),
                    name_prefix.clone(),
                ),
                JsonValue::Bool(b) => ConfigsTable::push_config(
                    names,
                    values,
                    groups,
                    descs,
                    k.to_string(),
                    b.to_string(),
                    group.clone(),
                    "".to_string(),
                    name_prefix.clone(),
                ),
                JsonValue::Array(v) => ConfigsTable::push_config(
                    names,
                    values,
                    groups,
                    descs,
                    k.to_string(),
                    v.iter().join(","),
                    group.clone(),
                    "".to_string(),
                    name_prefix.clone(),
                ),
                JsonValue::Object(_) => ConfigsTable::extract_config_with_name_prefix(
                    names,
                    values,
                    groups,
                    descs,
                    group.clone(),
                    v.clone(),
                    if let Some(prefix) = &name_prefix {
                        Some(format!("{prefix}.{k}"))
                    } else {
                        Some(k.to_string())
                    },
                ),
                JsonValue::Null => ConfigsTable::push_config(
                    names,
                    values,
                    groups,
                    descs,
                    k.to_string(),
                    "null".to_string(),
                    group.clone(),
                    "".to_string(),
                    name_prefix.clone(),
                ),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_config(
        names: &mut Vec<String>,
        values: &mut Vec<String>,
        groups: &mut Vec<String>,
        descs: &mut Vec<String>,
        name: String,
        value: String,
        group: String,
        desc: String,
        name_prefix: Option<String>,
    ) {
        if let Some(prefix) = name_prefix {
            names.push(format!("{}.{}", prefix, name));
        } else {
            names.push(name);
        }
        values.push(value);
        groups.push(group);
        descs.push(desc);
    }
}
