#![allow(clippy::disallowed_methods)]

use ehandle::{
    db::{MigrationErrorKind, SqlError, SqlErrorKind},
    lpm::LpmError,
    simple_e_fmt, try_execute, try_execute_prepared, ErrorCommons,
};
use min_sqlite3_sys::prelude::*;
use std::path::Path;

const INITIAL_VERSION: i64 = 0;

pub fn migrate_database_tables() -> Result<(), LpmError<SqlError>> {
    let db = Database::open(Path::new(super::CORE_DB_PATH))?;
    super::enable_foreign_keys(&db)?;

    let mut initial_version: i64 = INITIAL_VERSION;

    create_core_tables(&db, &mut initial_version)?;
    create_update_triggers_for_core_tables(&db, &mut initial_version)?;
    insert_defaults(&db, &mut initial_version)?;

    db.close();

    logger::success!("Db migrations are successfully completed.");

    Ok(())
}

fn set_migration_version(db: &Database, version: i64) -> Result<(), LpmError<SqlError>> {
    let statement = format!("PRAGMA user_version = {};", version);

    match db.execute(statement, super::SQL_NO_CALLBACK_FN) {
        Ok(_) => Ok(()),
        Err(_) => {
            Err(SqlErrorKind::MigrationError(MigrationErrorKind::VersionCouldNotSet).to_lpm_err())
        }
    }
}

fn can_migrate(db: &Database, version: i64) -> Result<bool, LpmError<SqlError>> {
    let statement = String::from("PRAGMA user_version;");

    let mut sql = db.prepare(statement.clone(), super::SQL_NO_CALLBACK_FN)?;
    try_execute_prepared!(
        sql,
        simple_e_fmt!("Failed executing SQL statement `{}`.", statement)
    );

    let db_user_version = sql.clone().get_data::<i64>(0)?;
    let result = version > db_user_version;
    sql.kill();
    Ok(result)
}

fn create_core_tables(db: &Database, version: &mut i64) -> Result<(), LpmError<SqlError>> {
    *version += 1;
    if !can_migrate(db, *version)? {
        logger::warning!("migration 'create_core_tables' already applied, skipping it.");
        return Ok(());
    }

    let statement = String::from(
        "
            /*
             * Statement of `checksum_kinds` table creation.
             * This table will hold the supported hashing algorithms
             * for the packages.
            */
            CREATE TABLE checksum_kinds (
               id            INTEGER    PRIMARY KEY    AUTOINCREMENT,
               kind          TEXT       NOT NULL       UNIQUE,
               created_at    TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP
            );

            /*
             * Statement of `package_kinds` table creation.
             * This table will hold the kind of packages to help
             * classify the packages installed in the system.
            */
            CREATE TABLE package_kinds (
               id            INTEGER    PRIMARY KEY    AUTOINCREMENT,
               kind          TEXT       NOT NULL       UNIQUE,
               created_at    TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP
            );

            /*
             * Statement of `repositories` table creation.
             * This table will hold the repository informations.
            */
            CREATE TABLE repositories (
               id               INTEGER    PRIMARY KEY    AUTOINCREMENT,
               name             TEXT       NOT NULL       UNIQUE,
               address          TEXT       NOT NULL,
               index_db_path    TEXT       NOT NULL,
               is_active        BOOLEAN    NOT NULL       CHECK(is_active IN (0, 1)),
               created_at       TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP,
               updated_at       TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP
            );

            /*
             * Statement of `packages` table creation.
             * This table will hold installed package informations.
            */
            CREATE TABLE packages (
               id                       INTEGER    PRIMARY KEY    AUTOINCREMENT,
               name                     TEXT       NOT NULL       UNIQUE,
               description              TEXT,
               maintainer               TEXT       NOT NULL,
               homepage                 TEXT,
               src_pkg_package_id       INTEGER,
               package_kind_id          INTEGER    NOT_NULL,
               installed_size           INTEGER    NOT_NULL,
               license                  TEXT,
               v_major                  INTEGER    NOT NULL,
               v_minor                  INTEGER    NOT NULL,
               v_patch                  INTEGER    NOT NULL,
               v_tag                    TEXT,
               v_readable               TEXT       NOT NULL,
               created_at               TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP,
               updated_at               TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP,

               FOREIGN KEY(src_pkg_package_id) REFERENCES packages(id),
               FOREIGN KEY(package_kind_id) REFERENCES package_kinds(id)
            );

            /*
             * Statement of `files` table creation.
             * This table will hold the information of files which are in the
             * packages.
            */
            CREATE TABLE files (
               id                  INTEGER    PRIMARY KEY    AUTOINCREMENT,
               name                TEXT       NOT NULL,
               absolute_path       TEXT       NOT NULL       UNIQUE,
               checksum            TEXT       NOT NULL,
               checksum_kind_id    INTEGER    NOT NULL,
               package_id          INTEGER    NOT NULL,
               created_at          TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP,

               FOREIGN KEY(package_id) REFERENCES packages(id) ON DELETE CASCADE,
               FOREIGN KEY(checksum_kind_id) REFERENCES checksum_kinds(id)
            );

            /*
             * Statement of `package_tags` table creation.
             * This table will hold the tag data which belongs to
             * packages.
            */
            CREATE TABLE package_tags (
               id                  INTEGER    PRIMARY KEY    AUTOINCREMENT,
               tag                 TEXT       NOT NULL,
               package_id          INTEGER    NOT NULL,
               created_at          TIMESTAMP  NOT NULL       DEFAULT CURRENT_TIMESTAMP,

               FOREIGN KEY(package_id) REFERENCES packages(id) ON DELETE CASCADE
            );

            /*
             * Statement of `modules` table creation.
             * This table will hold module informations.
            */
            CREATE TABLE modules (
               id                       INTEGER    PRIMARY KEY    AUTOINCREMENT,
               name                     TEXT       NOT NULL       UNIQUE,
               dylib_path               TEXT       NOT NULL
            );
        ",
    );

    try_execute!(db, statement);
    set_migration_version(db, *version)?;
    logger::info!("'create_core_tables' migration is finished.");

    Ok(())
}

fn create_update_triggers_for_core_tables(
    db: &Database,
    version: &mut i64,
) -> Result<(), LpmError<SqlError>> {
    *version += 1;
    if !can_migrate(db, *version)? {
        logger::warning!(
            "migration 'create_update_triggers_for_core_tables' already applied, skipping it."
        );
        return Ok(());
    }

    let statement = String::from(
        "
            /*
             * Statement of `repositories` update trigger.
             * This will allow automatic `updated_at` updates whenever an UPDATE
             * operation happens on the table.
            */
            CREATE TRIGGER repositories_update_trigger
                AFTER UPDATE ON repositories
            BEGIN
                UPDATE repositories SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
            END;

            /*
             * Statement of `packages` update trigger.
             * This will allow automatic `updated_at` updates whenever an UPDATE
             * operation happens on the table.
            */
            CREATE TRIGGER packages_update_trigger
                AFTER UPDATE ON packages
            BEGIN
                UPDATE packages SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
            END;
        ",
    );

    try_execute!(db, statement);
    set_migration_version(db, *version)?;
    logger::info!("'create_update_triggers_for_core_tables' migration is finished.");

    Ok(())
}

fn insert_defaults(db: &Database, version: &mut i64) -> Result<(), LpmError<SqlError>> {
    *version += 1;
    if !can_migrate(db, *version)? {
        logger::warning!("migration 'insert_defaults' already applied, skipping it.");
        return Ok(());
    }

    let statement = String::from(
        "
            INSERT INTO checksum_kinds
                (kind)
            VALUES
                ('md5'),
                ('sha256'),
                ('sha512');",
    );

    try_execute!(db, statement);
    set_migration_version(db, *version)?;
    logger::info!("'insert_defaults' migration is finished.");

    Ok(())
}
