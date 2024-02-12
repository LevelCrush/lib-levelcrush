/// Generates a unix timestamp
pub fn unix_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn slugify(input: &str) -> String {
    let slug: String = input
        .trim()
        .chars()
        .map(|c| match c {
            ' ' => '-',
            '%' => '-',
            '#' => '-',
            '(' => '-',
            ')' => '-',
            '[' => '-',
            ']' => '-',
            '\'' => '-',
            '"' => '-',
            _ => c,
        })
        .collect();

    slug.to_lowercase().trim().to_string()
}
