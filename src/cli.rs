use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "bucatini",
    about = "Republish an NDI source as a Syphon Metal texture"
)]
#[allow(dead_code)]
pub struct Args {
    /// List discovered NDI sources and exit
    #[arg(long)]
    pub list: bool,
    /// NDI source name (case-insensitive substring match)
    #[arg(long)]
    pub source: Option<String>,
    /// Syphon server name to publish under (default: the NDI source name)
    #[arg(long)]
    pub name: Option<String>,
    /// Discovery / capture timeout in milliseconds
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u32,
    /// Verbose logging (resolution, fps)
    #[arg(long)]
    pub verbose: bool,
}

#[allow(dead_code)]
pub fn parse() -> Args {
    Args::parse()
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum SourceMatch {
    None,
    One(usize),
    Many(Vec<usize>),
}

#[allow(dead_code)]
pub fn match_source(names: &[String], query: &str) -> SourceMatch {
    let q = query.to_lowercase();
    if let Some(i) = names.iter().position(|n| n.to_lowercase() == q) {
        return SourceMatch::One(i);
    }
    let hits: Vec<usize> = names
        .iter()
        .enumerate()
        .filter(|(_, n)| n.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect();
    match hits.len() {
        0 => SourceMatch::None,
        1 => SourceMatch::One(hits[0]),
        _ => SourceMatch::Many(hits),
    }
}

#[allow(dead_code)]
pub fn parse_selection(input: &str, count: usize) -> Result<usize, String> {
    let n: usize = input
        .trim()
        .parse()
        .map_err(|_| format!("'{}' is not a number", input.trim()))?;
    if n == 0 || n > count {
        return Err(format!("choose 1..={}", count));
    }
    Ok(n - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        vec![
            "STUDIO (Camera 1)".to_string(),
            "STUDIO (Camera 2)".to_string(),
            "LAPTOP (Screen)".to_string(),
        ]
    }

    #[test]
    fn no_match_returns_none() {
        assert!(matches!(match_source(&names(), "xyz"), SourceMatch::None));
    }

    #[test]
    fn unique_substring_returns_one() {
        match match_source(&names(), "laptop") {
            SourceMatch::One(i) => assert_eq!(i, 2),
            other => panic!("expected One, got {:?}", other),
        }
    }

    #[test]
    fn ambiguous_substring_returns_many() {
        match match_source(&names(), "camera") {
            SourceMatch::Many(v) => assert_eq!(v, vec![0, 1]),
            other => panic!("expected Many, got {:?}", other),
        }
    }

    #[test]
    fn exact_match_wins_over_substring() {
        let n = vec!["Cam".to_string(), "Cam (extra)".to_string()];
        match match_source(&n, "cam") {
            SourceMatch::One(i) => assert_eq!(i, 0),
            other => panic!("expected One, got {:?}", other),
        }
    }

    #[test]
    fn selection_valid_is_zero_based() {
        assert_eq!(parse_selection("2", 3), Ok(1));
    }

    #[test]
    fn selection_trims_whitespace() {
        assert_eq!(parse_selection("  1\n", 3), Ok(0));
    }

    #[test]
    fn selection_zero_is_error() {
        assert!(parse_selection("0", 3).is_err());
    }

    #[test]
    fn selection_out_of_range_is_error() {
        assert!(parse_selection("4", 3).is_err());
    }

    #[test]
    fn selection_non_numeric_is_error() {
        assert!(parse_selection("abc", 3).is_err());
    }
}
