use crate::audio;
use crate::dalle;
use crate::dalle::GenerateImage;
use crate::images;
use crate::obs_scenes;
use crate::obs_source;
use crate::openai;
use anyhow::Result;
use chrono::{DateTime, Utc};
use obws::requests::custom::source_settings::ImageSource;
use obws::requests::custom::source_settings::SlideshowFile;
use obws::Client as OBSClient;
use rodio::*;
use std::fs::create_dir;
use std::path::Path;
use std::path::PathBuf;

pub async fn telephone(
    obs_client: &OBSClient,
    sink: &Sink,
    url: String,
    prompt: String,
    num_connections: u8,
    ai_image_req: &impl GenerateImage,
) -> Result<String, anyhow::Error> {
    let now: DateTime<Utc> = Utc::now();
    let folder = format!("telephone/{}", now.timestamp());
    let new_tele_folder = format!("./archive/{}", folder);
    let _ = create_dir(new_tele_folder.clone());

    let og_file =
        format!("/home/begin/code/subd/archive/{}/original.png", folder);
    if let Err(e) = images::download_image(url.clone(), og_file.clone()).await {
        println!("Error Downloading Image: {} | {:?}", og_file.clone(), e);
    }

    let first_description =
        openai::ask_gpt_vision2("", Some(&url)).await.map(|m| m)?;

    let description = format!("{} {}", first_description, prompt);
    println!("First GPT Vision Description: {}", description);
    let mut dalle_path = ai_image_req
        .generate_image(description, Some(folder.clone()), false)
        .await;
    if dalle_path == "".to_string() {
        return Err(anyhow::anyhow!("Dalle Path is empty"));
    }
    let og_dalle_path = dalle_path.clone();

    let mut dalle_path_bufs = vec![];
    for _ in 1..num_connections {
        println!("\n\tAsking GPT VISION: {}", dalle_path.clone());
        let description = match openai::ask_gpt_vision2(&dalle_path, None).await
        {
            Ok(desc) => desc,
            Err(e) => {
                eprintln!("Error asking GPT Vision: {}", e);
                continue;
            }
        };

        let prompt = format!("{} {}", description, prompt);
        let req = dalle::DalleRequest {
            prompt: prompt.clone(),
            username: "beginbot".to_string(),
            amount: 1,
        };
        println!("\n\tSaving Image to: {}", folder.clone());

        dalle_path = req
            .generate_image(prompt, Some(folder.clone()), false)
            .await;
        if dalle_path != "".to_string() {
            let dp = dalle_path.clone();
            dalle_path_bufs.push(PathBuf::from(dp))
        }
    }

    let _ =
        update_obs_telephone_scene(obs_client, og_file, dalle_path_bufs).await;
    let _ = audio::play_sound(&sink, "8bitmackintro".to_string()).await;

    Ok(dalle_path)
}

pub async fn update_obs_telephone_scene(
    obs_client: &OBSClient,
    og_dalle_path: String,
    dalle_path_bufs: Vec<PathBuf>,
) -> Result<()> {
    // Create our list of files for the slideshow
    let dp = og_dalle_path.clone();
    let fp = Path::new(&dp);
    let slideshow_file = SlideshowFile {
        value: fp,
        hidden: false,
        selected: true,
    };
    let mut files: Vec<SlideshowFile> = vec![slideshow_file];

    let mut slideshow_files: Vec<SlideshowFile> = dalle_path_bufs
        .iter()
        .map(|path_string| SlideshowFile {
            value: &path_string,
            hidden: false,
            selected: false,
        })
        .collect();
    files.append(&mut slideshow_files);

    let source = "Telephone-Slideshow".to_string();
    let _ =
        obs_source::update_slideshow_source(obs_client, source, files).await;

    let source = "OG-Telephone-Image".to_string();
    let _ = obs_source::update_image_source(obs_client, source, og_dalle_path)
        .await;

    let scene = "TelephoneScene";
    let _ = obs_scenes::change_scene(&obs_client, scene).await;
    Ok(())
}

// TODO: I don't like the name
pub async fn create_screenshot_variation(
    sink: &Sink,
    obs_client: &OBSClient,
    filename: String,
    ai_image_req: &impl GenerateImage,
    prompt: String,
    source: String,
) -> Result<String, String> {
    // let _ = audio::play_sound(&sink).await;

    let _ = obs_source::save_screenshot(&obs_client, &source, &filename).await;

    let description = openai::ask_gpt_vision2(&filename, None)
        .await
        .map_err(|e| e.to_string())?;

    let new_description = format!(
        "{} {} . The most important thing to focus on is: {}",
        prompt, description, prompt
    );

    let dalle_path = ai_image_req
        .generate_image(new_description, Some("timelapse".to_string()), false)
        .await;

    if dalle_path == "".to_string() {
        return Err("Dalle Path is empty".to_string());
    }

    println!("Dalle Path: {}", dalle_path);
    Ok(dalle_path)
}
