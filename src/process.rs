// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(any(feature = "ollama", feature = "tesseract", feature = "imagemagick"))]
use crate::Asset;
use crate::AssetId;
#[cfg(any(feature = "ollama", feature = "tesseract", feature = "imagemagick"))]
use crate::Extractor;
#[cfg(feature = "ollama")]
use crate::Processor;
#[cfg(any(feature = "ollama", feature = "tesseract", feature = "imagemagick"))]
use crate::Transaction;
#[cfg(not(any(feature = "ollama", feature = "tesseract", feature = "imagemagick")))]
use crate::error::FeatureNotEnabledSnafu;
#[cfg(feature = "ollama")]
use crate::error::MissingProcessorConfigSnafu;
use crate::{Config, Error, Result};

#[cfg(feature = "ollama")]
use base64::engine::{Engine as _, general_purpose};
#[cfg(feature = "ollama")]
use rig::{
    OneOrMany,
    client::{CompletionClient, Nothing},
    completion::Prompt,
    completion::message::ImageMediaType,
    message::{Message, UserContent},
    providers::ollama,
};
#[cfg(feature = "ollama")]
use snafu::OptionExt;

#[cfg(any(feature = "ollama", feature = "tesseract", feature = "imagemagick"))]
use std::env::current_dir;
#[cfg(any(feature = "ollama", feature = "tesseract", feature = "imagemagick"))]
use std::fs::read;
#[cfg(feature = "ollama")]
use std::path::PathBuf;
#[cfg(feature = "ollama")]
use tokio::runtime::Runtime;

pub fn process<ID>(config: &Config, all: bool, unprocessed: bool, ids: Option<&[ID]>) -> Result<()>
where
    for<'a> &'a ID: TryInto<AssetId>,
    for<'a> Error: From<<&'a ID as TryInto<AssetId>>::Error>,
{
    #[cfg(not(any(feature = "ollama", feature = "tesseract", feature = "imagemagick")))]
    return {
        let _ = config;
        let _ = id;
        FeatureNotEnabledSnafu {
            feature: "ollama,tesseract,imagemagick",
        }
    }
    .fail();
    #[cfg(any(feature = "ollama", feature = "tesseract", feature = "imagemagick"))]
    {
        use crate::Assets;
        use crate::error::AssetProcessedSnafu;
        use crate::storage::instantiate_storage;
        use snafu::ensure;
        let storage = instantiate_storage(config)?;
        let assets = if all {
            storage.list()?
        } else if unprocessed {
            let all_assets = storage.list()?;
            let mut assets = Assets::new();
            for (_, asset) in all_assets.iter() {
                let asset = asset.clone();
                if asset.transaction().is_some() {
                    continue;
                }
                assets.add(asset.clone());
            }
            assets
        } else {
            let mut assets = Assets::new();
            if let Some(ids) = ids {
                for id in ids {
                    let asset_id: AssetId = id.try_into()?;
                    let asset = storage.load(asset_id)?;
                    if unprocessed && asset.transaction().is_some() {
                        continue;
                    }
                    assets.add(asset);
                }
            }
            assets
        };
        for (_, asset) in assets.iter() {
            let mut asset = asset.clone();
            log::info!("Processing asset {}", &asset.id());
            let has_existing = asset.transaction().is_some();
            log::debug!("Checking for previously processed: {has_existing}");
            ensure!(!has_existing || config.overwrite, AssetProcessedSnafu {});
            let data: String = match config.processor {
                Processor::Vision => {
                    log::info!("Using vision processor");
                    #[cfg(not(feature = "ollama"))]
                    return FeatureNotEnabledSnafu { feature: "ollama" }.fail();
                    #[cfg(feature = "ollama")]
                    {
                        println!("VISION MODEL RESULTS:");
                        let data = Runtime::new()?.block_on(query_ollama_vision(config, &asset))?;
                        println!("{}", &data);
                        data
                    }
                }
                Processor::OCR => {
                    log::info!("Using OCR processor");
                    #[cfg(not(any(feature = "tesseract", feature = "imagemagick")))]
                    FeatureNotEnabledSnafu {
                        feature: "tesseract,imagemagick",
                    }
                    .fail()?;
                    #[cfg(all(feature = "tesseract", feature = "imagemagick"))]
                    {
                        println!("OCR RESULTS:");
                        let ocr = ocr_tesseract(asset.clone())?;
                        println!("{}", &ocr);
                        asset.set_ocr(Some(ocr.clone()));
                        match config.extractor {
                            Extractor::LLM => {
                                log::info!("Using LLM extractor");
                                #[cfg(not(feature = "ollama"))]
                                return FeatureNotEnabledSnafu { feature: "ollama" }.fail();
                                #[cfg(feature = "ollama")]
                                {
                                    println!("OCR DERIVED DATA:");
                                    let data = Runtime::new()?
                                        .block_on(query_ollama_ocr(config, &asset))?;
                                    println!("{}", &data);
                                    data
                                }
                            }
                            _ => unimplemented!(),
                        }
                    }
                }
                Processor::Manual => unimplemented!(),
            };
            let transaction: Transaction = serde_json::from_str(&data)?;
            log::debug!("Saving transaction data: {:?}", transaction);
            asset.set_transaction(Some(transaction));
            storage.save(&asset)?;
        }
        Ok(())
    }
}

#[cfg(feature = "ollama")]
async fn query_ollama_vision(config: &Config, asset: &Asset) -> Result<String> {
    let vision = config.vision.clone().context(MissingProcessorConfigSnafu {
        processor: "vision",
    })?;
    let model = vision.model;
    let cwd = current_dir().unwrap_or(PathBuf::from("./"));
    let file = asset.asset_path(cwd.as_path()).unwrap();
    log::info!("Creating LLM agent for model {}", model);
    let client: ollama::Client = ollama::Client::new(Nothing).unwrap();
    let preamble = vision.preamble.render(config, asset)?;
    log::debug!("Using preamble: {}", preamble);
    let llm = client.agent(model).preamble(&preamble).build();
    let image_bytes = read(&file)?;
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    let media_type = match ext.as_str() {
        "png" => Some(ImageMediaType::PNG),
        "gif" => Some(ImageMediaType::GIF),
        "webp" => Some(ImageMediaType::WEBP),
        "heic" => Some(ImageMediaType::HEIC),
        "heif" => Some(ImageMediaType::HEIF),
        "jpg" | "jpeg" => Some(ImageMediaType::JPEG),
        _ => None,
    };
    log::debug!("Detected media type: {:?}", media_type);
    let image_base64 = general_purpose::STANDARD.encode(&image_bytes);
    let image_content = UserContent::image_base64(image_base64, media_type, None);
    let prompt = vision.prompt.render(config, asset)?;
    log::debug!("Sending prompt: {}", &prompt);
    let text_content = UserContent::text(&prompt);
    let content = vec![image_content, text_content];
    let content: OneOrMany<UserContent> =
        OneOrMany::many(content).expect("Unable to create user message");
    let content: Message = content.into();
    let response = llm.prompt(content).await.expect("Failed to prompt");
    Ok(response)
}

#[cfg(all(feature = "tesseract", feature = "imagemagick"))]
fn ocr_tesseract(asset: Asset) -> Result<String> {
    use subprocess::Exec;
    let file = asset.asset_path(&current_dir()?).unwrap();
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    let is_pdf = ext == "pdf";
    let file_content = read(&file)?;
    let output = if is_pdf {
        log::info!("Processing PDF file via ImageMagick, then tesseract");
        Exec::shell("magick -density 300 - -flatten png:- | tesseract - - -l tur")
            .stdin(file_content)
            .stdout(subprocess::Redirection::Pipe)
            .capture()?
            .stdout_str()
    } else {
        log::info!("Processing image file via tesseract");
        Exec::cmd("tesseract")
            .arg("stdin")
            .arg("stdout")
            .arg("-l")
            .arg("tur")
            .stdin(file_content)
            .stdout(subprocess::Redirection::Pipe)
            .capture()?
            .stdout_str()
    };
    Ok(output)
}

#[cfg(feature = "ollama")]
async fn query_ollama_ocr(config: &Config, asset: &Asset) -> Result<String> {
    let llm = config
        .llm
        .clone()
        .context(MissingProcessorConfigSnafu { processor: "ocr" })?;
    log::info!("Creating LLM agent for model {}", &llm.model);
    let client: ollama::Client = ollama::Client::new(Nothing).unwrap();
    let preamble = llm.preamble.render(config, asset)?;
    let agent = client.agent(llm.model).preamble(&preamble).build();
    log::debug!("Using preamble: {}", preamble);
    let prompt = llm.prompt.render(config, asset)?;
    log::debug!("Sending prompt: {}", &prompt);
    let response = agent.prompt(prompt).await.expect("Failed to prompt");
    Ok(response)
}
