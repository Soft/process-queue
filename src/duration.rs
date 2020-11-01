use anyhow::{bail, Result};
use std::time::Duration;

fn split_with<'a, F>(mut pred: F, input: &'a str) -> impl Iterator<Item = &'a str> + 'a
where
    F: FnMut(char) -> bool,
    F: 'a,
{
    let mut start = 0;
    let mut chars = input.char_indices();

    std::iter::from_fn(move || loop {
        if let Some((i, c)) = chars.next() {
            if pred(c) {
                let section = &input[start..i];
                start = i;
                if !section.is_empty() {
                    return Some(section);
                }
            }
            continue;
        } else {
            let section = &input[start..];
            if !section.is_empty() {
                start = input.len();
                return Some(section);
            } else {
                return None;
            }
        }
    })
}

fn separate_numbers(input: &str) -> impl Iterator<Item = &str> {
    let mut in_number = false;
    split_with(
        move |c| match c.is_ascii_digit() {
            true if !in_number => {
                in_number = true;
                true
            }
            false if in_number => {
                in_number = false;
                true
            }
            _ => false,
        },
        input,
    )
}

pub fn parse_duration(input: &str) -> Result<Duration> {
    let mut sum = Duration::new(0, 0);
    enum State {
        Duration,
        UnitNeeded(u64),
    };
    let mut state = State::Duration;
    for piece in input.split_whitespace().flat_map(separate_numbers) {
        match state {
            State::Duration => state = State::UnitNeeded(piece.parse()?),
            State::UnitNeeded(n) => {
                sum += match piece {
                    "s" | "sec" | "secs" | "second" | "seconds" => Duration::from_secs(n),
                    "m" | "min" | "mins" | "minute" | "minutes" => Duration::from_secs(n * 60),
                    "h" | "hour" | "hours" => Duration::from_secs(n * 60 * 60),
                    _ => bail!("unit expected"),
                };
                state = State::Duration;
            }
        }
    }
    match state {
        State::UnitNeeded(_) => bail!("unit expected"),
        _ => Ok(sum),
    }
}

#[test]
fn test_separate_numbers() {
    assert_eq!(
        separate_numbers("10aa20bb30cc").collect::<Vec<_>>(),
        vec!["10", "aa", "20", "bb", "30", "cc"]
    );
    assert_eq!(
        separate_numbers("a1b2c3").collect::<Vec<_>>(),
        vec!["a", "1", "b", "2", "c", "3"]
    );
    assert_eq!(separate_numbers("10").collect::<Vec<_>>(), vec!["10"]);
    assert_eq!(separate_numbers("aa").collect::<Vec<_>>(), vec!["aa"]);
    assert_eq!(
        separate_numbers("").collect::<Vec<_>>(),
        vec![] as Vec<&str>
    );
}

#[test]
fn test_duration() {
    assert_eq!(parse_duration("10s").unwrap(), Duration::from_secs(10));
    assert_eq!(parse_duration("10 s").unwrap(), Duration::from_secs(10));
    assert_eq!(
        parse_duration("10 s 1 hour").unwrap(),
        Duration::from_secs(10 + 60 * 60)
    );
    assert_eq!(
        parse_duration("30min").unwrap(),
        Duration::from_secs(60 * 30)
    );
    assert!(parse_duration("10").is_err(),);
}
