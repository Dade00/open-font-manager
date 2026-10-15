use crate::{
    errors::AppError,
    models::{
        Collection, FontCategory, FontItem, FontPage, FontQuery, FontSource, LibraryTab,
        ScanReport, ScanRoot, UpsertOutcome,
    },
};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const SCHEMA_VERSION: i64 = 5;
pub fn open_connection(path: &Path) -> Result<Connection, AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::Storage {
            path: parent.to_path_buf(),
            reason: e.to_string(),
        })?;
    }
    let c = Connection::open(path)?;
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;")?;
    Ok(c)
}
pub fn migrate(path: &Path) -> Result<(), AppError> {
    let mut c = open_connection(path)?;
    c.execute_batch("PRAGMA foreign_keys=OFF;")?;
    let tx = c.transaction()?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY,applied_at_unix_ms INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS scan_roots(path TEXT PRIMARY KEY,added_at_unix_ms INTEGER NOT NULL,last_scanned_at_unix_ms INTEGER,is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0,1)));
CREATE TABLE IF NOT EXISTS fonts(id TEXT PRIMARY KEY,family TEXT NOT NULL,subfamily TEXT NOT NULL,full_name TEXT NOT NULL,weight INTEGER NOT NULL CHECK(weight BETWEEN 1 AND 1000),is_italic INTEGER NOT NULL CHECK(is_italic IN (0,1)),file_path TEXT NOT NULL,source TEXT NOT NULL CHECK(source IN ('local','google')),source_family_id TEXT,source_variant_id TEXT,category TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0,1)),is_favorite INTEGER NOT NULL DEFAULT 0 CHECK(is_favorite IN (0,1)),file_size INTEGER NOT NULL,file_modified_unix_ms INTEGER NOT NULL,missing_on_disk INTEGER NOT NULL DEFAULT 0 CHECK(missing_on_disk IN (0,1)),created_at_unix_ms INTEGER NOT NULL,updated_at_unix_ms INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS collections(id INTEGER PRIMARY KEY,name TEXT NOT NULL UNIQUE COLLATE NOCASE,created_at_unix_ms INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS collection_items(collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,font_id TEXT NOT NULL REFERENCES fonts(id) ON DELETE CASCADE,PRIMARY KEY(collection_id,font_id));")?;
    let done: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=?1)",
        [SCHEMA_VERSION],
        |r| r.get(0),
    )?;
    if !done {
        let font_table_sql: Option<String> = tx
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='fonts'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if font_table_sql
            .as_deref()
            .is_some_and(|sql| sql.contains("file_path TEXT NOT NULL UNIQUE"))
        {
            tx.execute_batch(
                "ALTER TABLE collection_items RENAME TO collection_items_legacy;
                 ALTER TABLE fonts RENAME TO fonts_legacy;
                 CREATE TABLE fonts(id TEXT PRIMARY KEY,family TEXT NOT NULL,subfamily TEXT NOT NULL,full_name TEXT NOT NULL,weight INTEGER NOT NULL CHECK(weight BETWEEN 1 AND 1000),is_italic INTEGER NOT NULL CHECK(is_italic IN (0,1)),file_path TEXT NOT NULL,source TEXT NOT NULL CHECK(source IN ('local','google')),source_family_id TEXT,source_variant_id TEXT,category TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0,1)),is_favorite INTEGER NOT NULL DEFAULT 0 CHECK(is_favorite IN (0,1)),file_size INTEGER NOT NULL,file_modified_unix_ms INTEGER NOT NULL,missing_on_disk INTEGER NOT NULL DEFAULT 0 CHECK(missing_on_disk IN (0,1)),created_at_unix_ms INTEGER NOT NULL,updated_at_unix_ms INTEGER NOT NULL);
                 CREATE TABLE collection_items(collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,font_id TEXT NOT NULL REFERENCES fonts(id) ON DELETE CASCADE,PRIMARY KEY(collection_id,font_id));
                 INSERT INTO fonts SELECT * FROM fonts_legacy;
                 INSERT INTO collection_items SELECT * FROM collection_items_legacy;
                 DROP TABLE collection_items_legacy;
                 DROP TABLE fonts_legacy;",
            )?;
        }
        if !font_table_sql
            .as_deref()
            .is_some_and(|sql| sql.contains("'fontsource'"))
        {
            tx.execute_batch("CREATE TABLE fonts_new(id TEXT PRIMARY KEY,family TEXT NOT NULL,subfamily TEXT NOT NULL,full_name TEXT NOT NULL,weight INTEGER NOT NULL CHECK(weight BETWEEN 1 AND 1000),is_italic INTEGER NOT NULL CHECK(is_italic IN (0,1)),file_path TEXT NOT NULL,source TEXT NOT NULL CHECK(source IN ('local','google','fontsource','fontshare','nerd-fonts')),source_family_id TEXT,source_variant_id TEXT,category TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0,1)),is_favorite INTEGER NOT NULL DEFAULT 0 CHECK(is_favorite IN (0,1)),file_size INTEGER NOT NULL,file_modified_unix_ms INTEGER NOT NULL,missing_on_disk INTEGER NOT NULL DEFAULT 0 CHECK(missing_on_disk IN (0,1)),created_at_unix_ms INTEGER NOT NULL,updated_at_unix_ms INTEGER NOT NULL);
INSERT INTO fonts_new SELECT id,family,subfamily,full_name,weight,is_italic,file_path,source,source_family_id,source_variant_id,category,is_active,is_favorite,file_size,file_modified_unix_ms,missing_on_disk,created_at_unix_ms,updated_at_unix_ms FROM fonts;
DROP TABLE fonts;
ALTER TABLE fonts_new RENAME TO fonts;
CREATE INDEX IF NOT EXISTS idx_fonts_family ON fonts(family COLLATE NOCASE);CREATE INDEX IF NOT EXISTS idx_fonts_active ON fonts(is_active);CREATE INDEX IF NOT EXISTS idx_fonts_favorite ON fonts(is_favorite);CREATE INDEX IF NOT EXISTS idx_fonts_source ON fonts(source);")?;
        }
        for (column, definition) in [
            ("last_discovered_candidates", "INTEGER NOT NULL DEFAULT 0"),
            ("last_imported", "INTEGER NOT NULL DEFAULT 0"),
            ("last_updated", "INTEGER NOT NULL DEFAULT 0"),
            ("last_skipped_unchanged", "INTEGER NOT NULL DEFAULT 0"),
            ("last_skipped_unsupported", "INTEGER NOT NULL DEFAULT 0"),
            ("last_failed", "INTEGER NOT NULL DEFAULT 0"),
            ("last_error", "TEXT"),
        ] {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('scan_roots') WHERE name=?1)",
                [column],
                |row| row.get(0),
            )?;
            if !exists {
                tx.execute_batch(&format!(
                    "ALTER TABLE scan_roots ADD COLUMN {column} {definition}"
                ))?;
            }
        }
        if tx.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('fonts') WHERE name='is_variable'",
            [],
            |r| r.get::<_, i64>(0),
        )? == 0
        {
            tx.execute_batch("ALTER TABLE fonts ADD COLUMN is_variable INTEGER NOT NULL DEFAULT 0 CHECK(is_variable IN (0,1));")?;
        }
        tx.execute(
            "INSERT INTO schema_migrations VALUES(?1,?2)",
            params![SCHEMA_VERSION, now_ms()],
        )?;
    }
    tx.commit()?;
    c.execute_batch("PRAGMA foreign_keys=ON;")?;
    Ok(())
}
pub fn upsert_font(path: &Path, item: &FontItem) -> Result<UpsertOutcome, AppError> {
    if !(1..=1000).contains(&item.weight) {
        return Err(AppError::InvalidInput(
            "Font weight must be between 1 and 1000.".into(),
        ));
    }
    let c = open_connection(path)?;
    let old=c.query_row("SELECT id,family,subfamily,full_name,weight,is_italic,file_path,source,source_family_id,source_variant_id,category,is_active,is_favorite,file_size,file_modified_unix_ms,missing_on_disk,is_variable FROM fonts WHERE id=?1 OR (file_path=?2 AND source_variant_id IS ?3) LIMIT 1",params![item.id,item.file_path.to_string_lossy(),item.source_variant_id],row_font).optional()?;
    if old.as_ref().is_some_and(|x| same_metadata(x, item)) {
        return Ok(UpsertOutcome::Unchanged);
    }
    let now = now_ms();
    c.execute("INSERT INTO fonts(id,family,subfamily,full_name,weight,is_italic,file_path,source,source_family_id,source_variant_id,category,is_active,is_favorite,file_size,file_modified_unix_ms,missing_on_disk,is_variable,created_at_unix_ms,updated_at_unix_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,0,?16,?17,?17)
    ON CONFLICT(id) DO UPDATE SET family=excluded.family,subfamily=excluded.subfamily,full_name=excluded.full_name,weight=excluded.weight,is_italic=excluded.is_italic,file_path=excluded.file_path,source=excluded.source,source_family_id=excluded.source_family_id,source_variant_id=excluded.source_variant_id,category=excluded.category,is_variable=excluded.is_variable,file_size=excluded.file_size,file_modified_unix_ms=excluded.file_modified_unix_ms,missing_on_disk=0,updated_at_unix_ms=excluded.updated_at_unix_ms",params![item.id,item.family,item.subfamily,item.full_name,item.weight,item.is_italic,item.file_path.to_string_lossy(),source_str(item.source),item.source_family_id,item.source_variant_id,category_str(item.category),item.is_active,item.is_favorite,i64::try_from(item.file_size).unwrap_or(i64::MAX),item.file_modified_unix_ms,item.is_variable,now])?;
    Ok(if old.is_some() {
        UpsertOutcome::Updated
    } else {
        UpsertOutcome::Inserted
    })
}
fn font_query_filters(q: &FontQuery) -> (String, Vec<Value>) {
    let (mut conditions, mut args) = (vec!["1=1".to_string()], Vec::<Value>::new());
    match q.tab {
        LibraryTab::Local => {}
        LibraryTab::Google => {
            conditions.push("f.source='google'".into());
        }
        LibraryTab::Active => conditions.push("f.is_active=1".into()),
        LibraryTab::Favorites => conditions.push("f.is_favorite=1".into()),
    }
    if !q.search.trim().is_empty() {
        conditions.push("(f.family LIKE ? ESCAPE '\\' COLLATE NOCASE OR f.subfamily LIKE ? ESCAPE '\\' COLLATE NOCASE OR f.full_name LIKE ? ESCAPE '\\' COLLATE NOCASE)".into());
        let x = format!("%{}%", escape_like(q.search.trim()));
        args.extend([
            Value::Text(x.clone()),
            Value::Text(x.clone()),
            Value::Text(x),
        ]);
    }
    if q.category != FontCategory::All {
        conditions.push("f.category=?".into());
        args.push(Value::Text(category_str(q.category).into()));
    }
    if let Some(id) = q.collection_id {
        conditions.push("EXISTS(SELECT 1 FROM collection_items ci WHERE ci.font_id=f.id AND ci.collection_id=?)".into());
        args.push(Value::Integer(id));
    }
    (conditions.join(" AND "), args)
}

pub fn list_fonts(path: &Path, q: FontQuery) -> Result<FontPage, AppError> {
    if !(1..=100).contains(&q.page_size) {
        return Err(AppError::InvalidInput(
            "Font page size must be between 1 and 100.".into(),
        ));
    }
    let c = open_connection(path)?;
    let (clause, mut args) = font_query_filters(&q);
    let total: i64 = c.query_row(
        &format!("SELECT COUNT(*) FROM fonts f WHERE {clause}"),
        params_from_iter(args.iter()),
        |r| r.get(0),
    )?;
    args.extend([
        Value::Integer(i64::from(q.page_size)),
        Value::Integer(i64::from(q.page).saturating_mul(i64::from(q.page_size))),
    ]);
    let mut s=c.prepare(&format!("SELECT f.id,f.family,f.subfamily,f.full_name,f.weight,f.is_italic,f.file_path,f.source,f.source_family_id,f.source_variant_id,f.category,f.is_active,f.is_favorite,f.file_size,f.file_modified_unix_ms,f.missing_on_disk,f.is_variable FROM fonts f WHERE {clause} ORDER BY f.family COLLATE NOCASE,f.weight,f.subfamily COLLATE NOCASE LIMIT ? OFFSET ?"))?;
    let items = s
        .query_map(params_from_iter(args.iter()), row_font)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FontPage {
        items,
        total: usize::try_from(total).unwrap_or(usize::MAX),
        page: q.page,
        page_size: q.page_size,
    })
}

/// Streams candidates once, retaining only the requested page after live installation checks.
pub fn list_installed_fonts(
    path: &Path,
    mut query: FontQuery,
    roots: &[std::path::PathBuf],
) -> Result<FontPage, AppError> {
    if !(1..=100).contains(&query.page_size) {
        return Err(AppError::InvalidInput(
            "Font page size must be between 1 and 100.".into(),
        ));
    }
    query.tab = LibraryTab::Local;
    let (clause, args) = font_query_filters(&query);
    let connection = open_connection(path)?;
    let mut statement = connection.prepare(&format!("SELECT f.id,f.family,f.subfamily,f.full_name,f.weight,f.is_italic,f.file_path,f.source,f.source_family_id,f.source_variant_id,f.category,f.is_active,f.is_favorite,f.file_size,f.file_modified_unix_ms,f.missing_on_disk,f.is_variable FROM fonts f WHERE {clause} ORDER BY f.family COLLATE NOCASE,f.weight,f.subfamily COLLATE NOCASE"))?;
    let start = (query.page as usize).saturating_mul(query.page_size as usize);
    let mut total = 0;
    let mut items = Vec::new();
    for row in statement.query_map(params_from_iter(args.iter()), row_font)? {
        let mut font = row?;
        font.is_installed = font.is_active
            || crate::installed_fonts::is_installed_font_path(&font.file_path, roots);
        if font.is_installed {
            if total >= start && items.len() < query.page_size as usize {
                items.push(font);
            }
            total += 1;
        }
    }
    Ok(FontPage {
        items,
        total,
        page: query.page,
        page_size: query.page_size,
    })
}

pub fn font_by_id(path: &Path, id: &str) -> Result<FontItem, AppError> {
    open_connection(path)?
        .query_row(
            "SELECT id,family,subfamily,full_name,weight,is_italic,file_path,source,source_family_id,source_variant_id,category,is_active,is_favorite,file_size,file_modified_unix_ms,missing_on_disk,is_variable FROM fonts WHERE id=?1",
            [id],
            row_font,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Font '{id}' was not found.")))
}

pub fn list_active_fonts(path: &Path) -> Result<Vec<FontItem>, AppError> {
    let mut page = 0;
    let mut active = Vec::new();
    loop {
        let result = list_fonts(
            path,
            FontQuery {
                tab: LibraryTab::Active,
                search: String::new(),
                category: FontCategory::All,
                collection_id: None,
                page,
                page_size: 100,
            },
        )?;
        active.extend(result.items);
        if active.len() >= result.total {
            return Ok(active);
        }
        page += 1;
    }
}

pub fn count_active_fonts(path: &Path) -> Result<usize, AppError> {
    let connection = open_connection(path)?;
    let mut statement = connection
        .prepare("SELECT file_path FROM fonts WHERE is_active=1 AND missing_on_disk=0")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut count = 0;
    for row in rows {
        if Path::new(&row?).is_file() {
            count += 1;
        }
    }
    Ok(count)
}

pub fn set_font_active(p: &Path, id: &str, v: bool) -> Result<(), AppError> {
    update_flag(p, id, "is_active", v)
}
pub fn set_font_favorite(p: &Path, id: &str, v: bool) -> Result<(), AppError> {
    update_flag(p, id, "is_favorite", v)
}
fn update_flag(p: &Path, id: &str, col: &str, v: bool) -> Result<(), AppError> {
    if open_connection(p)?.execute(
        &format!("UPDATE fonts SET {col}=?1,updated_at_unix_ms=?2 WHERE id=?3"),
        params![v, now_ms(), id],
    )? == 0
    {
        return Err(AppError::NotFound(format!("Font '{id}' was not found.")));
    }
    Ok(())
}
pub fn mark_root_missing_files(p: &Path, root: &Path, present: &[PathBuf]) -> Result<(), AppError> {
    let c = open_connection(p)?;
    let root = norm(root);
    let present: std::collections::HashSet<_> = present.iter().map(|x| norm(x)).collect();
    let mut s =
        c.prepare("SELECT id,file_path FROM fonts WHERE source='local' AND missing_on_disk=0")?;
    let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut gone = Vec::new();
    for row in rows {
        let (id, path) = row?;
        let path = norm(Path::new(&path));
        if (path == root || path.starts_with(&root)) && !present.contains(&path) {
            gone.push(id)
        }
    }
    drop(s);
    let tx = c.unchecked_transaction()?;
    for id in gone {
        tx.execute(
            "UPDATE fonts SET missing_on_disk=1,updated_at_unix_ms=?1 WHERE id=?2",
            params![now_ms(), id],
        )?;
    }
    tx.commit()?;
    Ok(())
}
pub fn add_scan_root(p: &Path, root: &Path) -> Result<(), AppError> {
    open_connection(p)?.execute("INSERT INTO scan_roots(path,added_at_unix_ms,is_enabled) VALUES(?1,?2,1) ON CONFLICT(path) DO UPDATE SET is_enabled=1",params![root.to_string_lossy(),now_ms()])?;
    Ok(())
}
pub fn finish_scan_root(p: &Path, root: &Path) -> Result<(), AppError> {
    let connection = open_connection(p)?;
    if connection.execute(
        "UPDATE scan_roots SET last_scanned_at_unix_ms=?1 WHERE path=?2",
        params![now_ms(), root.to_string_lossy()],
    )? == 0
    {
        return Err(AppError::NotFound(format!(
            "Scan root '{}' was not found.",
            root.display()
        )));
    }
    Ok(())
}
pub fn finish_scan_root_report(p: &Path, report: &ScanReport) -> Result<(), AppError> {
    let last_error = report.failed.first().map(|issue| {
        format!("{}: {}", issue.path.display(), issue.reason)
            .chars()
            .take(1000)
            .collect::<String>()
    });
    let changed = open_connection(p)?.execute(
        "UPDATE scan_roots SET last_scanned_at_unix_ms=?1,last_discovered_candidates=?2,last_imported=?3,last_updated=?4,last_skipped_unchanged=?5,last_skipped_unsupported=?6,last_failed=?7,last_error=?8 WHERE path=?9",
        params![
            now_ms(),
            sql_count(report.discovered_candidates),
            sql_count(report.imported),
            sql_count(report.updated),
            sql_count(report.skipped_unchanged),
            sql_count(report.skipped_unsupported),
            sql_count(report.failed.len()),
            last_error,
            report.root.to_string_lossy(),
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!(
            "Scan root '{}' was not found.",
            report.root.display()
        )));
    }
    Ok(())
}
pub fn record_scan_root_failure(p: &Path, root: &Path, error: &str) -> Result<(), AppError> {
    let message = error.chars().take(1000).collect::<String>();
    let changed = open_connection(p)?.execute(
        "UPDATE scan_roots SET last_scanned_at_unix_ms=?1,last_discovered_candidates=0,last_imported=0,last_updated=0,last_skipped_unchanged=0,last_skipped_unsupported=0,last_failed=1,last_error=?2 WHERE path=?3",
        params![now_ms(), message, root.to_string_lossy()],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!(
            "Scan root '{}' was not found.",
            root.display()
        )));
    }
    Ok(())
}
pub fn set_scan_root_enabled(p: &Path, root: &Path, enabled: bool) -> Result<(), AppError> {
    if open_connection(p)?.execute(
        "UPDATE scan_roots SET is_enabled=?1 WHERE path=?2",
        params![enabled, root.to_string_lossy()],
    )? == 0
    {
        return Err(AppError::NotFound(format!(
            "Scan root '{}' was not found.",
            root.display()
        )));
    }
    Ok(())
}
pub fn remove_scan_root(p: &Path, root: &Path) -> Result<(), AppError> {
    if open_connection(p)?.execute(
        "DELETE FROM scan_roots WHERE path=?1",
        [root.to_string_lossy().as_ref()],
    )? == 0
    {
        return Err(AppError::NotFound(format!(
            "Scan root '{}' was not found.",
            root.display()
        )));
    }
    Ok(())
}
pub fn list_scan_roots(p: &Path) -> Result<Vec<ScanRoot>, AppError> {
    let c = open_connection(p)?;
    let mut s=c.prepare("SELECT path,added_at_unix_ms,last_scanned_at_unix_ms,is_enabled,last_discovered_candidates,last_imported,last_updated,last_skipped_unchanged,last_skipped_unsupported,last_failed,last_error FROM scan_roots ORDER BY path COLLATE NOCASE")?;
    let rows = s
        .query_map([], |r| {
            Ok(ScanRoot {
                path: PathBuf::from(r.get::<_, String>(0)?),
                added_at_unix_ms: r.get(1)?,
                last_scanned_at_unix_ms: r.get(2)?,
                is_enabled: r.get(3)?,
                last_discovered_candidates: usize::try_from(r.get::<_, i64>(4)?)
                    .unwrap_or(usize::MAX),
                last_imported: usize::try_from(r.get::<_, i64>(5)?).unwrap_or(usize::MAX),
                last_updated: usize::try_from(r.get::<_, i64>(6)?).unwrap_or(usize::MAX),
                last_skipped_unchanged: usize::try_from(r.get::<_, i64>(7)?).unwrap_or(usize::MAX),
                last_skipped_unsupported: usize::try_from(r.get::<_, i64>(8)?)
                    .unwrap_or(usize::MAX),
                last_failed: usize::try_from(r.get::<_, i64>(9)?).unwrap_or(usize::MAX),
                last_error: r.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
pub fn create_collection(p: &Path, name: &str) -> Result<Collection, AppError> {
    let name = valid_name(name)?;
    let c = open_connection(p)?;
    c.execute(
        "INSERT INTO collections(name,created_at_unix_ms) VALUES(?1,?2)",
        params![name, now_ms()],
    )?;
    Ok(Collection {
        id: c.last_insert_rowid(),
        name,
        font_count: 0,
    })
}
pub fn rename_collection(p: &Path, id: i64, name: &str) -> Result<(), AppError> {
    let name = valid_name(name)?;
    if open_connection(p)?.execute(
        "UPDATE collections SET name=?1 WHERE id=?2",
        params![name, id],
    )? == 0
    {
        return Err(AppError::NotFound(format!(
            "Collection '{id}' was not found."
        )));
    }
    Ok(())
}
pub fn delete_collection(p: &Path, id: i64) -> Result<(), AppError> {
    if open_connection(p)?.execute("DELETE FROM collections WHERE id=?1", [id])? == 0 {
        return Err(AppError::NotFound(format!(
            "Collection '{id}' was not found."
        )));
    }
    Ok(())
}
pub fn add_font_to_collection(p: &Path, cid: i64, fid: &str) -> Result<(), AppError> {
    open_connection(p)?.execute(
        "INSERT OR IGNORE INTO collection_items(collection_id,font_id) VALUES(?1,?2)",
        params![cid, fid],
    )?;
    Ok(())
}
pub fn remove_font_from_collection(p: &Path, cid: i64, fid: &str) -> Result<(), AppError> {
    open_connection(p)?.execute(
        "DELETE FROM collection_items WHERE collection_id=?1 AND font_id=?2",
        params![cid, fid],
    )?;
    Ok(())
}
pub fn list_collections(p: &Path) -> Result<Vec<Collection>, AppError> {
    let c = open_connection(p)?;
    let mut s=c.prepare("SELECT c.id,c.name,COUNT(ci.font_id) FROM collections c LEFT JOIN collection_items ci ON ci.collection_id=c.id GROUP BY c.id,c.name ORDER BY c.name COLLATE NOCASE")?;
    let rows = s
        .query_map([], |r| {
            Ok(Collection {
                id: r.get(0)?,
                name: r.get(1)?,
                font_count: usize::try_from(r.get::<_, i64>(2)?).unwrap_or(usize::MAX),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
pub fn delete_font(p: &Path, id: &str) -> Result<(), AppError> {
    if open_connection(p)?.execute("DELETE FROM fonts WHERE id=?1", [id])? == 0 {
        return Err(AppError::NotFound(format!("Font '{id}' was not found.")));
    }
    Ok(())
}
fn row_font(r: &rusqlite::Row<'_>) -> rusqlite::Result<FontItem> {
    let source: String = r.get(7)?;
    let cat: String = r.get(10)?;
    let size: i64 = r.get(13)?;
    Ok(FontItem {
        id: r.get(0)?,
        family: r.get(1)?,
        subfamily: r.get(2)?,
        full_name: r.get(3)?,
        weight: r.get(4)?,
        is_italic: r.get(5)?,
        file_path: PathBuf::from(r.get::<_, String>(6)?),
        source: match source.as_str() {
            "google" => FontSource::Google,
            "fontsource" => FontSource::Fontsource,
            "fontshare" => FontSource::Fontshare,
            "nerd-fonts" => FontSource::NerdFonts,
            _ => FontSource::Local,
        },
        source_family_id: r.get(8)?,
        source_variant_id: r.get(9)?,
        category: parse_cat(&cat),
        is_active: r.get(11)?,
        is_installed: r.get(11)?,
        is_favorite: r.get(12)?,
        file_size: u64::try_from(size).unwrap_or(0),
        file_modified_unix_ms: r.get(14)?,
        missing_on_disk: r.get(15)?,
        is_variable: r.get(16)?,
    })
}
fn same_metadata(a: &FontItem, b: &FontItem) -> bool {
    a.id == b.id
        && a.family == b.family
        && a.subfamily == b.subfamily
        && a.full_name == b.full_name
        && a.weight == b.weight
        && a.is_italic == b.is_italic
        && a.file_path == b.file_path
        && a.source == b.source
        && a.source_family_id == b.source_family_id
        && a.source_variant_id == b.source_variant_id
        && a.category == b.category
        && a.is_variable == b.is_variable
        && a.file_size == b.file_size
        && a.file_modified_unix_ms == b.file_modified_unix_ms
        && !a.missing_on_disk
}
fn source_str(x: FontSource) -> &'static str {
    match x {
        FontSource::Local => "local",
        FontSource::Google => "google",
        FontSource::Fontsource => "fontsource",
        FontSource::Fontshare => "fontshare",
        FontSource::NerdFonts => "nerd-fonts",
    }
}
fn category_str(x: FontCategory) -> &'static str {
    match x {
        FontCategory::All | FontCategory::Unknown => "unknown",
        FontCategory::SansSerif => "sans-serif",
        FontCategory::Serif => "serif",
        FontCategory::Display => "display",
        FontCategory::Handwriting => "handwriting",
        FontCategory::Monospace => "monospace",
    }
}
fn parse_cat(x: &str) -> FontCategory {
    match x {
        "sans-serif" => FontCategory::SansSerif,
        "serif" => FontCategory::Serif,
        "display" => FontCategory::Display,
        "handwriting" => FontCategory::Handwriting,
        "monospace" => FontCategory::Monospace,
        _ => FontCategory::Unknown,
    }
}
fn escape_like(x: &str) -> String {
    x.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
fn norm(x: &Path) -> String {
    x.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}
fn valid_name(x: &str) -> Result<String, AppError> {
    let x = x.trim();
    if x.is_empty() || x.chars().count() > 80 {
        return Err(AppError::InvalidInput(
            "Collection name must contain 1 to 80 characters.".into(),
        ));
    }
    Ok(x.into())
}
fn sql_count(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
