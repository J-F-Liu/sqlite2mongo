[![Crates.io](https://img.shields.io/crates/v/sqlite2mongo.svg)](https://crates.io/crates/sqlite2mongo)

Import sqlite database to mongodb.

### Usage

```
USAGE:
    sqlite2mongo.exe [FLAGS] <sqlite-path> <mongodb-uri> <mongo-database>

FLAGS:
        --dry-run       Test reading sqlite data, do not create mongodb collection
    -h, --help          Prints help information
        --lower-camel   Convert field name to lower camel case
    -V, --version       Prints version information

ARGS:
    <sqlite-path>       Sqlite data file path
    <mongodb-uri>       Mongodb URI
    <mongo-database>    Database name to save the imported data
```

Example:

```
sqlite2mongo sqlite://D:/Database/mydb.db?mode=ro mongodb://localhost:27017 mydb --lower-camel
```

### Differences to [sqlitemongo](https://www.npmjs.com/package/sqlitemongo)

- New ObjectId is generated for \_id field.
- DateTime, Boolean field types are reserved.
- Supports dry-run and convert field name to mixed case.
