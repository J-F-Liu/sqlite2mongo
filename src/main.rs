use anyhow::Result;
use heck::ToLowerCamelCase;
use mongodb::{bson::*, Client, Database};
use sqlx::{
    sqlite::*,
    types::chrono::{DateTime, Utc},
    *,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "sqlite2mongo")]
struct Args {
    /// Sqlite data file path
    sqlite_path: String,
    /// Mongodb URI
    mongodb_uri: String,
    /// Database name to save the imported data
    mongo_database: String,
    /// Test reading sqlite data, do not create mongodb collection
    #[structopt(long)]
    dry_run: bool,
    /// Convert field name to lower camel case
    #[structopt(long)]
    lower_camel: bool,
}

#[async_std::main]
async fn main() -> Result<()> {
    let args = Args::from_args();
    let mut conn = SqliteConnection::connect(&args.sqlite_path).await?;
    let client = Client::with_uri_str(&args.mongodb_uri).await?;
    let database = client.database(&args.mongo_database);

    if database.list_collection_names(None).await?.len() > 0 {
        println!("Confirm delete the exiting database (type 'yes' to continue)?");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim_end() == "yes" {
            database.drop(None).await?;
        } else {
            println!("Abort");
            return Ok(());
        }
    }

    let tables = get_tables(&mut conn).await?;
    for table in tables {
        println!("Table: {}", &table);
        let row_count =
            create_collection(&mut conn, &database, table, args.lower_camel, args.dry_run).await?;
        println!("Imported {} rows.", row_count);
    }
    Ok(())
}

async fn get_tables(conn: &mut SqliteConnection) -> Result<Vec<String>> {
    let tables = sqlx::query(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%';",
    )
    .fetch_all(conn)
    .await?;
    let table_names = tables
        .into_iter()
        .map(|table| table.get::<String, _>("name"))
        .collect();
    Ok(table_names)
}

async fn create_collection(
    conn: &mut SqliteConnection,
    database: &Database,
    table: String,
    lower_camel: bool,
    dry_run: bool,
) -> Result<usize> {
    let mut row_count = 0;
    let collection = database.collection(table.as_str());
    let rows = sqlx::query(&format!("SELECT * FROM {};", table))
        .fetch_all(conn)
        .await?;
    for row in rows {
        let doc = create_mongo_document(row, lower_camel);
        if !dry_run {
            collection.insert_one(doc, None).await?;
        }
        row_count += 1;
    }
    Ok(row_count)
}

fn create_mongo_document(row: SqliteRow, lower_camel: bool) -> Document {
    let mut doc = Document::new();
    for column in row.columns() {
        let field = column.name();
        let raw_value = row.try_get_raw(field).unwrap();
        let bson_value = if raw_value.is_null() {
            Bson::Null
        } else {
            let value = get_field_value(&row, field, column.type_info().name());
            if value == Bson::Null {
                get_field_value(&row, field, raw_value.type_info().name())
            } else {
                value
            }
        };
        if lower_camel {
            doc.insert(field.to_lower_camel_case(), bson_value);
        } else {
            doc.insert(field, bson_value);
        }
    }
    doc
}

fn get_field_value(row: &SqliteRow, field: &str, type_name: &str) -> Bson {
    match type_name {
        "BOOLEAN" => {
            if let Ok(value) = row.try_get::<bool, _>(field) {
                Bson::Boolean(value)
            } else {
                let value = row.get::<String, _>(field);
                if value.as_str() == "t" {
                    Bson::Boolean(true)
                } else {
                    Bson::Boolean(false)
                }
            }
        }
        "INTEGER" => Bson::Int64(row.get::<i64, _>(field)),
        "REAL" => Bson::Double(row.get::<f64, _>(field)),
        "TEXT" => Bson::String(row.get::<String, _>(field)),
        "DATETIME" => Bson::DateTime(row.get::<DateTime<Utc>, _>(field).into()),
        "BLOB" => Bson::Binary(Binary {
            subtype: mongodb::bson::spec::BinarySubtype::Generic,
            bytes: row.get::<Vec<u8>, _>(field),
        }),
        "NULL" => Bson::Null,
        _ => unimplemented!("Column type {}", type_name),
    }
}
