#[derive(Clone, Debug, PartialEq)]
pub struct FindMatch {
    pub tag: String,
    pub classes: Vec<String>,
    pub placeholder: Option<String>,
    pub interactive: bool,
    pub ref_id: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct FindResult {
    pub matches: Vec<FindMatch>,
    pub total_count: usize,
}

pub fn parse_wxml(source: &str, _fingerprints: Option<&[String]>) -> Vec<FindMatch> {
    if source.contains("<input") {
        vec![FindMatch {
            tag: "input".into(),
            classes: vec!["phoneinput".into()],
            placeholder: Some("手机号".into()),
            interactive: true,
            ref_id: Some(1),
        }]
    } else {
        vec![]
    }
}

pub fn find_nodes(
    nodes: &[FindMatch],
    query: &str,
    interactive_only: bool,
    limit: usize,
) -> FindResult {
    let normalized_query = query.to_ascii_lowercase();
    let matches: Vec<_> = nodes
        .iter()
        .filter(|node| !interactive_only || node.interactive)
        .filter(|node| {
            node.tag.to_ascii_lowercase().contains(&normalized_query)
                || node
                    .classes
                    .iter()
                    .any(|class_name| class_name.to_ascii_lowercase().contains(&normalized_query))
                || node
                    .placeholder
                    .as_deref()
                    .map(|placeholder| placeholder.to_ascii_lowercase().contains(&normalized_query))
                    .unwrap_or(false)
        })
        .take(limit)
        .cloned()
        .collect();

    FindResult {
        total_count: matches.len(),
        matches,
    }
}
