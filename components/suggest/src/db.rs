/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{path::Path, sync::Arc};

use interrupt_support::{SqlInterruptHandle, SqlInterruptScope};
use parking_lot::Mutex;
use rusqlite::{
    named_params,
    types::{FromSql, ToSql},
    Connection, OpenFlags,
};
use sql_support::{open_database::open_database_with_flags, ConnExt};

use crate::{
    keyword::full_keyword,
    rs::{DownloadedSuggestion, SuggestRecordId, SuggestionProvider},
    schema::SuggestConnectionInitializer,
    Result, Suggestion,
};

/// The metadata key whose value is the timestamp of the last record ingested
/// from the Suggest Remote Settings collection.
pub const LAST_INGEST_META_KEY: &str = "last_quicksuggest_ingest";

/// A list of [`Suggestion::iab_category`] values used to distinguish
/// non-sponsored suggestions.
pub const NONSPONSORED_IAB_CATEGORIES: &[&str] = &["5 - Education"];

/// The database connection type.
#[derive(Clone, Copy)]
pub(crate) enum ConnectionType {
    ReadOnly,
    ReadWrite,
}

impl From<ConnectionType> for OpenFlags {
    fn from(type_: ConnectionType) -> Self {
        match type_ {
            ConnectionType::ReadOnly => {
                OpenFlags::SQLITE_OPEN_URI
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX
                    | OpenFlags::SQLITE_OPEN_READ_ONLY
            }
            ConnectionType::ReadWrite => {
                OpenFlags::SQLITE_OPEN_URI
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_READ_WRITE
            }
        }
    }
}

/// A thread-safe wrapper around an SQLite connection to the Suggest database,
/// and its interrupt handle.
pub(crate) struct SuggestDb {
    pub conn: Mutex<Connection>,

    /// An object that's used to interrupt an ongoing database operation.
    ///
    /// When this handle is interrupted, the thread that's currently accessing
    /// the database will be told to stop and release the `conn` lock as soon
    /// as possible.
    pub interrupt_handle: Arc<SqlInterruptHandle>,
}

impl SuggestDb {
    /// Opens a read-only or read-write connection to a Suggest database at the
    /// given path.
    pub fn open(path: impl AsRef<Path>, type_: ConnectionType) -> Result<Self> {
        let conn = open_database_with_flags(path, type_.into(), &SuggestConnectionInitializer)?;
        Ok(Self::with_connection(conn))
    }

    fn with_connection(conn: Connection) -> Self {
        let interrupt_handle = Arc::new(SqlInterruptHandle::new(&conn));
        Self {
            conn: Mutex::new(conn),
            interrupt_handle,
        }
    }

    /// Accesses the Suggest database for reading.
    pub fn read<T>(&self, op: impl FnOnce(&SuggestDao) -> Result<T>) -> Result<T> {
        let conn = self.conn.lock();
        let scope = self.interrupt_handle.begin_interrupt_scope()?;
        let dao = SuggestDao::new(&conn, scope);
        op(&dao)
    }

    /// Accesses the Suggest database in a transaction for reading and writing.
    pub fn write<T>(&self, op: impl FnOnce(&mut SuggestDao) -> Result<T>) -> Result<T> {
        let mut conn = self.conn.lock();
        let scope = self.interrupt_handle.begin_interrupt_scope()?;
        let tx = conn.transaction()?;
        let mut dao = SuggestDao::new(&tx, scope);
        let result = op(&mut dao)?;
        tx.commit()?;
        Ok(result)
    }
}

/// A data access object (DAO) that wraps a connection to the Suggest database
/// with methods for reading and writing suggestions, icons, and metadata.
///
/// Methods that only read from the database take an immutable reference to
/// `self` (`&self`), and methods that write to the database take a mutable
/// reference (`&mut self`).
pub(crate) struct SuggestDao<'a> {
    pub conn: &'a Connection,
    scope: SqlInterruptScope,
}

impl<'a> SuggestDao<'a> {
    fn new(conn: &'a Connection, scope: SqlInterruptScope) -> Self {
        Self { conn, scope }
    }

    /// Fetches suggestions that match the given keyword from the database.
    pub fn fetch_by_keyword(&self, keyword: &str) -> Result<Vec<Suggestion>> {
        self.conn.query_rows_and_then_cached(
            "SELECT s.id, k.rank, s.title, s.url, s.provider,
                        (SELECT i.data FROM icons i WHERE i.id = s.icon_id) AS icon
                  FROM suggestions s
                  JOIN keywords k ON k.suggestion_id = s.id
                  WHERE k.keyword = :keyword
                  LIMIT 1",
            named_params! {
                ":keyword": keyword,
            },
            |row| -> Result<Suggestion>{
                let suggestion_id: i64 = row.get("id")?;
                let title = row.get("title")?;
                let url = row.get("url")?;
                let icon = row.get("icon")?;
                let provider = row.get("provider")?;

                let keywords: Vec<String> = self.conn.query_rows_and_then_cached(
                    "SELECT keyword FROM keywords
                     WHERE suggestion_id = :suggestion_id AND rank >= :rank
                     ORDER BY rank ASC",
                    named_params! {
                        ":suggestion_id": suggestion_id,
                        ":rank": row.get::<_, i64>("rank")?,
                    },
                    |row| row.get(0),
                )?;

                match provider {
                    SuggestionProvider::Amp => {
                        self.conn.query_row_and_then(
                            "SELECT amp.advertiser, amp.block_id, amp.iab_category, amp.impression_url, amp.click_url
                            FROM amp_custom_details amp WHERE amp.suggestion_id = :suggestion_id", 
                            named_params! {
                                ":suggestion_id": suggestion_id
                            },
                            |row|{
                                let iab_category = row.get::<_, String>("iab_category")?;
                                let is_sponsored = !NONSPONSORED_IAB_CATEGORIES.contains(&iab_category.as_str());
                                Ok(Suggestion {
                                    block_id: row.get("block_id")?,
                                    advertiser: row.get("advertiser")?,
                                    iab_category,
                                    is_sponsored,
                                    title,
                                    url,
                                    full_keyword: full_keyword(keyword, &keywords),
                                    icon,
                                    impression_url: row.get("impression_url")?,
                                    click_url: row.get("click_url")?,
                                    provider
                                })
                            }
                        )
                    },
                    SuggestionProvider::Wikipedia =>{
                        Ok(Suggestion {
                            block_id: 0,
                            advertiser: "Wikipedia".to_string(),
                            iab_category: "5 - Education".to_string(),
                            is_sponsored: false,
                            title,
                            url,
                            full_keyword: full_keyword(keyword, &keywords),
                            icon,
                            impression_url: None,
                            click_url: None,
                            provider
                        })
                    }
                }


            },
        )
    }

    /// Inserts all suggestions associated with a Remote Settings record into
    /// the database.
    pub fn insert_suggestions(
        &mut self,
        record_id: &SuggestRecordId,
        suggestions: &[DownloadedSuggestion],
    ) -> Result<()> {
        for suggestion in suggestions {
            self.scope.err_if_interrupted()?;
            let common_details = suggestion.common_details();
            let provider = suggestion.provider();
            let suggestion_id: i64 = self.conn.query_row_and_then_cachable(
                "INSERT INTO suggestions(
                     record_id,
                     provider,
                     title,
                     url,
                     icon_id
                 )
                 VALUES(
                     :record_id,
                     :provider,
                     :title,
                     :url,
                     :icon_id
                 )
                 RETURNING id
                 ",
                named_params! {
                    ":record_id": record_id.as_str(),
                    ":provider": &provider,
                    ":title": common_details.title,
                    ":url": common_details.url,
                    ":icon_id": common_details.icon_id

                },
                |row| row.get(0),
                true,
            )?;
            match suggestion {
                DownloadedSuggestion::Amp(amp) => {
                    self.conn.execute(
                        "INSERT INTO amp_custom_details(
                             suggestion_id,
                             advertiser,
                             block_id,
                             iab_category,
                             impression_url,
                             click_url
                         )
                         VALUES(
                             :suggestion_id,
                             :advertiser,
                             :block_id,
                             :iab_category,
                             :impression_url,
                             :click_url
                         )",
                        named_params! {
                            ":suggestion_id": suggestion_id,
                            ":advertiser": amp.advertiser,
                            ":block_id": amp.block_id,
                            ":iab_category": amp.iab_category,
                            ":impression_url": amp.impression_url,
                            ":click_url": amp.click_url

                        },
                    )?;
                }
                DownloadedSuggestion::Wikipedia(_) => {}
            }
            for (index, keyword) in common_details.keywords.iter().enumerate() {
                self.conn.execute(
                    "INSERT INTO keywords(
                         keyword,
                         suggestion_id,
                         rank
                     )
                     VALUES(
                         :keyword,
                         :suggestion_id,
                         :rank
                     )",
                    named_params! {
                        ":keyword": keyword,
                        ":rank": index,
                        ":suggestion_id": suggestion_id,
                    },
                )?;
            }
        }
        Ok(())
    }

    /// Inserts an icon for a suggestion into the database.
    pub fn insert_icon(&mut self, icon_id: &str, data: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO icons(
                 id,
                 data
             )
             VALUES(
                 :id,
                 :data
             )",
            named_params! {
                ":id": icon_id,
                ":data": data,
            },
        )?;
        Ok(())
    }

    /// Deletes all suggestions associated with a Remote Settings record from
    /// the database.
    pub fn drop_suggestions(&mut self, record_id: &SuggestRecordId) -> Result<()> {
        self.conn.execute_cached(
            "DELETE FROM suggestions WHERE record_id = :record_id",
            named_params! { ":record_id": record_id.as_str() },
        )?;
        Ok(())
    }

    /// Deletes an icon for a suggestion from the database.
    pub fn drop_icon(&mut self, icon_id: &str) -> Result<()> {
        self.conn.execute_cached(
            "DELETE FROM icons WHERE id = :id",
            named_params! { ":id": icon_id },
        )?;
        Ok(())
    }

    /// Clears the database, removing all suggestions, icons, and metadata.
    pub fn clear(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM suggestions;
             DELETE FROM icons;
             DELETE FROM meta;",
        )?;
        Ok(())
    }

    /// Returns the value associated with a metadata key.
    pub fn get_meta<T: FromSql>(&self, key: &str) -> Result<Option<T>> {
        Ok(self.conn.try_query_one(
            "SELECT value FROM meta WHERE key = :key",
            named_params! { ":key": key },
            true,
        )?)
    }

    /// Sets the value for a metadata key.
    pub fn put_meta(&mut self, key: &str, value: impl ToSql) -> Result<()> {
        self.conn.execute_cached(
            "REPLACE INTO meta(key, value) VALUES(:key, :value)",
            named_params! { ":key": key, ":value": value },
        )?;
        Ok(())
    }
}
