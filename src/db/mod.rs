use turso::{Builder, Connection, Error};

pub struct DBHandle {
    connection: Connection,
}

impl DBHandle {
    pub async fn init() -> Result<Self, Error> {
        let conn = Builder::new_local("jdr_rolls.db")
            .build()
            .await?
            .connect()?;

        let res = DBHandle { connection: conn };

        res.execute_migrations().await?;

        Ok(res)
    }

    async fn execute_migrations(&self) -> turso::Result<()> {
        let mut user_version = self
            .connection
            .query("PRAGMA user_version", ())
            .await?
            .next()
            .await?
            .unwrap()
            .get_value(0)?
            .as_integer()
            .unwrap_or(&0)
            .to_owned();

        if user_version == 0 {
            self.connection
                .execute(
                    r#"CREATE TABLE check_types (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL
                )"#,
                    (),
                )
                .await?;

            self.connection
                .execute(
                    r#"CREATE TABLE characters (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL
                )"#,
                    (),
                )
                .await?;

            self.connection
                .execute(
                    r#"CREATE TABLE abilities (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL
                )"#,
                    (),
                )
                .await?;

            self.connection
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

            self.connection
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

            user_version = 1;
            self.connection
                .execute(&format!("PRAGMA user_version = {}", user_version), ())
                .await?;
        }

        Ok(())
    }
}
