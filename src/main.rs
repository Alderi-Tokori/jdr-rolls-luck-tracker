mod db;

#[tokio::main]
async fn main() -> turso::Result<()> {
    let database = db::DBHandle::init().await?;

    Ok(())
}
