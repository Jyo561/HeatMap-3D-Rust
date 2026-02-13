use serde::Deserialize;
use std::env;
use svg::node::element::Polygon;
use svg::Document;
use dotenvy::dotenv;

// --- CONFIGURATION ---
const CUBE_SIZE: f64 = 10.0;
const GAP: f64 = 2.0;
const HEIGHT_SCALE: f64 = 4.0;
const VIEW_WIDTH: f64 = 1000.0;
const VIEW_HEIGHT: f64 = 600.0;

// --- GITHUB API STRUCTS ---
#[derive(Deserialize, Debug)]
struct GithubResponse {
    data: Data,
}

#[derive(Deserialize, Debug)]
struct Data {
    user: User,
}

#[derive(Deserialize, Debug)]
struct User {
    #[serde(rename = "contributionsCollection")]
    contributions_collection: ContributionsCollection,
}

#[derive(Deserialize, Debug)]
struct ContributionsCollection {
    #[serde(rename = "contributionCalendar")]
    contribution_calendar: ContributionCalendar,
}

#[derive(Deserialize, Debug)]
struct ContributionCalendar {
    weeks: Vec<Week>,
}

#[derive(Deserialize, Debug)]
struct Week {
    #[serde(rename = "contributionDays")]
    contribution_days: Vec<ContributionDay>,
}

#[derive(Deserialize, Debug, Clone)]
struct ContributionDay {
    #[serde(rename = "contributionCount")]
    count: i32,
    color: String,
}

// --- LOGIC ---

fn fetch_github_data(user: &str, token: &str) -> Result<Vec<Vec<ContributionDay>>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    
    let query = r#"
        query($login: String!) {
          user(login: $login) {
            contributionsCollection {
              contributionCalendar {
                weeks {
                  contributionDays {
                    contributionCount
                    color
                  }
                }
              }
            }
          }
        }
    "#;

    let response = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .header("User-Agent", "rust-github-3d-profile")
        .json(&serde_json::json!({
            "query": query,
            "variables": { "login": user }
        }))
        .send()?
        .json::<GithubResponse>()?;

    let weeks = response.data.user.contributions_collection.contribution_calendar.weeks;
    let data = weeks.into_iter().map(|w| w.contribution_days).collect();
    
    Ok(data)
}

fn project(x: f64, y: f64, z: f64) -> (f64, f64) {
    let angle = 30.0_f64.to_radians();
    let offset_x = VIEW_WIDTH / 2.0 - 250.0; // Adjusted for 52 weeks
    let offset_y = VIEW_HEIGHT / 2.0;

    let sx = offset_x + (x - y) * angle.cos() * (CUBE_SIZE + GAP);
    let sy = offset_y + (x + y) * angle.sin() * (CUBE_SIZE + GAP) - z;
    (sx, sy)
}

fn darken(hex: &str, amount: f64) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 { return "#333333".to_string(); }
    
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

    format!("#{:02x}{:02x}{:02x}", 
        (r as f64 * amount) as u8, 
        (g as f64 * amount) as u8, 
        (b as f64 * amount) as u8
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get credentials from environment
    dotenv().ok();

    let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not found in .env file or environment");
    let username = env::var("GITHUB_USER").expect("GITHUB_USER not found in .env file or environment");

    println!("Fetching data for {}...", username);
    let data = fetch_github_data(&username, &token)?;

    let mut document = Document::new()
        .set("viewBox", (0, 0, VIEW_WIDTH, VIEW_HEIGHT))
        .set("style", "background-color: #0d1117;"); // GitHub Dark Mode Background

    // 2. Render Loop
    for (x, week) in data.iter().enumerate() {
        for (y, day) in week.iter().enumerate() {
            let x_f = x as f64;
            let y_f = y as f64;
            
            // Logarithmic height or linear scale? 
            // Linear looks more like your image.
            let h = day.count as f64 * HEIGHT_SCALE + 2.0;

            let p_top_back = project(x_f, y_f, h);
            let p_top_left = project(x_f + 1.0, y_f, h);
            let p_top_right = project(x_f, y_f + 1.0, h);
            let p_top_front = project(x_f + 1.0, y_f + 1.0, h);

            let p_bot_left = project(x_f + 1.0, y_f, 0.0);
            let p_bot_right = project(x_f, y_f + 1.0, 0.0);
            let p_bot_front = project(x_f + 1.0, y_f + 1.0, 0.0);

            // Left Face
            document = document.add(Polygon::new()
                .set("fill", darken(&day.color, 0.7))
                .set("points", format!("{},{} {},{} {},{} {},{}", 
                    p_top_left.0, p_top_left.1, p_top_front.0, p_top_front.1, 
                    p_bot_front.0, p_bot_front.1, p_bot_left.0, p_bot_left.1)));

            // Right Face
            document = document.add(Polygon::new()
                .set("fill", darken(&day.color, 0.5))
                .set("points", format!("{},{} {},{} {},{} {},{}", 
                    p_top_right.0, p_top_right.1, p_top_front.0, p_top_front.1, 
                    p_bot_front.0, p_bot_front.1, p_bot_right.0, p_bot_right.1)));

            // Top Face
            document = document.add(Polygon::new()
                .set("fill", &*day.color)
                .set("points", format!("{},{} {},{} {},{} {},{}", 
                    p_top_back.0, p_top_back.1, p_top_left.0, p_top_left.1, 
                    p_top_front.0, p_top_front.1, p_top_right.0, p_top_right.1)));
        }
    }

    svg::save("github_3d_real.svg", &document)?;
    println!("Done! Generated 'github_3d_real.svg'");
    Ok(())
}
