pub const PRESET_HOST_LOCAL: &str = "http://localhost:3000";
pub const PRESET_HOST_SECONDARY: &str = "http://localhost:3010";

pub fn get_preset_hosts() -> Vec<String> {
    let mut hosts = vec![
        PRESET_HOST_LOCAL.to_string(),
        PRESET_HOST_SECONDARY.to_string(),
    ];
    if let Ok(remote) = std::env::var("DOER_REMOTE_URL") {
        let norm = normalize_url(&remote);
        if !hosts.contains(&norm) {
            hosts.push(norm);
        }
    }
    hosts
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub api_token: Option<String>,
}

pub fn normalize_url(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if !s.starts_with("http://") && !s.starts_with("https://") {
        s = format!("http://{}", s);
    }
    // Trim trailing slashes
    while s.ends_with('/') {
        s.pop();
    }
    // Strip trailing /api/v1 if provided
    if s.ends_with("/api/v1") {
        s.truncate(s.len() - 7);
    }
    while s.ends_with('/') {
        s.pop();
    }
    s
}

impl Config {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: normalize_url(base_url),
            api_token: std::env::var("DOER_API_KEY")
                .or_else(|_| std::env::var("DOER_API_TOKEN"))
                .ok(),
        }
    }

    pub fn from_env_and_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut chosen_url: Option<String> = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--url" | "--host" | "-u" => {
                    if i + 1 < args.len() {
                        chosen_url = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                arg if arg.starts_with("--url=") => {
                    chosen_url = Some(arg.trim_start_matches("--url=").to_string());
                }
                arg if arg.starts_with("--host=") => {
                    chosen_url = Some(arg.trim_start_matches("--host=").to_string());
                }
                "--local" => {
                    chosen_url = Some(PRESET_HOST_LOCAL.to_string());
                }
                "--remote" => {
                    let remote = std::env::var("DOER_REMOTE_URL")
                        .unwrap_or_else(|_| PRESET_HOST_SECONDARY.to_string());
                    chosen_url = Some(remote);
                }
                _ => {}
            }
            i += 1;
        }

        let base_url = chosen_url
            .or_else(|| std::env::var("DOER_API_URL").ok())
            .unwrap_or_else(|| PRESET_HOST_LOCAL.to_string());

        Self::new(&base_url)
    }

    pub fn set_url(&mut self, new_url: &str) {
        self.base_url = normalize_url(new_url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("http://localhost:3000/"), "http://localhost:3000");
        assert_eq!(normalize_url("http://localhost:3010/"), "http://localhost:3010");
        assert_eq!(normalize_url("localhost:3000"), "http://localhost:3000");
        assert_eq!(normalize_url("http://localhost:3000/api/v1"), "http://localhost:3000");
        assert_eq!(normalize_url("http://localhost:3000/api/v1/"), "http://localhost:3000");
    }
}
