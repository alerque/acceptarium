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
#[cfg(feature = "ollama")]
use crate::config::{LLMConfig, VisionConfig};
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

#[cfg(feature = "ollama")]
const PREAMBLE: &str = r#"You are a data extraction agent that analyzes scanned receipts and derives structured transaction data.
Always respond with valid JSON only, no additional text."#;

#[cfg(feature = "ollama")]
const FIELDS: &str = r#"
Look for and extract the following fields in the receipt:
- payee: The vendor or merchant name
- date: The transaction date in ISO 8601 format (YYYY-MM-DD), or include time (YYYY-MM-DDTHH:MM:SS) if available
- total: The total amount as a number (without currency symbols)
- currency: The currency used a it's ISO code, for example "TRY" or "USD"
- payment_type: The payment method used - "cash", "card", or "other"
- payment_identifier: The last 4 digits of card or other identifier if visible (can be null)
- category: "receipt" or "invoice" depending on document type
- invoice_number: The invoice or receipt number if visible (can be null)
- items: An array of items with description, quantity, and total amount for each line item

Return a JSON object with as many of those fields as were positively detected, for example:
{
  "payee": "Store Name",
  "date": "2024-01-15T14:30:00",
  "total": 125.50,
  "currency": "TRY",
  "payment_type": "card",
  "payment_identifier": "**** **** **** 1234",
  "category": "receipt",
  "invoice_number": "A12345",
  "items": [
    {"description": "Item 1", "quantity": 1, "amount": 50.00},
    {"description": "Item 2", "quantity": 2, "amount": 37.75}
  ]
}
"#;

pub fn process<ID>(config: &Config, id: ID) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
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
        use crate::error::AssetProcessedSnafu;
        use crate::storage::instantiate_storage;
        use snafu::ensure;
        let storage = instantiate_storage(config)?;
        let id: AssetId = id.try_into()?;
        let mut asset = storage.load(id.clone())?;
        log::info!("Processing asset {id}");
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
                    let vision_config =
                        config.vision.clone().context(MissingProcessorConfigSnafu {
                            processor: "vision",
                        })?;
                    println!("VISION MODEL RESULTS:");
                    let data = Runtime::new()?
                        .block_on(query_ollama_vision(&vision_config, asset.clone()))?;
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
                                let llm_config = config
                                    .llm
                                    .clone()
                                    .context(MissingProcessorConfigSnafu { processor: "ocr" })?;
                                println!("OCR DERIVED DATA:");
                                let data = Runtime::new()?
                                    .block_on(query_ollama_ocr(&llm_config, ocr.as_str()))?;
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
        Ok(())
    }
}

#[cfg(feature = "ollama")]
async fn query_ollama_vision(config: &VisionConfig, asset: Asset) -> Result<String> {
    let model = &config.model;
    let cwd = current_dir().unwrap_or(PathBuf::from("./"));
    let file = asset.asset_path(cwd.as_path()).unwrap();
    log::info!("Creating LLM agent for model {}", model);
    let client: ollama::Client = ollama::Client::new(Nothing).unwrap();
    let llm = client.agent(model).preamble(PREAMBLE).build();
    log::debug!("Using preamble: {}", PREAMBLE);
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
    let message = format!(
        r#"The attached image is a scanned receipt or invoice in Turkish.

{}"#,
        FIELDS,
    );
    log::debug!("Sending message: {}", &message);
    let text_content = UserContent::text(&message);
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
async fn query_ollama_ocr(config: &LLMConfig, ocr: &str) -> Result<String> {
    let model = &config.model;
    log::info!("Creating LLM agent for model {}", model);
    let client: ollama::Client = ollama::Client::new(Nothing).unwrap();
    let llm = client.agent(model).preamble(PREAMBLE).build();
    log::debug!("Using preamble: {}", PREAMBLE);
    let message = format!(
        r#"The following content is a scanned receipt or invoice in Turkish read with OCR.

{}

Receipt content:
{}
"#,
        FIELDS, ocr
    );
    log::debug!("Sending message: {}", &message);
    let response = llm.prompt(message).await.expect("Failed to prompt");
    Ok(response)
}
