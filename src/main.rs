use futures::future::join_all;
use image::io::Reader as ImageReader;
use minifb::{Key, Window, WindowOptions};
use rand::Rng;
use reqwest;
use std::{thread, time::Duration};
use tokio;
use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

async fn fetch_meme_urls() -> Vec<String> {
    println!("🔍 Fetching meme URLs from Reddit...");
    let client = reqwest::Client::new();
    let meme_subreddits = vec![
        "https://www.reddit.com/r/memes/top/.json?limit=10",
        "https://www.reddit.com/r/dankmemes/top/.json?limit=10",
    ];

    let mut all_meme_urls = Vec::new();
    for subreddit_url in meme_subreddits {
        if let Ok(response) = client
            .get(subreddit_url)
            .header("User-Agent", "MemeViewer/1.0")
            .send()
            .await
        {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                if let Some(posts) = json["data"]["children"].as_array() {
                    let meme_urls: Vec<String> = posts
                        .iter()
                        .filter_map(|post| {
                            let url = post["data"]["url"].as_str()?;
                            Some(url.to_string())
                        })
                        .filter(|url| {
                            url.ends_with(".jpg")
                                || url.ends_with(".png")
                                || url.ends_with(".gif")
                                || url.contains("v.redd.it")
                        })
                        .collect();
                    all_meme_urls.extend(meme_urls);
                }
            }
        }
    }
    println!("✅ Found {} meme URLs", all_meme_urls.len());
    all_meme_urls
}

async fn load_images(urls: &[String]) -> Vec<Vec<u32>> {
    println!("📥 Loading {} images...", urls.len());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let futures = urls.iter().enumerate().map(|(i, url)| {
        let client_clone = client.clone();
        async move {
            println!("  🌐 Attempting to load image {}/{}: {}", i + 1, urls.len(), url);
            match client_clone.get(url).send().await {
                Ok(response) => {
                    println!("  ✅ Got response for image {}", i + 1);
                    match response.bytes().await {
                        Ok(bytes) => {
                            println!("  📦 Downloaded image {} ({} bytes)", i + 1, bytes.len());
                            match ImageReader::new(std::io::Cursor::new(bytes))
                                .with_guessed_format()
                                .unwrap()
                                .decode()
                            {
                                Ok(img) => {
                                    println!("  🎨 Successfully decoded image {}", i + 1);
                                    let rgb_img = img.resize_exact(
                                        400,
                                        300,
                                        image::imageops::FilterType::Nearest,
                                    );
                                    Some(
                                        rgb_img
                                            .to_rgb8()
                                            .pixels()
                                            .map(|p| {
                                                ((p[0] as u32) << 16)
                                                    | ((p[1] as u32) << 8)
                                                    | p[2] as u32
                                            })
                                            .collect(),
                                    )
                                }
                                Err(e) => {
                                    println!("  ❌ Failed to decode image {}: {}", i + 1, e);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Failed to download image {}: {}", i + 1, e);
                            None
                        }
                    }
                }
                Err(e) => {
                    println!("  ❌ Failed to fetch image {}: {}", i + 1, e);
                    None
                }
            }
        }
    });

    let results: Vec<Vec<u32>> = join_all(futures)
        .await
        .into_iter()
        .filter_map(|x| x)
        .collect();
    
    if results.is_empty() {
        println!("⚠️ WARNING: No images were successfully loaded!");
    } else {
        println!("✅ Successfully loaded {}/{} images", results.len(), urls.len());
    }
    results
}

#[tokio::main]
async fn main() {
    println!("🚀 Starting Meme Attack!");
    
    let mut attempts = 0;
    let max_attempts = 3;
    
    while attempts < max_attempts {
        attempts += 1;
        println!("📝 Attempt {} of {}", attempts, max_attempts);
        
        match fetch_meme_urls().await {
            urls if !urls.is_empty() => {
                match load_images(&urls).await {
                    memes if !memes.is_empty() => {
                        println!("✨ Successfully loaded {} memes, starting display loop", memes.len());
                        run_meme_loop(memes);
                        break;
                    }
                    _ => println!("⚠️ Failed to load any images, retrying..."),
                }
            }
            _ => println!("⚠️ Failed to fetch URLs, retrying..."),
        }
        
        thread::sleep(Duration::from_secs(5));
    }
    
    if attempts >= max_attempts {
        println!("❌ Failed after {} attempts, exiting.", max_attempts);
    }
}

fn run_meme_loop(memes: Vec<Vec<u32>>) {
    let width: usize = 400;
    let height: usize = 300;
    let mut rng = rand::thread_rng();

    loop {
        println!("🎯 Creating new window...");
        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

        let x = rng.gen_range(0..(screen_width - width as i32));
        let y = rng.gen_range(0..(screen_height - height as i32));
        println!("📍 Window position: ({}, {})", x, y);

        let mut window = Window::new(
            "Meme Attack!",
            width,
            height,
            WindowOptions {
                borderless: false,
                title: true,
                resize: false,
                topmost: true,
                ..WindowOptions::default()
            },
        )
        .unwrap_or_else(|e| panic!("Unable to open window: {}", e));

        window.limit_update_rate(Some(Duration::from_millis(100)));

        println!("🖼️ Displaying meme {} of {}", 
            rng.gen_range(0..memes.len()) + 1, 
            memes.len()
        );

        let buffer = &memes[rng.gen_range(0..memes.len())];

        while window.is_open() && !window.is_key_down(Key::Escape) {
            window.update_with_buffer(&buffer, width, height).unwrap();
            thread::sleep(Duration::from_millis(100));
        }
        println!("🔄 Window closed, creating new one in 1 second...");
        thread::sleep(Duration::from_secs(1));
    }
}
