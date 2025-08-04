use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::*;

#[derive(Debug)]
pub struct WallpaperQueueBuilder {
    pub db: Option<Sqlite>,
    pub db_bgs: Option<Vec<(Cow<'static, str>, Vec<Arc<String>>)>>,
    pub fs_bgs: Option<Vec<Arc<String>>>,
}

impl WallpaperQueueBuilder {
    pub(super) fn new() -> Self {
        Self {
            db: None,
            db_bgs: None,
            fs_bgs: None,
        }
    }

    pub async fn with_filesystem_backgrounds(mut self) -> anyhow::Result<Self> {
        let fs_bgs = super::fs_backgrounds::FsBackgrounds::new()
            .await?
            .into_inner()
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>();

        self.fs_bgs = Some(fs_bgs);

        Ok(self)
    }

    pub async fn with_persisted(mut self) -> Result<Self, sqlx::Error> {
        let db = self.db.unwrap_or(open_or_make_db().await.unwrap());

        let rows = db.get_all_queues().await?;

        // Map paths present in multiple queues to the same Arc
        // (drop duplicates and only keep one of each path in memory)
        let mut rows = rows.into_iter().map(|e| {
            let p = Arc::new(e.path);
            (p, e.playlist)
        });

        let unique_paths = rows
            .by_ref()
            .map(|(p, _)| p.clone())
            .collect::<HashSet<_>>();

        let mut rc_duplicate_paths =
            rows.map(|(path, playlist)| (unique_paths.get(&path).unwrap().clone(), playlist));

        // Filter by playlist name

        let playlist_names = rc_duplicate_paths
            .by_ref()
            .map(|(_, playlist)| playlist)
            .collect::<HashSet<_>>();

        let playlists = playlist_names.into_iter().map(|name| {
            let v = if let Some(ref name) = name {
                rc_duplicate_paths
                    .by_ref()
                    .skip_while(|(_, p)| p.is_none() || p.as_ref().unwrap() != name)
                    .take_while(|(_, p)| p.is_some() && p.as_ref().unwrap() == name)
                    .collect::<Vec<_>>()
            } else {
                rc_duplicate_paths
                    .by_ref()
                    // sql statement should put NULL playlist rows at head. Skip to be safe.
                    .skip_while(|(_, p)| p.is_some())
                    .take_while(|(_, p)| p.is_none())
                    .collect::<Vec<_>>()
            };

            let name = if let Some(name) = name {
                Cow::Owned(name)
            } else {
                Cow::Borrowed("")
            };

            (name, v)
        });

        // Remove unneded fields of playlist entries
        let playlists = playlists
            .map(|(k, v)| (k, v.into_iter().map(|(path, _)| path).collect::<Vec<_>>()))
            .collect::<Vec<(_, _)>>();

        dbg!(&playlists);

        self.db = Some(db);
        self.db_bgs = Some(playlists);

        Ok(self)
    }

    pub async fn build(self) -> WallpaperQueue {
        let playlists: HashMap<Cow<'static, str>, Vec<Arc<String>>> =
            match (self.fs_bgs, self.db_bgs) {
                (Some(fs_bgs), Some(mut db_bgs)) => {
                    // Join these together.
                    // Also report images that have been removed.

                    let mut db_playlist_all =
                        db_bgs.iter_mut().find(|(playlist, _)| playlist == "ALL");

                    let _removed_ones = if let Some(ref all) = db_playlist_all {
                        Some(
                            all.1
                                .iter()
                                .filter_map(|p| {
                                    if fs_bgs.contains(p) {
                                        None
                                    } else {
                                        Some(p.clone())
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    };

                    // None if the "ALL" playlist could not be found (this is likely because of a fresh start)
                    // Some with an empty Vec means no new backgrounds were found on the filesystem.
                    let new_according_to_db = if let Some(ref all) = db_playlist_all {
                        Some(
                            fs_bgs
                                .iter()
                                .filter_map(|p| {
                                    if all.1.contains(p) {
                                        None
                                    } else {
                                        Some(p.clone())
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    };

                    if let Some(mut paths) = new_according_to_db {
                        if !paths.is_empty() {
                            // As these are newly added to the filesystem they have no playlist specified. Add them to ALL.
                            let (_, all) = &mut db_playlist_all
                                .as_mut()
                                .expect("Some when new_according_to_db is Some");

                            // put new images at head
                            paths.append(all);
                            std::mem::replace(all, paths);
                        }

                        db_bgs.into_iter().collect()
                    } else {
                        [(Cow::Borrowed("ALL"), fs_bgs)].into()
                    }
                }
                (None, None) => panic!("Programmer error. Either read images from fs or db."),
                (_, _) => todo!(
                    "Unprioritized. Clause description: Read from either fs or db. Whoever is Some."
                ),
            };

        WallpaperQueue::new(Queue::new(playlists), self.db).await
    }
}
