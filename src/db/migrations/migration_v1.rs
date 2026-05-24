use turso::{Connection};

pub async fn migrate(connection: &Connection) -> turso::Result<()> {
    connection
        .execute(
            r#"CREATE TABLE check_types (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL
                )"#,
            (),
        )
        .await?;

    connection
        .execute(
            r#"CREATE TABLE characters (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL
                )"#,
            (),
        )
        .await?;

    connection
        .execute(
            r#"CREATE TABLE abilities (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL
                )"#,
            (),
        )
        .await?;

    connection
        .execute(
            r#"CREATE TABLE checks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    character_id INTEGER NOT NULL,
                    check_type_id INTEGER NOT NULL,
                    ability_id INTEGER,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(character_id) REFERENCES characters(id),
                    FOREIGN KEY(check_type_id) REFERENCES check_types(id),
                    FOREIGN KEY(ability_id) REFERENCES abilities(id)
                )"#,
            (),
        )
        .await?;

    connection
        .execute(
            r#"CREATE TABLE rolls (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    check_id INTEGER NOT NULL,
                    dice_size INTEGER NOT NULL,
                    result INTEGER NOT NULL,
                    FOREIGN KEY(check_id) REFERENCES checks(id)
                )"#,
            (),
        )
        .await?;

    connection
        .execute(&format!("PRAGMA user_version = {}", 1), ())
        .await?;

    Ok(())
}