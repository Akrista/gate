use crate::get_or_null;

use super::{ExecuteResult, Pool, TableRow, RECORDS_LIMIT_PER_PAGE};
use async_trait::async_trait;
use database_tree::{Child, Database, Schema, Table};
use futures::TryStreamExt;
use itertools::Itertools;
use sqlx::mssql::{MssqlColumn, MssqlPoolOptions, MssqlRow};
use sqlx::{Column as _, Row as _, TypeInfo as _};
use std::time::Duration;

pub struct MssqlPool {
    pool: sqlx::mssql::MssqlPool,
}

impl MssqlPool {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            pool: MssqlPoolOptions::new()
                .acquire_timeout(Duration::from_secs(5))
                .connect(database_url)
                .await?,
        })
    }
}

pub struct Constraint {
    name: String,
    column_name: String,
}

impl TableRow for Constraint {
    fn fields(&self) -> Vec<String> {
        vec!["name".to_string(), "column_name".to_string()]
    }

    fn columns(&self) -> Vec<String> {
        vec![self.name.to_string(), self.column_name.to_string()]
    }
}

pub struct Column {
    name: Option<String>,
    r#type: Option<String>,
    null: Option<String>,
    default: Option<String>,
    comment: Option<String>,
}

impl TableRow for Column {
    fn fields(&self) -> Vec<String> {
        vec![
            "name".to_string(),
            "type".to_string(),
            "null".to_string(),
            "default".to_string(),
            "comment".to_string(),
        ]
    }

    fn columns(&self) -> Vec<String> {
        vec![
            self.name
                .as_ref()
                .map_or(String::new(), |name| name.to_string()),
            self.r#type
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
            self.null
                .as_ref()
                .map_or(String::new(), |null| null.to_string()),
            self.default
                .as_ref()
                .map_or(String::new(), |default| default.to_string()),
            self.comment
                .as_ref()
                .map_or(String::new(), |comment| comment.to_string()),
        ]
    }
}

pub struct ForeignKey {
    name: Option<String>,
    column_name: Option<String>,
    ref_table: Option<String>,
    ref_column: Option<String>,
}

impl TableRow for ForeignKey {
    fn fields(&self) -> Vec<String> {
        vec![
            "name".to_string(),
            "column_name".to_string(),
            "ref_table".to_string(),
            "ref_column".to_string(),
        ]
    }

    fn columns(&self) -> Vec<String> {
        vec![
            self.name
                .as_ref()
                .map_or(String::new(), |name| name.to_string()),
            self.column_name
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
            self.ref_table
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
            self.ref_column
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
        ]
    }
}

pub struct Index {
    name: Option<String>,
    column_name: Option<String>,
    r#type: Option<String>,
}

impl TableRow for Index {
    fn fields(&self) -> Vec<String> {
        vec![
            "name".to_string(),
            "column_name".to_string(),
            "type".to_string(),
        ]
    }

    fn columns(&self) -> Vec<String> {
        vec![
            self.name
                .as_ref()
                .map_or(String::new(), |name| name.to_string()),
            self.column_name
                .as_ref()
                .map_or(String::new(), |column_name| column_name.to_string()),
            self.r#type
                .as_ref()
                .map_or(String::new(), |r#type| r#type.to_string()),
        ]
    }
}

#[async_trait]
impl Pool for MssqlPool {
    async fn execute(&self, query: &String) -> anyhow::Result<ExecuteResult> {
        let query = query.trim();
        if query.to_uppercase().starts_with("SELECT") {
            let mut rows = sqlx::query(query).fetch(&self.pool);
            let mut headers = vec![];
            let mut records = vec![];
            while let Some(row) = rows.try_next().await? {
                headers = row
                    .columns()
                    .iter()
                    .map(|column| column.name().to_string())
                    .collect();
                let mut new_row = vec![];
                for column in row.columns() {
                    new_row.push(convert_column_value_to_string(&row, column)?)
                }
                records.push(new_row)
            }
            return Ok(ExecuteResult::Read {
                headers,
                rows: records,
                database: Database {
                    name: "-".to_string(),
                    children: Vec::new(),
                },
                table: Table {
                    name: "-".to_string(),
                    create_time: None,
                    update_time: None,
                    engine: None,
                    schema: None,
                },
            });
        }

        let result = sqlx::query(query).execute(&self.pool).await?;
        Ok(ExecuteResult::Write {
            updated_rows: result.rows_affected(),
        })
    }

    async fn get_databases(&self) -> anyhow::Result<Vec<Database>> {
        let databases = sqlx::query(
            "SELECT NAME
        FROM SYS.DATABASES",
        )
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|table| table.get(0))
        .collect::<Vec<String>>();
        let mut list = vec![];
        for db in databases {
            list.push(Database::new(
                db.clone(),
                self.get_tables(db.clone()).await?,
            ))
        }
        Ok(list)
    }

    async fn get_tables(&self, database: String) -> anyhow::Result<Vec<Child>> {
        let query = format!(
            "
            USE {database}
            SELECT TABLE_NAME, TABLE_SCHEMA FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_CATALOG = '{database}'
            ",
            database = database
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut tables = Vec::new();
        while let Some(row) = rows.try_next().await? {
            tables.push(Table {
                name: row.try_get("TABLE_NAME")?,
                create_time: None,
                update_time: None,
                engine: None,
                schema: row.try_get("TABLE_SCHEMA")?,
            })
        }
        let mut schemas = vec![];
        for (key, group) in &tables
            .iter()
            .sorted_by(|a, b| Ord::cmp(&b.schema, &a.schema))
            .group_by(|t| t.schema.as_ref())
        {
            if let Some(key) = key {
                schemas.push(
                    Schema {
                        name: key.to_string(),
                        tables: group.cloned().collect(),
                    }
                    .into(),
                )
            }
        }
        Ok(schemas)
    }

    async fn get_records(
        &self,
        database: &Database,
        table: &Table,
        page: u16,
        filter: Option<String>,
    ) -> anyhow::Result<(Vec<String>, Vec<Vec<String>>)> {
        let query = if let Some(filter) = filter.as_ref() {
            format!(
                r#"SELECT * FROM "{database}"."{table_schema}"."{table}" WHERE {filter} ORDER BY 1 OFFSET {page} ROWS FETCH NEXT {limit} ROWS ONLY"#,
                database = database.name,
                table = table.name,
                filter = filter,
                table_schema = table.schema.clone().unwrap_or_else(|| "dbo".to_string()),
                page = page,
                limit = RECORDS_LIMIT_PER_PAGE
            )
        } else {
            format!(
                r#"SELECT * FROM "{database}"."{table_schema}"."{table}" ORDER BY 1 OFFSET {page} ROWS FETCH NEXT {limit} ROWS ONLY"#,
                database = database.name,
                table = table.name,
                table_schema = table.schema.clone().unwrap_or_else(|| "dbo".to_string()),
                page = page,
                limit = RECORDS_LIMIT_PER_PAGE
            )
        };
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut headers = vec![];
        let mut records = vec![];
        while let Some(row) = rows.try_next().await? {
            headers = row
                .columns()
                .iter()
                .map(|column| column.name().to_string())
                .collect();
            let mut new_row = vec![];
            for column in row.columns() {
                new_row.push(convert_column_value_to_string(&row, column)?)
            }
            records.push(new_row)
        }
        Ok((headers, records))
    }

    async fn get_columns(
        &self,
        database: &Database,
        table: &Table,
    ) -> anyhow::Result<Vec<Box<dyn TableRow>>> {
        let query = format!(
            "
            USE {}
            SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT
            FROM INFORMATION_SCHEMA.COLUMNS
            WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}'
            ",
            database.name,
            table.name,
            table.schema.clone().unwrap_or_else(|| "dbo".to_string())
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut columns: Vec<Box<dyn TableRow>> = vec![];
        while let Some(row) = rows.try_next().await? {
            columns.push(Box::new(Column {
                name: row.try_get("COLUMN_NAME")?,
                r#type: row.try_get("DATA_TYPE")?,
                null: row.try_get("IS_NULLABLE")?,
                default: row.try_get("COLUMN_DEFAULT")?,
                comment: None,
            }))
        }
        Ok(columns)
    }

    async fn get_constraints(
        &self,
        database: &Database,
        table: &Table,
    ) -> anyhow::Result<Vec<Box<dyn TableRow>>> {
        let query = format!(
            "
            USE {}
            SELECT
            TC.CONSTRAINT_NAME,
            KCU.COLUMN_NAME
            FROM
            INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS TC
            JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE AS KCU ON TC.CONSTRAINT_NAME = KCU.CONSTRAINT_NAME
            AND TC.TABLE_SCHEMA = KCU.TABLE_SCHEMA
            JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE AS CCU ON CCU.CONSTRAINT_NAME = TC.CONSTRAINT_NAME
            AND CCU.TABLE_SCHEMA = TC.TABLE_SCHEMA
            WHERE
            NOT TC.CONSTRAINT_TYPE = 'FOREIGN KEY'
            AND TC.TABLE_NAME = '{}'
            ",
            database.name,
            table.name
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut constraints: Vec<Box<dyn TableRow>> = vec![];
        while let Some(row) = rows.try_next().await? {
            constraints.push(Box::new(Constraint {
                name: row.try_get("CONSTRAINT_NAME")?,
                column_name: row.try_get("COLUMN_NAME")?,
            }))
        }
        Ok(constraints)
    }

    async fn get_foreign_keys(
        &self,
        database: &Database,
        table: &Table,
    ) -> anyhow::Result<Vec<Box<dyn TableRow>>> {
        let query = format!(
            "
            USE {}
            SELECT
            TC.CONSTRAINT_NAME,
            KCU.COLUMN_NAME,
            CCU.TABLE_NAME AS FOREIGN_TABLE_NAME,
            CCU.COLUMN_NAME AS FOREIGN_COLUMN_NAME
            FROM
            INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS TC
            JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE AS KCU ON TC.CONSTRAINT_NAME = KCU.CONSTRAINT_NAME
            AND TC.TABLE_SCHEMA = KCU.TABLE_SCHEMA
            JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE AS CCU ON CCU.CONSTRAINT_NAME = TC.CONSTRAINT_NAME
            AND CCU.TABLE_SCHEMA = TC.TABLE_SCHEMA
            WHERE
            TC.CONSTRAINT_TYPE = 'FOREIGN KEY'
            AND TC.TABLE_NAME = '{}'
        ",
            database.name,
            table.name
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut foreign_keys: Vec<Box<dyn TableRow>> = vec![];
        while let Some(row) = rows.try_next().await? {
            foreign_keys.push(Box::new(ForeignKey {
                name: row.try_get("CONSTRAINT_NAME")?,
                column_name: row.try_get("COLUMN_NAME")?,
                ref_table: row.try_get("FOREIGN_TABLE_NAME")?,
                ref_column: row.try_get("FOREIGN_COLUMN_NAME")?,
            }))
        }
        Ok(foreign_keys)
    }

    async fn get_indexes(
        &self,
        database: &Database,
        table: &Table,
    ) -> anyhow::Result<Vec<Box<dyn TableRow>>> {
        let query = format!(
            "
            USE {}
            SELECT
            ind.name AS INDEX_NAME,
			col.name AS COLUMN_NAME,
            ind.TYPE_DESC AS TYPE
            FROM SYS.INDEXES ind
			INNER JOIN 
			sys.index_columns ic ON  ind.object_id = ic.object_id and ind.index_id = ic.index_id 
			INNER JOIN 
			sys.columns col ON ic.object_id = col.object_id and ic.column_id = col.column_id
			INNER JOIN 
			sys.tables t ON ind.object_id = t.object_id
            WHERE t.name = '{}'
            ",
            database.name, table.name
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut foreign_keys: Vec<Box<dyn TableRow>> = vec![];
        while let Some(row) = rows.try_next().await? {
            foreign_keys.push(Box::new(Index {
                name: row.try_get("INDEX_NAME")?,
                column_name: row.try_get("COLUMN_NAME")?,
                r#type: row.try_get("TYPE")?,
            }))
        }
        Ok(foreign_keys)
    }

    async fn close(&self) {
        self.pool.close().await;
    }
}

fn convert_column_value_to_string(row: &MssqlRow, column: &MssqlColumn) -> anyhow::Result<String> {
    let column_name = column.name();
    if let Ok(value) = row.try_get(column_name) {
        let value: Option<String> = value;
        Ok(value.unwrap_or_else(|| "NULL".to_string()))
    } else if let Ok(value) = row.try_get(column_name) {
        let value: Option<i8> = value;
        Ok(get_or_null!(value))
    } else if let Ok(value) = row.try_get(column_name) {
        let value: Option<i16> = value;
        Ok(get_or_null!(value))
    } else if let Ok(value) = row.try_get(column_name) {
        let value: Option<i32> = value;
        Ok(get_or_null!(value))
    } else if let Ok(value) = row.try_get(column_name) {
        let value: Option<i64> = value;
        Ok(get_or_null!(value))
    } else if let Ok(value) = row.try_get(column_name) {
        let value: Option<f32> = value;
        Ok(get_or_null!(value))
    } else if let Ok(value) = row.try_get(column_name) {
        let value: Option<f64> = value;
        Ok(get_or_null!(value))
    } else if let Ok(value) = row.try_get(column_name) {
        let value: Option<bool> = value;
        Ok(get_or_null!(value))
    } else {
        anyhow::bail!(
            "column type not implemented: `{}` {}",
            column_name,
            column.type_info().clone().name()
        )
    }
}
