//! Marker-region text engine: replace body between
//! `# >>> resilum:managed <tag>` and `# <<< resilum:managed`.

pub(super) fn replace_region(
    text: &str,
    tag: &str,
    body: &[String],
) -> Result<Option<String>, String> {
    let open_prefix = format!("# >>> resilum:managed {tag}");
    let close_prefix = "# <<< resilum:managed";
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|ln| is_open(ln, &open_prefix)) else {
        return Ok(None);
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, ln)| ln.trim_start().starts_with(close_prefix))
        .map(|(i, _)| i)
        .ok_or_else(|| format!("unterminated resilum:managed region for {tag:?}"))?;
    let mut new: Vec<String> = lines[..=start].iter().map(|s| s.to_string()).collect();
    new.extend(body.iter().cloned());
    new.extend(lines[end..].iter().map(|s| s.to_string()));
    let tail = if text.ends_with('\n') { "\n" } else { "" };
    Ok(Some(new.join("\n") + tail))
}

// Matches the open marker but not a longer sibling like `<tag>-extra`: whatever
// follows the tag must be whitespace or end-of-line.
fn is_open(line: &str, open_prefix: &str) -> bool {
    let t = line.trim_start();
    t.starts_with(open_prefix)
        && t[open_prefix.len()..]
            .chars()
            .next()
            .is_none_or(|c| c.is_ascii_whitespace())
}
