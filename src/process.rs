// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::error::AssetProcessedSnafu;
use crate::storage::instantiate_storage;
use crate::{Asset, AssetId};
use crate::{Config, Error, Result};

use snafu::ensure;
use snafu::OptionExt;
use tokio::runtime::Runtime;

pub fn process<ID>(config: &Config, id: ID) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let storage = instantiate_storage(config)?;
    let id: AssetId = id.try_into()?;
    let asset = storage.load(id)?;
    let has_existing = asset.transaction().is_some();
    ensure!(!has_existing || config.overwrite, AssetProcessedSnafu {});
    let res = Runtime::new()?.block_on(query_ollama_vision(asset.clone()))?;
    println!("VISION MODEL RESULTS");
    println!("{}", &res);
    let ocr = ocr_tesseract(asset.clone())?;
    println!("OCR RESULTS");
    println!("{}", &ocr);
    let data = Runtime::new()?.block_on(query_ollama_ocr(ocr.as_str(), ""))?;
    println!("OCR DERIVED DATA");
    println!("{}", &data);
    Ok(())
}

use base64::engine::{general_purpose, Engine as _};
use rig::client::{CompletionClient, Nothing};
use rig::completion::message::ImageMediaType;
use rig::completion::Prompt;
use rig::message::Message;
use rig::message::UserContent;
use rig::providers::ollama;
use rig::OneOrMany;
use std::env::current_dir;
use std::fs;
use std::path::PathBuf;

const PREAMBLE: &str = r#"You are a data extraction agent that analyzes scanned receipts and derives structured transaction data.
Always respond with valid JSON only, no additional text."#;

const FIELDS: &str = r#"
Look for and extract the following fields in the receipt:
- payee: The vendor or merchant name
- date: The transaction date in ISO 8601 format (YYYY-MM-DD), or include time (YYYY-MM-DDTHH:MM:SS) if available
- total: The total amount as a number (without currency symbols)
- currency: The curreny used a it's ISO code, for example "TRY" ur "USD"
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

async fn query_ollama_vision(asset: Asset) -> Result<String> {
    let cwd = current_dir().unwrap_or(PathBuf::from("./"));
    let file = asset.asset_path(cwd.as_path()).unwrap();
    let client: ollama::Client = ollama::Client::new(Nothing).unwrap();
    let llm = client
        // .agent("bakllava:7b")
        // .agent("gemma3:27b")
        // .agent("gemma3:4b")
        // .agent("glm-ocr:bf16")
        // .agent("granite3.2-vision:latest")
        // .agent("llama3.2-vision")
        // .agent("qwen3.5:35b")
        .agent("qwen3.5:9b")
        .preamble(PREAMBLE)
        .build();
    let image_bytes = fs::read(&file)?;
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
    let image_base64 = general_purpose::STANDARD.encode(&image_bytes);
    let image_content = UserContent::image_base64(image_base64, media_type, None);
    let message = format!(
        r#"The attached image is a scanned receipt or invoice in Turkish.

{}"#,
        FIELDS,
    );
    let text_content = UserContent::text(&message);
    let content = vec![image_content, text_content];
    // let content = vec![text_content, image_content];
    let content: OneOrMany<UserContent> =
        OneOrMany::many(content).expect("Unable to create user message");
    let content: Message = content.into();
    let response = llm.prompt(content).await.expect("Failed to prompt");
    Ok(response)
}

fn ocr_tesseract(asset: Asset) -> Result<String> {
    use subprocess::Exec;
    let file = asset.asset_path(&current_dir()?).unwrap();
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    let is_pdf = ext == "pdf";
    let file_content = fs::read(&file)?;
    let output = if is_pdf {
        Exec::shell("magick -density 300 - -flatten png:- | tesseract - - -l tur")
            .stdin(file_content)
            .stdout(subprocess::Redirection::Pipe)
            .capture()?
            .stdout_str()
    } else {
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

async fn query_ollama_ocr(ocr: &str, _query: &str) -> Result<String> {
    let client: ollama::Client = ollama::Client::new(Nothing).unwrap();
    let llm = client.agent("qwen3.5:35b").preamble(PREAMBLE).build();
    let message = format!(
        r#"The following content is a scanned receipt or invoice in Turkish read with OCR.

{}

Receipt content:
{}
"#,
        FIELDS, ocr
    );
    let response = llm.prompt(message).await.expect("Failed to prompt");
    Ok(response)
}
