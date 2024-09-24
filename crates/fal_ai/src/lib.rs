use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use fal_rust::client::{ClientCredentials, FalClient};
use regex::Regex;
use serde::Deserialize;
use tokio::fs::create_dir_all;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub mod models;
pub mod utils;

pub async fn create_image_for_music_video(
    id: String,
    prompt: String,
    index: usize,
) -> Result<()> {
    let client = FalClient::new(ClientCredentials::from_env());
    let model = "fal-ai/fast-sdxl";
    let music_video_folder = format!("./tmp/music_videos/{}/", id);

    run_model_create_and_save_image_with_index(
        &client,
        model,
        prompt,
        index,
        Some(&music_video_folder),
    )
    .await
}

/// Creates an image using the "fal-ai/fast-sdxl" model.
pub async fn create_turbo_image(prompt: String) -> Result<()> {
    let client = FalClient::new(ClientCredentials::from_env());

    let model = "fal-ai/fast-turbo-diffusion";

    let stream_background_path = "./tmp/dalle-1.png";
    run_model_create_and_save_image(
        &client,
        model,
        prompt,
        Some(stream_background_path),
        None,
    )
    .await
}

/// Creates an image using the "fal-ai/fast-sdxl" model.
pub async fn create_fast_sd_image(prompt: String) -> Result<()> {
    let client = FalClient::new(ClientCredentials::from_env());

    let model = "fal-ai/fast-sdxl";

    let stream_background_path = "./tmp/dalle-1.png";
    run_model_create_and_save_image(
        &client,
        model,
        prompt,
        Some(stream_background_path),
        None,
    )
    .await
}

/// Creates an image using the "fal-ai/stable-cascade" model and saves it to the specified folder.
pub async fn create_image_from_prompt_in_folder(
    prompt: String,
    suno_save_folder: &str,
) -> Result<()> {
    let client = FalClient::new(ClientCredentials::from_env());
    let model = "fal-ai/stable-cascade";

    let stream_background_path = "./tmp/dalle-1.png";
    // Create the image
    run_model_create_and_save_image(
        &client,
        model,
        prompt,
        Some(stream_background_path),
        Some(suno_save_folder),
    )
    .await
}

/// Creates a video from the given image file path.
pub async fn create_video_from_image(image_file_path: &str) -> Result<()> {
    // Encode the image file as a data URI
    let fal_source_image_data_uri =
        subd_image_utils::encode_file_as_data_uri(image_file_path).await?;

    let client = FalClient::new(ClientCredentials::from_env());
    let model = "fal-ai/stable-video";

    // Run the model and get the JSON response
    let parameters =
        serde_json::json!({ "image_url": fal_source_image_data_uri });
    let json = run_model_and_get_json(&client, model, parameters).await?;

    // Extract the video URL from the response
    let url = json["video"]["url"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to extract video URL from JSON"))?;

    // Download and save the video
    let video_bytes = subd_image_utils::download_video(url).await?;
    let timestamp = Utc::now().timestamp();
    save_video_bytes(&video_bytes, timestamp).await?;

    Ok(())
}

/// Submits a request to the Sadtalker model.
pub async fn fal_submit_sadtalker_request(
    fal_source_image_data_uri: &str,
    fal_driven_audio_data_uri: &str,
) -> Result<String> {
    println!("Calling to Sadtalker");
    let fal_client = FalClient::new(ClientCredentials::from_env());
    let model = "fal-ai/sadtalker";

    // Prepare the parameters
    let parameters = serde_json::json!({
        "source_image_url": fal_source_image_data_uri,
        "driven_audio_url": fal_driven_audio_data_uri,
    });

    // Run the model and get the text response
    run_model_and_get_text(&fal_client, model, parameters).await
}

/// Helper function to create an image using the specified model.
async fn run_model_create_and_save_image_with_index(
    client: &FalClient,
    model: &str,
    prompt: String,
    index: usize,
    extra_save_folder: Option<&str>,
) -> Result<()> {
    println!("\tCreating image with model: {}", model);

    let parameters = serde_json::json!({
        "prompt": prompt,
        "image_size": "landscape_16_9",
    });
    let raw_json =
        run_model_and_get_raw_json(client, model, parameters).await?;

    // Save the raw JSON response to a file
    let timestamp = Utc::now().timestamp();
    save_raw_json_response(&raw_json, timestamp).await?;
    let primary_save_path = format!("./tmp/fal_images/{}", timestamp);

    // Parse and process images from the JSON response
    utils::parse_and_process_images_from_json_for_music_video(
        &raw_json,
        &extra_save_folder.unwrap_or(&primary_save_path),
        index,
        extra_save_folder,
    )
    .await?;

    Ok(())
}

/// Helper function to create an image using the specified model.
async fn run_model_create_and_save_image(
    client: &FalClient,
    model: &str,
    prompt: String,
    stream_background_path: Option<&str>,
    extra_save_folder: Option<&str>,
) -> Result<()> {
    println!("\tCreating image with model: {}", model);

    // Prepare the parameters
    let parameters = serde_json::json!({
        "prompt": prompt,
        "image_size": "landscape_16_9",
    });

    // Run the model and get the raw JSON response
    let raw_json =
        run_model_and_get_raw_json(client, model, parameters).await?;

    println!("Raw JSON: {:?}", raw_json);

    // Save the raw JSON response to a file
    let timestamp = Utc::now().timestamp();
    save_raw_json_response(&raw_json, timestamp).await?;
    let primary_save_path = format!("tmp/fal_images/{}", timestamp);

    // Define filename patterns for saving images
    // let stream_background_path = "./tmp/dalle-1.png";

    // Parse and process images from the JSON response
    utils::parse_and_process_images_from_json(
        &raw_json,
        &primary_save_path,
        stream_background_path,
        extra_save_folder,
    )
    .await?;

    Ok(())
}

/// Runs the specified model with the given parameters and returns the raw JSON response.
async fn run_model_and_get_raw_json(
    client: &FalClient,
    model: &str,
    parameters: serde_json::Value,
) -> Result<bytes::Bytes> {
    let res = client
        .run(model, parameters)
        .await
        .map_err(|e| anyhow!("Failed to run FAL Client: {:?}", e))?;

    let raw_json = res
        .bytes()
        .await
        .with_context(|| "Failed to get bytes from FAL response")?;

    Ok(raw_json)
}

/// Runs the specified model with the given parameters and returns the JSON response.
async fn run_model_and_get_json(
    client: &FalClient,
    model: &str,
    parameters: serde_json::Value,
) -> Result<serde_json::Value> {
    let res = client
        .run(model, parameters)
        .await
        .map_err(|e| anyhow!("Failed to run FAL Client: {:?}", e))?;

    let body = res.text().await?;
    let json: serde_json::Value = serde_json::from_str(&body)?;
    Ok(json)
}

/// Runs the specified model with the given parameters and returns the text response.
async fn run_model_and_get_text(
    client: &FalClient,
    model: &str,
    parameters: serde_json::Value,
) -> Result<String> {
    let response = client
        .run(model, parameters)
        .await
        .map_err(|e| anyhow!("Failed to run client: {:?}", e))?;

    if response.status().is_success() {
        response
            .text()
            .await
            .map_err(|e| anyhow!("Error getting text: {:?}", e))
    } else {
        Err(anyhow!(
            "FAL request failed with status: {}",
            response.status()
        ))
    }
}

/// Saves the raw JSON response to a file.
async fn save_raw_json_response(raw_json: &[u8], timestamp: i64) -> Result<()> {
    let json_dir = subd_types::consts::get_fal_responses_dir();
    let json_path = format!("{}/{}.json", json_dir, timestamp);

    // Ensure the directory exists
    create_dir_all(json_dir).await?;

    // Write the JSON data to the file
    tokio::fs::write(&json_path, raw_json)
        .await
        .with_context(|| format!("Failed to write JSON to {}", json_path))?;

    Ok(())
}

/// Saves the video bytes to a file and returns the file path.
async fn save_video_bytes(
    video_bytes: &[u8],
    timestamp: i64,
) -> Result<String> {
    let video_dir = subd_types::consts::get_ai_videos_dir();
    let filename = format!("{}/{}.mp4", video_dir, timestamp);

    // Ensure the directory exists
    create_dir_all(video_dir).await?;

    // Write the video data to the file
    tokio::fs::write(&filename, video_bytes)
        .await
        .with_context(|| format!("Failed to write video to {}", filename))?;

    println!("Video saved to: {}", filename);
    Ok(filename)
}

pub async fn create_turbo_image_in_folder(prompt: String) -> Result<()> {
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
    let _ = process_images_from_turbo(&timestamp.to_string(), &json_path).await;

    Ok(())
}

#[derive(Deserialize)]
struct FalImage {
    url: String,
    _width: Option<u32>,
    _height: Option<u32>,
    _content_type: Option<String>,
}

#[derive(Deserialize)]
struct FalData {
    images: Vec<FalImage>,
}

async fn process_images_from_turbo(
    timestamp: &str,
    json_path: &str,
) -> Result<()> {
    // Read the JSON file asynchronously
    let json_data = tokio::fs::read_to_string(json_path).await?;

    // Parse the JSON data into the FalData struct
    let data: FalData = serde_json::from_str(&json_data)?;

    // Regex to match data URLs
    let data_url_regex =
        Regex::new(r"data:(?P<mime>[\w/]+);base64,(?P<data>.+)")?;

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
            let filename =
                format!("./tmp/fal_images/{}.{}", timestamp, extension);

            // Save the image bytes to a file asynchronously
            let mut file =
                File::create(&filename).await.with_context(|| {
                    format!("Error creating file: {}", filename)
                })?;
            file.write_all(&image_bytes).await.with_context(|| {
                format!("Error writing to file: {}", filename)
            })?;

            // **New Code Start**
            // Also save the image to "./tmp/dalle-1.png"
            let additional_filename = "./tmp/dalle-1.png";
            let mut additional_file =
                File::create(additional_filename).await.with_context(|| {
                    format!("Error creating file: {}", additional_filename)
                })?;
            additional_file.write_all(&image_bytes).await.with_context(
                || format!("Error writing to file: {}", additional_filename),
            )?;

            println!("Saved {}", filename);
        } else {
            eprintln!("Invalid data URL for image at index {}", index);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_tag::tag;

    #[tokio::test]
    #[tag(fal)]
    async fn test_turbo_sd() {
        let prompt = "raccoon".to_string();
        let res = create_turbo_image_in_folder(prompt).await.unwrap();
        dbg!(res);
        // Now we can test it
        assert!(true);
    }
}
