mod migrations;

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

        migrations::execute_migrations(&conn).await?;

        let res = DBHandle { connection: conn };

        Ok(res)
    }
}
