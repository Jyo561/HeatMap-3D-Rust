use dotenvy::dotenv;
use serde::Deserialize;
use std::collections::HashMap;
use std::{env, f64::consts::PI};
use svg::node::element::{Group, Path, Polygon, Text as SvgText};
use svg::node::Text as TextNode;
use svg::Document;

// Larger canvas to prevent crowding
const VIEW_WIDTH: f64 = 1400.0;
const VIEW_HEIGHT: f64 = 1000.0;

// --- GITHUB API STRUCTS ---
#[derive(Deserialize, Debug)]
struct GithubResponse { data: Data }
#[derive(Deserialize, Debug)]
struct Data { user: User }
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct User {
    contributions_collection: ContributionsCollection,
    repositories: Repositories,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContributionsCollection {
    total_commit_contributions: i32,
    total_issue_contributions: i32,
    total_pull_request_contributions: i32,
    total_pull_request_review_contributions: i32,
    total_repository_contributions: i32,
    contribution_calendar: ContributionCalendar,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContributionCalendar {
    total_contributions: i32,
    weeks: Vec<Week>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Week { contribution_days: Vec<Day> }
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Day { contribution_count: i32 }
#[derive(Deserialize, Debug)]
struct Repositories { nodes: Vec<RepoNode> }
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RepoNode {
    stargazer_count: i32,
    fork_count: i32,
    languages: Option<Languages>,
}
#[derive(Deserialize, Debug)]
struct Languages { edges: Vec<LangEdge> }
#[derive(Deserialize, Debug)]
struct LangEdge { size: i32, node: LangNode }
#[derive(Deserialize, Debug)]
struct LangNode { name: String, color: Option<String> }

// --- HELPERS ---

fn project(x: f64, y: f64, z: f64) -> (f64, f64) {
    let angle = 30.0_f64.to_radians();
    // Increase multiplier to 20.0 for a much longer/wider "extended" look
    let scale = 20.0; 
    let sx = 400.0 + (x - y) * angle.cos() * scale;
    let sy = 300.0 + (x + y) * angle.sin() * scale - z;
    (sx, sy)
}

fn darken(hex: &str, amount: f64) -> String {
    let hex = hex.trim_start_matches('#');
    let r = (u8::from_str_radix(&hex[0..2], 16).unwrap_or(200) as f64 * amount) as u8;
    let g = (u8::from_str_radix(&hex[2..4], 16).unwrap_or(200) as f64 * amount) as u8;
    let b = (u8::from_str_radix(&hex[4..6], 16).unwrap_or(200) as f64 * amount) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn get_seasonal_color(week_idx: usize, count: i32) -> String {
    if count == 0 { return "#ebedf0".to_string(); }
    match week_idx {
        0..=12  => "#c6e48b".to_string(), // Q1
        13..=25 => "#f4e04d".to_string(), // Q2
        26..=38 => "#a3a3a3".to_string(), // Q3
        _       => "#d1a3d1".to_string(), // Q4
    }
}

// --- DRAWING ---

fn draw_3d_heatmap(weeks: &[Week]) -> Group {
    let mut g = Group::new();
    for (x, week) in weeks.iter().enumerate() {
        for (y, day) in week.contribution_days.iter().enumerate() {
            let h = (day.contribution_count as f64 * 5.0).max(2.0); // Taller bars
            let (xf, yf) = (x as f64, y as f64);
            let color = get_seasonal_color(x, day.contribution_count);

            let p_top_back = project(xf, yf, h);
            let p_top_left = project(xf + 1.0, yf, h);
            let p_top_right = project(xf, yf + 1.0, h);
            let p_top_front = project(xf + 1.0, yf + 1.0, h);
            let p_bot_left = project(xf + 1.0, yf, 0.0);
            let p_bot_right = project(xf, yf + 1.0, 0.0);
            let p_bot_front = project(xf + 1.0, yf + 1.0, 0.0);

            g = g.add(Polygon::new().set("fill", darken(&color, 0.8)).set("points", format!("{},{} {},{} {},{} {},{}", p_top_left.0, p_top_left.1, p_top_front.0, p_top_front.1, p_bot_front.0, p_bot_front.1, p_bot_left.0, p_bot_left.1)))
                 .add(Polygon::new().set("fill", darken(&color, 0.6)).set("points", format!("{},{} {},{} {},{} {},{}", p_top_right.0, p_top_right.1, p_top_front.0, p_top_front.1, p_bot_front.0, p_bot_front.1, p_bot_right.0, p_bot_right.1)))
                 .add(Polygon::new().set("fill", color.as_str()).set("points", format!("{},{} {},{} {},{} {},{}", p_top_back.0, p_top_back.1, p_top_left.0, p_top_left.1, p_top_front.0, p_top_front.1, p_top_right.0, p_top_right.1)));
        }
    }
    g
}

fn draw_donut_chart(lang_stats: HashMap<String, (i32, String)>) -> Group {
    // Moved lower to (180, 820) to avoid heatmap overlap
    let mut g = Group::new().set("transform", "translate(180, 820)");
    let mut sorted_langs: Vec<_> = lang_stats.into_iter().collect();
    sorted_langs.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    
    let total: i32 = sorted_langs.iter().map(|v| v.1.0).sum();
    let mut current_angle: f64 = 0.0;
    let radius = 90.0;
    let inner_radius = 60.0;

    for (i, (name, (size, color))) in sorted_langs.iter().enumerate() {
        let slice_angle = (*size as f64 / total as f64) * 2.0 * PI;
        let x1 = current_angle.cos() * radius;
        let y1 = current_angle.sin() * radius;
        let x2 = (current_angle + slice_angle).cos() * radius;
        let y2 = (current_angle + slice_angle).sin() * radius;
        let x3 = (current_angle + slice_angle).cos() * inner_radius;
        let y3 = (current_angle + slice_angle).sin() * inner_radius;
        let x4 = current_angle.cos() * inner_radius;
        let y4 = current_angle.sin() * inner_radius;

        let large_arc = if slice_angle > PI { 1 } else { 0 };
        let d = format!("M {} {} A {} {} 0 {} 1 {} {} L {} {} A {} {} 0 {} 0 {} {} Z", x1, y1, radius, radius, large_arc, x2, y2, x3, y3, inner_radius, inner_radius, large_arc, x4, y4);
        g = g.add(Path::new().set("d", d).set("fill", color.as_str()));
        
        // Dynamic multi-column legend
        let col = i / 8;
        let row = i % 8;
        let x_off = 120 + (col * 140);
        let y_off = (row as i32 * 22) - 80;

        g = g.add(Polygon::new().set("points", "0,0 12,0 12,12 0,12").set("fill", color.as_str()).set("transform", format!("translate({}, {})", x_off, y_off)));
        g = g.add(SvgText::new().set("x", x_off + 18).set("y", y_off + 10).set("fill", "#586069").set("font-size", 14).add(TextNode::new(name)));
        
        current_angle += slice_angle;
    }
    g
}

fn draw_radar_chart(stats: &[i32; 5]) -> Group {
    // Pushed far right and slightly up
    let mut g = Group::new().set("transform", "translate(1150, 250)"); 
    let labels = ["Commit", "Issue", "PullReq", "Review", "Repo"];
    let max_r = 110.0;
    
    for r in [0.25, 0.5, 0.75, 1.0] {
        let mut points = String::new();
        for i in 0..5 {
            let a = (i as f64 * 72.0 - 90.0).to_radians();
            points.push_str(&format!("{},{} ", a.cos() * max_r * r, a.sin() * max_r * r));
        }
        g = g.add(Polygon::new().set("points", points).set("fill", "none").set("stroke", "#e1e4e8"));
    }

    let mut data_points = String::new();
    for (i, &val) in stats.iter().enumerate() {
        let val_scaled = (val as f64 + 1.0).log10() / 4.0; 
        let a = (i as f64 * 72.0 - 90.0).to_radians();
        let r = val_scaled.min(1.0) * max_r;
        data_points.push_str(&format!("{},{} ", a.cos() * r, a.sin() * r));
        g = g.add(SvgText::new().set("x", a.cos() * 140.0 - 25.0).set("y", a.sin() * 140.0).set("fill", "#586069").set("font-size", 15).add(TextNode::new(labels[i])));
    }
    g.add(Polygon::new().set("points", data_points).set("fill", "rgba(46, 160, 67, 0.2)").set("stroke", "#2ea043").set("stroke-width", 2))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN required");
    let username = env::var("GITHUB_USER").expect("GITHUB_USER required");

    let client = reqwest::blocking::Client::new();
    let query = r#"query($login:String!){user(login:$login){contributionsCollection{totalCommitContributions totalIssueContributions totalPullRequestContributions totalPullRequestReviewContributions totalRepositoryContributions contributionCalendar{totalContributions weeks{contributionDays{contributionCount}}}} repositories(first:100,ownerAffiliations:OWNER){nodes{stargazerCount forkCount languages(first:10,orderBy:{field:SIZE,direction:DESC}){edges{size node{name color}}}}}}}"#;

    let res: GithubResponse = client.post("https://api.github.com/graphql").bearer_auth(token).header("User-Agent", "rust").json(&serde_json::json!({"query":query,"variables":{"login":username}})).send()?.json()?;
    let user = res.data.user;

    let mut langs = HashMap::new();
    let mut total_stars = 0;
    let mut total_forks = 0;
    for repo in &user.repositories.nodes {
        total_stars += repo.stargazer_count;
        total_forks += repo.fork_count;
        if let Some(l) = &repo.languages {
            for edge in &l.edges {
                let entry = langs.entry(edge.node.name.clone()).or_insert((0, edge.node.color.clone().unwrap_or("#cccccc".to_string())));
                entry.0 += edge.size;
            }
        }
    }

    let mut doc = Document::new().set("viewBox", (0, 0, VIEW_WIDTH, VIEW_HEIGHT)).set("style", "background:#ffffff; font-family: sans-serif;");
    
    doc = doc.add(draw_3d_heatmap(&user.contributions_collection.contribution_calendar.weeks));
    doc = doc.add(draw_donut_chart(langs));
    doc = doc.add(draw_radar_chart(&[user.contributions_collection.total_commit_contributions, user.contributions_collection.total_issue_contributions, user.contributions_collection.total_pull_request_contributions, user.contributions_collection.total_pull_request_review_contributions, user.contributions_collection.total_repository_contributions]));

    // Footer - Placed at safe bottom
    let footer_text = format!("{} contributions    ⭐ {}     {}", user.contributions_collection.contribution_calendar.total_contributions, total_stars, total_forks);
    doc = doc.add(SvgText::new().set("x", VIEW_WIDTH / 2.0).set("y", VIEW_HEIGHT - 40.0).set("fill", "#586069").set("text-anchor", "middle").set("font-size", 24).set("font-weight", "bold").add(TextNode::new(footer_text)));

    svg::save("github_extended_no_overlap.svg", &doc)?;
    println!("Generated: github_extended_no_overlap.svg");
    Ok(())
}
