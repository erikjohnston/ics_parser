use std::collections::VecDeque;

use anyhow::{bail, Error};

/// Unescape string.
pub fn unescape(s: &str) -> Result<String, Error> {
    let mut queue: VecDeque<_> = String::from(s).chars().collect();
    let mut s = String::new();

    while let Some(c) = queue.pop_front() {
        if c != '\\' {
            s.push(c);
            continue;
        }

        match queue.pop_front() {
            Some('n') => s.push('\n'),
            Some('N') => s.push('\n'),
            Some('\\') => s.push('\\'),
            Some(';') => s.push(';'),
            Some(',') => s.push(','),
            Some(c) => bail!("Unexpected escape sequence \\{}", c),
            None => bail!("String ends up in \\"),
        };
    }

    Ok(s)
}
