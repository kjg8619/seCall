use anyhow::Result;
use secall_core::{
    store::{get_default_db_path, Database},
    vault::Config,
};

pub async fn run(all: bool, batch_size: Option<usize>) -> Result<()> {
    let config = Config::load_or_default();
    let db_path = get_default_db_path();
    let db = Database::open(&db_path)?;

    let vector_indexer = secall_core::search::vector::create_vector_indexer(&config).await;
    let Some(indexer) = vector_indexer else {
        eprintln!("No embedding backend available.");
        eprintln!("  1. Download model: secall model download");
        eprintln!("  2. Check config: [embedding] section in config.toml");
        return Ok(());
    };

    let session_ids = if all {
        db.list_all_session_ids()?
    } else {
        db.find_sessions_without_vectors()?
    };

    if session_ids.is_empty() {
        println!("All sessions already embedded.");
        return Ok(());
    }

    let total = session_ids.len();
    let _batch_size = batch_size.unwrap_or(32);
    println!("Embedding {} session(s)...", total);

    for (i, sid) in session_ids.iter().enumerate() {
        let session = match db.get_session_for_embedding(sid) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "  [{}/{}] {} — failed to load: {e}",
                    i + 1,
                    total,
                    &sid[..sid.len().min(8)]
                );
                continue;
            }
        };
        match indexer.index_session(&db, &session).await {
            Ok(stats) => eprintln!(
                "  [{}/{}] {} — {} chunks",
                i + 1,
                total,
                &sid[..sid.len().min(8)],
                stats.chunks_embedded
            ),
            Err(e) => eprintln!(
                "  [{}/{}] {} — embedding failed: {e}",
                i + 1,
                total,
                &sid[..sid.len().min(8)]
            ),
        }
    }

    println!("Done.");
    Ok(())
}
