use anyhow::anyhow;
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use crate::audio;
use crate::{constants, twitch_stream_state};
use events::EventHandler;
use mime_guess::MimeGuess;
use obws::Client as OBSClient;
use regex::Regex;
use reqwest::Client;
use rodio::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::path::Path;
use subd_types::{Event, UserMessage};

// Which do I need?
// use std::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt; 
use tokio::fs::File;
use tokio::sync::broadcast;

use twitch_irc::{
    login::StaticLoginCredentials, SecureTCPTransport, TwitchIRCClient,
};
use fal_rust::{
    client::{ClientCredentials, FalClient},
    utils::download_image,
};

#[derive(Deserialize)]
struct FalImage {
    url: String,
    width: Option<u32>,
    height: Option<u32>,
    content_type: Option<String>,
}

#[derive(Deserialize)]
struct FalData {
    images: Vec<FalImage>,
    // Other fields can be added here if needed
}

pub struct FalHandler {
    // pub queue_rx: &'static queue::SourcesQueueOutput<f32>,
    pub obs_client: OBSClient,
    pub pool: sqlx::PgPool,
    pub sink: Sink,
    pub twitch_client:
        TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>,
}

#[async_trait]
impl EventHandler for FalHandler {
    async fn handle(
        self: Box<Self>,
        tx: broadcast::Sender<Event>,
        mut rx: broadcast::Receiver<Event>,
    ) -> Result<()> {
        loop {
            let event = rx.recv().await?;
            let msg = match event {
                Event::UserMessage(msg) => msg,
                _ => continue,
            };

            let splitmsg = msg
                .contents
                .split(" ")
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            match handle_fal_commands(
                &tx,
                &self.obs_client,
                &self.twitch_client,
                &self.pool,
                &self.sink,
                splitmsg,
                msg,
            )
            .await
            {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Error: {err}");
                    continue;
                }
            }
        }
    }
}

pub async fn handle_fal_commands(
    _tx: &broadcast::Sender<Event>,
    obs_client: &OBSClient,
    _twitch_client: &TwitchIRCClient<
        SecureTCPTransport,
        StaticLoginCredentials,
    >,
    pool: &sqlx::PgPool,
    _sink: &Sink,
    splitmsg: Vec<String>,
    msg: UserMessage,
) -> Result<()> {
    let default_source = constants::DEFAULT_SOURCE.to_string();
    let source: &str = splitmsg.get(1).unwrap_or(&default_source);

    let is_mod = msg.roles.is_twitch_mod();
    let _not_beginbot =
        msg.user_name != "beginbot" && msg.user_name != "beginbotbot";
    let command = splitmsg[0].as_str();
    let word_count = msg.contents.split_whitespace().count();

    // THIS IS DUMB
    // let (_stream, stream_handle) =
    //     audio::get_output_stream("pulse").expect("stream handle");
    // let (sink, mut queue_rx) = rodio::Sink::new_idle();
    // println!("{:?}", queue_rx.next());
    // stream_handle.play_raw(queue_rx)?;

    match command {
        "!theme" => {
            if _not_beginbot && !is_mod {
                return Ok(());
            }
            let theme = &splitmsg
                .iter()
                .skip(1)
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .join(" ");
            twitch_stream_state::set_ai_background_theme(pool, &theme).await?;
        }

        "!talk" => {
            // Replace with your local file paths specific to fal
            let fal_image_file_path = "prime.jpg";
            let fal_audio_file_path = "TwitchChatTTSRecordings/1701059381_beginbot_prime.wav";

            // Read and encode the image file to a data URI for fal
            let fal_source_image_data_uri = fal_encode_file_as_data_uri(fal_image_file_path).await?;

            // Read and encode the audio file to a data URI for fal
            let fal_driven_audio_data_uri = fal_encode_file_as_data_uri(fal_audio_file_path).await?;

            // Submit the request to fal and handle the result
            match fal_submit_sadtalker_request(&fal_source_image_data_uri, &fal_driven_audio_data_uri).await
            {
                Ok(fal_result) => {
                    println!("fal Result: {}", fal_result);
                }
                Err(e) => {
                    eprintln!("fal Error: {}", e);
                }
            }
        }

        "!fal" => {}

        _ => {
            if !command.starts_with('!')
                && !command.starts_with('@')
                && word_count > 1
            {
                let prompt = msg.contents;
                let theme =
                    twitch_stream_state::get_ai_background_theme(pool).await?;
                let final_prompt = format!("{} {}", theme, prompt);
                create_turbo_image(final_prompt).await?;

                // let theme = "Waifu";
                // let final_prompt = format!("{} {}", theme, prompt);
                // create_turbo_image(final_prompt).await?;
            }
        }
    };

    Ok(())
}

// async fn process_images(
//     timestamp: &str,
//     json_path: &str,
//     extra_save_folder: Option<&str>,
// ) -> Result<()> {
//     // Read the JSON file asynchronously
//
//     // need to take the json_path name and extract out the timestamp
//     let json_data = tokio::fs::read_to_string(json_path).await?;
//
//     // Parse the JSON data into the Data struct
//     let data: FalData = serde_json::from_str(&json_data)?;
//
//     // Regex to match data URLs
//     let data_url_regex =
//         Regex::new(r"data:(?P<mime>[\w/]+);base64,(?P<data>.+)")?;
//
//     for (index, image) in data.images.iter().enumerate() {
//         // Match the data URL and extract MIME type and base64 data
//         if let Some(captures) = data_url_regex.captures(&image.url) {
//             let mime_type = captures.name("mime").unwrap().as_str();
//             let base64_data = captures.name("data").unwrap().as_str();
//
//             // Decode the base64 data
//             let image_bytes = general_purpose::STANDARD.decode(base64_data)?;
//
//             // Determine the file extension based on the MIME type
//             let extension = match mime_type {
//                 "image/png" => "png",
//                 "image/jpeg" => "jpg",
//                 _ => "bin", // Default to binary if unknown type
//             };
//
//             // We might want to look for an ID here or make sure we are using the same json
//             let filename =
//                 format!("tmp/fal_images/{}.{}", timestamp, extension);
//
//             // Save the image bytes to a file
//             let mut file = File::create(&filename).await?;
//             file.write_all(&image_bytes).await?;
//
//             let filename = format!("./tmp/dalle-1.png");
//             let _ = File::create(&Path::new(&filename)).await
//                 .map(|mut f| f.write_all(&image_bytes))
//                 .with_context(|| format!("Error creating: {}", filename))?;
//
//             println!("Saved {}", filename);
//
//             if extra_save_folder.is_some() {
//                 let suno_folder = extra_save_folder.unwrap();
//                 let _ = File::create(&Path::new(&suno_folder)).await
//                     .map(|mut f| f.write_all(&image_bytes))
//                     .with_context(|| {
//                         format!("Error creating: {}", suno_folder)
//                     })?;
//             }
//         } else {
//             eprintln!("Invalid data URL for image at index {}", index);
//         }
//     }
//
//     Ok(())
// }

async fn process_images(
    timestamp: &str,
    json_path: &str,
    extra_save_folder: Option<&str>,
) -> Result<()> {
    // Read the JSON file asynchronously
    let json_data = tokio::fs::read_to_string(json_path).await?;

    // Parse the JSON data into the FalData struct
    let data: FalData = serde_json::from_str(&json_data)?;

    // Regex to match data URLs
    let data_url_regex = Regex::new(r"data:(?P<mime>[\w/]+);base64,(?P<data>.+)")?;

    for (index, image) in data.images.iter().enumerate() {
        // Match the data URL and extract MIME type and base64 data
        if let Some(captures) = data_url_regex.captures(&image.url) {
            let mime_type = captures.name("mime").unwrap().as_str();
            let base64_data = captures.name("data").unwrap().as_str();

            // Decode the base64 data
            let image_bytes = general_purpose::STANDARD.decode(base64_data)?;

            // Determine the file extension based on the MIME type
            let extension = match mime_type {
                "image/png" => "png",
                "image/jpeg" => "jpg",
                _ => "bin", // Default to binary if unknown type
            };

            // Construct the filename using the timestamp and extension
            let filename = format!("tmp/fal_images/{}.{}", timestamp, extension);

            // Save the image bytes to a file asynchronously
            let mut file = File::create(&filename)
                .await
                .with_context(|| format!("Error creating file: {}", filename))?;
            file.write_all(&image_bytes)
                .await
                .with_context(|| format!("Error writing to file: {}", filename))?;

            // **New Code Start**
            // Also save the image to "./tmp/dalle-1.png"
            let additional_filename = "./tmp/dalle-1.png";
            let mut additional_file = File::create(additional_filename)
                .await
                .with_context(|| format!("Error creating file: {}", additional_filename))?;
            additional_file
                .write_all(&image_bytes)
                .await
                .with_context(|| format!("Error writing to file: {}", additional_filename))?;
            println!("Also saved to {}", additional_filename);
            // **New Code End**

            // Optionally save the image to an additional location
            if let Some(extra_folder) = extra_save_folder {
                let extra_filename = format!("{}/{}.{}", extra_folder, timestamp, extension);
                let mut extra_file = File::create(&extra_filename)
                    .await
                    .with_context(|| format!("Error creating file: {}", extra_filename))?;
                extra_file
                    .write_all(&image_bytes)
                    .await
                    .with_context(|| format!("Error writing to file: {}", extra_filename))?;
            }

            println!("Saved {}", filename);
        } else {
            eprintln!("Invalid data URL for image at index {}", index);
        }
    }

    Ok(())
}
// async fn process_images(
//     timestamp: &str,
//     json_path: &str,
//     extra_save_folder: Option<&str>,
// ) -> Result<()> {
//     // Read the JSON file asynchronously
//     let json_data = tokio::fs::read_to_string(json_path).await?;
//
//     // Parse the JSON data into the FalData struct
//     let data: FalData = serde_json::from_str(&json_data)?;
//
//     // Regex to match data URLs
//     let data_url_regex = Regex::new(r"data:(?P<mime>[\w/]+);base64,(?P<data>.+)")?;
//
//     for (index, image) in data.images.iter().enumerate() {
//         // Match the data URL and extract MIME type and base64 data
//         if let Some(captures) = data_url_regex.captures(&image.url) {
//             let mime_type = captures.name("mime").unwrap().as_str();
//             let base64_data = captures.name("data").unwrap().as_str();
//
//             // Decode the base64 data
//             let image_bytes = general_purpose::STANDARD.decode(base64_data)?;
//
//             // Determine the file extension based on the MIME type
//             let extension = match mime_type {
//                 "image/png" => "png",
//                 "image/jpeg" => "jpg",
//                 _ => "bin", // Default to binary if unknown type
//             };
//
//             // Construct the filename using the timestamp and extension
//             let filename = format!("tmp/fal_images/{}.{}", timestamp, extension);
//
//             // Save the image bytes to a file asynchronously
//             let mut file = File::create(&filename)
//                 .await
//                 .with_context(|| format!("Error creating file: {}", filename))?;
//             file.write_all(&image_bytes)
//                 .await
//                 .with_context(|| format!("Error writing to file: {}", filename))?;
//
//             // Optionally save the image to an additional location
//             if let Some(extra_folder) = extra_save_folder {
//                 let extra_filename = format!("{}/{}.{}", extra_folder, timestamp, extension);
//                 let mut extra_file = File::create(&extra_filename)
//                     .await
//                     .with_context(|| format!("Error creating file: {}", extra_filename))?;
//                 extra_file
//                     .write_all(&image_bytes)
//                     .await
//                     .with_context(|| format!("Error writing to file: {}", extra_filename))?;
//             }
//
//             println!("Saved {}", filename);
//         } else {
//             eprintln!("Invalid data URL for image at index {}", index);
//         }
//     }
//
//     Ok(())
// }

pub async fn create_turbo_image_in_folder(
    prompt: String,
    suno_save_folder: &String,
) -> Result<()> {
    // Can I move this into it's own function that takes a prompt?
    // So here is as silly place I can run fal
    let client = FalClient::new(ClientCredentials::from_env());

    // let model = "fal-ai/stable-cascade";
    let model = "fal-ai/fast-turbo-diffusion";

    let res = client
        .run(
            model,
            serde_json::json!({
                "prompt": prompt,
                "image_size": "landscape_16_9",
            }),
        )
        .await
        .unwrap();

    let raw_json = res.bytes().await.unwrap();
    let timestamp = chrono::Utc::now().timestamp();
    let json_path = format!("tmp/fal_responses/{}.json", timestamp);
    tokio::fs::write(&json_path, &raw_json).await.unwrap();

    // This is not the folder
    // let save_folder = "tmp/fal_images";
    let _ = process_images(
        &timestamp.to_string(),
        &json_path,
        Some(&suno_save_folder),
    )
    .await;

    Ok(())
}

// This is too specific
pub async fn create_turbo_image(prompt: String) -> Result<()> {
    // Can I move this into it's own function that takes a prompt?
    // So here is as silly place I can run fal
    let client = FalClient::new(ClientCredentials::from_env());

    // let model = "fal-ai/stable-cascade/sote-diffusion";
    // let model = "fal-ai/stable-cascade";
    let model = "fal-ai/fast-turbo-diffusion";

    let res = client
        .run(
            model,
            serde_json::json!({
                "prompt": prompt,
                "image_size": "landscape_16_9",
            }),
        )
        .await
        .unwrap();

    let raw_json = res.bytes().await.unwrap();
    let timestamp = chrono::Utc::now().timestamp();
    let json_path = format!("tmp/fal_responses/{}.json", timestamp);
    tokio::fs::write(&json_path, &raw_json).await.unwrap();
    let _ = process_images(&timestamp.to_string(), &json_path, None).await;

    Ok(())
}

// Function to submit the request to the fal 'sadtalker' model
async fn fal_submit_sadtalker_request(
    fal_source_image_data_uri: &str,
    fal_driven_audio_data_uri: &str,
) -> Result<String> {
    // Create an asynchronous HTTP client
    let fal_client = FalClient::new(ClientCredentials::from_env());

    // Prepare the JSON payload specific to fal
    // let fal_arguments = json!({
    //     "source_image_url": fal_source_image_data_uri,
    //     "driven_audio_url": fal_driven_audio_data_uri,
    // });

    // Send a POST request to the fal 'sadtalker' API endpoint
    let fal_response = fal_client
        .run(
            "https://api.fal-ai.com/models/sadtalker",
            serde_json::json!({
                "source_image_url": fal_source_image_data_uri,
                "driven_audio_url": fal_driven_audio_data_uri,
            }),
        ).await.unwrap();

    // Check if the request was successful
    if fal_response.status().is_success() {
        // Retrieve the response body as text
        let fal_result = fal_response.text().await?;
        Ok(fal_result)
    } else {
        // Return an error with the status code
        Err(anyhow!(format!( "fal request failed with status: {}", fal_response.status())))
    }
}

async fn fal_encode_file_as_data_uri(file_path: &str) -> Result<String> {
    // Open the file asynchronously
    let mut fal_file = File::open(file_path).await?;
    let mut fal_file_data = Vec::new();

    // Read the entire file into the buffer
    fal_file.read_to_end(&mut fal_file_data).await?;

    // Encode the file data to Base64
    let fal_encoded_data = general_purpose::STANDARD.encode(&fal_file_data);

    // Convert the encoded data to a String
    let fal_encoded_data_string = String::from_utf8(fal_encoded_data.into_bytes())?;

    // Guess the MIME type based on the file extension
    let fal_mime_type = MimeGuess::from_path(file_path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    // Create the data URI for fal
    let fal_data_uri = format!(
        "data:{};base64,{}",
        fal_mime_type,
        fal_encoded_data_string
    );

    Ok(fal_data_uri)
}

// Function to read and encode a file into a Base64 data URI for fal
// async fn fal_encode_file_as_data_uri(file_path: &str) -> Result<String> {
//     // Open the file asynchronously
//     let mut fal_file = File::open(file_path).await?;
//     let mut fal_file_data = Vec::new();
//
//     // Read the entire file into the buffer
//     fal_file.read_to_end(&mut fal_file_data).await?;
//
//     // Encode the file data to Base64
//     let fal_encoded_data = general_purpose::STANDARD.decode(&fal_file_data)?;
//
//     // Guess the MIME type based on the file extension
//     let fal_mime_type = MimeGuess::from_path(file_path)
//         .first_or_octet_stream()
//         .essence_str()
//         .to_string();
//
//     // Create the data URI for fal
//     let fal_data_uri = format!("data:{};base64,{}", fal_mime_type, fal_encoded_data);
//
//     Ok(fal_data_uri)
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obs::obs;
    use serde_json::{json, Error, Value};

    #[tokio::test]
    async fn test_parsing_fal() {
        // Saved w/ Text
        // let tmp_file_path = "tmp/fal_responses/1726345706.json";
        //
        // Saved with bytes
        let timestamp = "1726347150";
        let tmp_file_path = format!("tmp/fal_responses/{}.json", timestamp);

        process_images(&timestamp, &tmp_file_path, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_fal() {
        let prompt = "Magical Cat wearing a wizard hat";
        create_turbo_image(prompt.to_string()).await;
    }
}
