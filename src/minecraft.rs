use reqwest;

const MOJANG_SESSION_SERVER: &str = "https://sessionserver.mojang.com";

pub fn has_joined(username: &str, hash: &str) -> reqwest::Result<bool> {
    let resp = reqwest::get(&format!(
        "{}/session/minecraft/hasJoined?username={}&serverId={}",
        MOJANG_SESSION_SERVER, username, hash
    ))?;
    Ok(resp.status() == reqwest::StatusCode::Ok)
}
