// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashSet;
use std::path::PathBuf;

use derive_more::Debug;
use snafu::OptionExt;

use crate::CommitMessage;
use crate::error::{
    FilesystemSnafu, NoStorageConfiguredSnafu, NoTrackerConfiguredSnafu, UnsupportedStorageSnafu,
};
use crate::ingestable::Ingestable;
use crate::storage::filesystem::{plain::PlainBlob, sidecar::SidecarInfo};
#[cfg(feature = "git-annex")]
use crate::storage::git_annex::AnnexedBlob;
#[cfg(feature = "git")]
use crate::vcs::GitTracker;
use crate::vcs::ManualTracker;
use crate::vcs::PreStageCallback;
use crate::{Asset, AssetId, Assets};
use crate::{AssetSelectors, Config, InfoFormat, OperationMode};
use crate::{BlobHandler, BlobStorage};
use crate::{Error, Result};
use crate::{InfoHandler, InfoStorage};
use crate::{StorageTracker, VersionHandler};

#[derive(Debug)]
pub struct Acceptarium {
    dry_run: bool,
    dirty: bool,
    #[debug(skip)]
    blob: Box<dyn BlobStorage>,
    #[debug(skip)]
    info: Box<dyn InfoStorage>,
    #[debug(skip)]
    tracker: Box<dyn StorageTracker>,
}

impl Acceptarium {
    pub fn new(config: &Config) -> Result<Self> {
        log::debug!("Selecting and initializing storage backend");
        let blob: Box<dyn BlobStorage> = match config.blob_storage {
            Some(BlobHandler::Filesystem) => Box::new(PlainBlob::init(config)?),
            #[cfg(feature = "git-annex")]
            Some(BlobHandler::GitAnnex) => Box::new(AnnexedBlob::init(config)?),
            #[cfg(not(feature = "git-annex"))]
            Some(BlobHandler::GitAnnex) => UnsupportedStorageSnafu {
                driver: "git-annex",
            }
            .fail()?,
            None => NoStorageConfiguredSnafu {}.fail()?,
        };
        let info: Box<dyn InfoStorage> = match config.info_storage {
            Some(InfoHandler::Sidecar) => Box::new(SidecarInfo::new(config)?),
            #[cfg(feature = "git-annex")]
            Some(InfoHandler::Metadata) => blob.as_info().context(UnsupportedStorageSnafu {
                driver: "metadata requires git-annex",
            })?,
            #[cfg(not(feature = "git-annex"))]
            Some(InfoHandler::Metadata) => {
                return UnsupportedStorageSnafu { driver: "metadata" }.fail();
            }
            None => NoStorageConfiguredSnafu {}.fail()?,
        };
        let tracker: Box<dyn StorageTracker> = match config.tracker {
            #[cfg(feature = "git")]
            Some(VersionHandler::Git) => Box::new(GitTracker::new(config)?),
            #[cfg(not(feature = "git"))]
            Some(VersionHandler::Git) => {
                return UnsupportedStorageSnafu { driver: "git" }.fail();
            }
            Some(VersionHandler::Manual) => Box::new(ManualTracker {}),
            None => NoTrackerConfiguredSnafu {}.fail()?,
        };
        Ok(Self {
            dry_run: config.dry_run,
            dirty: config.dirty,
            blob,
            info,
            tracker,
        })
    }

    pub fn add<I, P>(&self, sources: I) -> Result<()>
    where
        I: IntoIterator<Item = P>,
        P: TryInto<Box<dyn Ingestable>, Error = Error>,
    {
        self.tracker.is_clean(self.dirty)?;
        let ingestables: Vec<Box<dyn Ingestable>> = sources
            .into_iter()
            .map(|source| source.try_into())
            .collect::<Result<_>>()?;
        let mut seen_hashes = HashSet::new();
        let mut valid_ingestables: Vec<Box<dyn Ingestable>> = Vec::new();
        for ingestable in ingestables {
            log::debug!("Attempting dry run add for {:?}", ingestable);
            match self.blob.ingest(
                OperationMode::JustCheck,
                &*ingestable,
                &*self.info,
                &*self.tracker,
            )? {
                Some(_) => {
                    let hash = ingestable.blake3();
                    if !seen_hashes.insert(hash.clone()) {
                        return FilesystemSnafu {
                            message: format!(
                                "Skipping duplicate file '{}' with hash already seen in this batch",
                                ingestable
                                    .filename()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default()
                            ),
                        }
                        .fail();
                    } else {
                        valid_ingestables.push(ingestable);
                    }
                }
                None => {
                    log::warn!(
                        "An asset is already tracking the same hash as '{}'",
                        ingestable
                            .filename()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    );
                }
            }
        }
        if !self.dry_run {
            for ingestable in &valid_ingestables {
                log::debug!("Adding {:?}", ingestable);
                if let Some(asset) = self.blob.ingest(
                    OperationMode::JustRun,
                    &**ingestable,
                    &*self.info,
                    &*self.tracker,
                )? {
                    println!("{}", asset);
                } else {
                    return FilesystemSnafu {
                        message: "Something went south with wet run!".to_string(),
                    }
                    .fail()?;
                }
            }
        }
        self.tracker.commit_staged(Some(&move |msg| {
            msg.subject = Some("Track new asset(s)".into());
        }))?;
        Ok(())
    }

    pub fn remove(&self, assets: Assets) -> Result<()> {
        self.tracker.is_clean(self.dirty)?;
        for (_, asset) in &assets {
            self.blob.egest(asset, &*self.info, &*self.tracker)?;
        }
        self.tracker.commit_staged(Some(&move |msg| {
            msg.subject = Some("Remove asset(s)".into());
        }))?;
        Ok(())
    }

    pub fn list(&self) -> Result<Assets> {
        self.info.list()
    }

    pub fn get(&self, format: InfoFormat, id: AssetId, key: &str) -> Result<String> {
        self.info.get(format, id, key)
    }

    pub fn set(&self, format: InfoFormat, id: AssetId, key: &str, value: &str) -> Result<()> {
        let mut asset = self.info.load(id.clone())?;
        asset.set_field(format, key, value)?;
        self.info.write(&asset, &*self.tracker)?;
        Ok(())
    }

    pub fn write(&self, asset: &Asset) -> Result<()> {
        self.info.write(asset, &*self.tracker)
    }

    pub fn select(&self, selectors: &AssetSelectors) -> Result<Assets> {
        let assets = if selectors.all {
            self.info.list()?
        } else if selectors.processed {
            let mut assets = self.info.list()?;
            assets.retain(|_, asset| asset.transaction().is_some());
            assets
        } else if selectors.unprocessed {
            let mut assets = self.info.list()?;
            assets.retain(|_, asset| asset.transaction().is_none());
            assets
        } else {
            let mut assets = Assets::new();
            if let Some(ids) = &selectors.ids {
                for id in ids {
                    let asset_id: AssetId = id.try_into()?;
                    let asset = self.info.load(asset_id)?;
                    assets.insert(asset);
                }
            }
            assets
        };
        Ok(assets)
    }

    pub fn is_clean(&self, dirty: bool) -> Result<()> {
        self.tracker.is_clean(dirty)
    }

    pub fn stage_paths(
        &self,
        paths: &[PathBuf],
        pre_stage_hook: Option<&PreStageCallback>,
    ) -> Result<()> {
        self.tracker.stage_paths(paths, pre_stage_hook)
    }

    pub fn commit_staged(&self, composer: Option<&dyn Fn(&mut CommitMessage)>) -> Result<()> {
        self.tracker.commit_staged(composer)
    }
}
