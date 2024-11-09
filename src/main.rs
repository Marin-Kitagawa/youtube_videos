use clap::Parser;
use csv::Writer;
use reqwest::Client;
use serde_json::Value;

async fn fetch_channel_id(
    api_key: &str,
    handle: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!(
        "https://www.googleapis.com/youtube/v3/channels?key={}&part=id&forHandle={}",
        api_key, handle
    );
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        println!(
            "Unable to fetch Channel ID. API request failed with status {}",
            response.status()
        );
        println!("Response Body: {}", response.text().await?);
        return Err("API request failed".into());
    }

    let json: Value = response.json().await?;

    if let Some(error) = json.get("error") {
        println!("Error: {}", error);
        return Err("Error".into());
    }

    let channel_id = json["items"][0]["id"].as_str().unwrap();
    println!("Channel ID: {}", channel_id);
    Ok(channel_id.to_string())
}

async fn fetch_videos(
    api_key: &str,
    channel_id: String,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut videos = Vec::new();
    let mut page_token = String::new();
    loop {
        let url = format!(
                "https://www.googleapis.com/youtube/v3/search?key={}&channelId={}&part=snippet,id&order=date&maxResults=50&type=video&pageToken={}",
                api_key, channel_id, page_token
            );
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            println!(
                "Unable to fetch videos. API request failed with status {}",
                response.status()
            );
            return Err("API request failed".into());
        }

        let json: Value = response.json().await?;

        if let Some(error) = json.get("error") {
            println!("Error: {}", error);
            return Err("Error".into());
        }

        if let Some(items) = json["items"].as_array() {
            videos.extend(items.clone())
        }

        if let Some(next_page_token) = json["nextPageToken"].as_str() {
            page_token = next_page_token.to_string();
        } else {
            break;
        }
    }

    Ok(videos)
}

fn write_to_csv(handle: String, videos: Vec<Value>) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = Writer::from_path(format!("{}.csv", handle.as_str().replace("@", "")))?;

    writer.write_record(&["Video ID", "Title", "Description", "Published At"])?;

    for video in videos {
        let snippet = &video["snippet"];
        writer.write_record(&[
            video["id"]["videoId"].as_str().unwrap_or(""),
            snippet["title"].as_str().unwrap_or(""),
            snippet["description"].as_str().unwrap_or(""),
            snippet["publishedAt"].as_str().unwrap_or(""),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

// Simple program to fetch videos for a given channel from YouTube and save it to a CSV file
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // YouTube API key to access YouTube Data API v3
    api_key: String,

    // Handle for the channel to fetch videos. It can be prepended with `@`
    channel_handle: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let channel_id = fetch_channel_id(&args.api_key, &args.channel_handle).await?;

    match fetch_videos(&args.api_key, channel_id).await {
        Ok(videos) => {
            println!("Fetched {} videos", videos.len());

            if videos.is_empty() {
                println!("No videos found");
            } else {
                write_to_csv(args.channel_handle, videos)?;
                println!("Videos written to a CSV file successfully");
            }
        }
        Err(e) => {
            println!("Error fetching videos: {}", e);
            if let Some(error) = e.downcast_ref::<reqwest::Error>() {
                if let Some(status) = error.status() {
                    println!("API request failed with status {}", status);
                }
            }
        }
    }
    Ok(())
}
