use futures::future::join_all;
use image::io::Reader as ImageReader;
use minifb::{Key, Window, WindowOptions};
use rand::Rng;
use reqwest;
use std::{thread, time::Duration};
use tokio;

async fn fetch_meme_urls() -> Vec<String> {
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
    all_meme_urls
}

async fn load_images(urls: &[String]) -> Vec<Vec<u32>> {
    let client = reqwest::Client::new();
    let futures = urls.iter().map(|url| {
        let client_clone = client.clone();
        async move {
            match client_clone.get(url).send().await {
                Ok(response) => match response.bytes().await {
                    Ok(bytes) => {
                        match ImageReader::new(std::io::Cursor::new(bytes))
                            .with_guessed_format()
                            .unwrap()
                            .decode()
                        {
                            Ok(img) => {
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
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            }
        }
    });

    join_all(futures)
        .await
        .into_iter()
        .filter_map(|x| x)
        .collect()
}

#[tokio::main]
async fn main() {
    let width = 400;
    let height = 300;

    let meme_urls = fetch_meme_urls().await;
    assert!(!meme_urls.is_empty(), "No memes found!");

    let memes = load_images(&meme_urls).await;
    assert!(!memes.is_empty(), "No memes processed!");

    let mut rng = rand::thread_rng();

    loop {
        let x = rng.gen_range(100..800);
        let y = rng.gen_range(100..600);

        let mut window = Window::new(
            "Meme Attack!",
            width,
            height,
            WindowOptions {
                borderless: false,
                title: true,
                resize: false,
                ..WindowOptions::default()
            },
        )
        .unwrap_or_else(|_| panic!("Unable to open window"));
        window.limit_update_rate(Some(Duration::from_millis(100)));

        let buffer = &memes[rng.gen_range(0..memes.len())];

        while window.is_open() && !window.is_key_down(Key::Escape) {
            window.update_with_buffer(&buffer, width, height).unwrap();
            thread::sleep(Duration::from_millis(100));
        }
        thread::sleep(Duration::from_secs(1));
    }
}
