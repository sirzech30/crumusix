use crate::lyrics::models::LyricLine;

pub fn parse_lrc(lrc_text: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    
    for line in lrc_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        if line.starts_with('[') {
            if let Some(close_bracket) = line.find(']') {
                let timestamp_str = &line[1..close_bracket];
                let text = line[close_bracket + 1..].trim().to_string();
                
                if let Some(ms) = parse_timestamp(timestamp_str) {
                    lines.push(LyricLine {
                        timestamp_ms: Some(ms),
                        text,
                    });
                }
            }
        } else {
            lines.push(LyricLine {
                timestamp_ms: None,
                text: line.to_string(),
            });
        }
    }
    
    lines.sort_by_key(|l| l.timestamp_ms.unwrap_or(0));
    lines
}

fn parse_timestamp(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    
    let minutes: u64 = parts[0].parse().ok()?;
    
    let sec_part = parts[1];
    let seconds_parts: Vec<&str> = sec_part.split(|c| c == '.' || c == ':').collect();
    
    let seconds: u64 = seconds_parts[0].parse().ok()?;
    let mut ms: u64 = 0;
    
    if seconds_parts.len() > 1 {
        let frac_str = seconds_parts[1];
        if let Ok(frac) = frac_str.parse::<u64>() {
            if frac_str.len() == 2 {
                ms = frac * 10;
            } else if frac_str.len() == 3 {
                ms = frac;
            } else if frac_str.len() == 1 {
                ms = frac * 100;
            }
        }
    }
    
    Some((minutes * 60 + seconds) * 1000 + ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(parse_timestamp("01:12.50"), Some(72500));
        assert_eq!(parse_timestamp("00:04.05"), Some(4050));
        assert_eq!(parse_timestamp("02:00"), Some(120000));
    }

    #[test]
    fn test_parse_lrc() {
        let lrc = "[00:12.50] Hello\n[00:15.00] World";
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].text, "Hello");
        assert_eq!(parsed[0].timestamp_ms, Some(12500));
    }
}
