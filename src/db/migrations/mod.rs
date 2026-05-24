use turso::Connection;

mod migration_v1;

pub async fn execute_migrations(connection: &Connection) -> turso::Result<()> {
    let mut user_version = connection
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
        migration_v1::migrate(&connection).await?;

        user_version = 1;
    }

    Ok(())
}