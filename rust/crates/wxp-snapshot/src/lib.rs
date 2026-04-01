use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub struct ElementNode {
    pub tag: String,
    pub classes: Vec<String>,
    pub text: Option<String>,
    pub placeholder: Option<String>,
    pub attrs: BTreeMap<String, String>,
    pub interactive: bool,
    pub selector: String,
    pub ref_id: Option<u32>,
    pub children: Vec<ElementNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FindMatch {
    pub ref_id: Option<u32>,
    pub tag: String,
    pub classes: Vec<String>,
    pub text: Option<String>,
    pub placeholder: Option<String>,
    pub selector: String,
    pub interactive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FindResult {
    pub matches: Vec<FindMatch>,
    pub shown_count: usize,
    pub total_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderResult {
    pub output: String,
    pub shown_count: usize,
    pub total_count: usize,
}

const INTERACTIVE_TAGS: &[&str] = &[
    "button", "input", "textarea", "picker", "switch", "slider", "checkbox", "radio",
];

const INTERACTIVE_ATTRS: &[&str] = &[
    "bindtap",
    "catchtap",
    "bind:tap",
    "catch:tap",
    "bindchange",
    "bind:change",
    "bindinput",
    "bind:input",
    "bindsubmit",
    "bind:submit",
    "bindlongpress",
    "bind:longpress",
];

#[derive(Clone, Debug, Default)]
struct RawNode {
    tag: String,
    attrs: BTreeMap<String, String>,
    text_fragments: Vec<String>,
    children: Vec<RawNode>,
}

pub fn parse_wxml(
    source: &str,
    max_depth: usize,
    event_fingerprints: Option<&HashSet<String>>,
) -> Vec<ElementNode> {
    let raw_nodes = parse_raw_nodes(source);
    let mut roots = Vec::new();
    let mut idx = 0;

    for raw in raw_nodes {
        let is_page_root = raw.tag == "page";
        if let Some(node) = build_element_node(
            &raw,
            if is_page_root { "" } else { "page" },
            idx,
            0,
            max_depth,
            event_fingerprints,
        ) {
            roots.push(node);
            idx += 1;
        }
    }

    assign_refs(&mut roots);
    roots
}

pub fn assign_refs(nodes: &mut [ElementNode]) {
    fn walk(node: &mut ElementNode, next_ref: &mut u32) {
        if node.interactive {
            node.ref_id = Some(*next_ref);
            *next_ref += 1;
        }
        for child in &mut node.children {
            walk(child, next_ref);
        }
    }

    let mut next_ref = 1;
    for node in nodes {
        walk(node, &mut next_ref);
    }
}

pub fn render_tree(
    nodes: &[ElementNode],
    compact: bool,
    depth: usize,
    limit: Option<usize>,
    text_limit: usize,
) -> RenderResult {
    let total_count = count_interactive(nodes);
    let mut shown_count = 0;
    let mut lines = Vec::new();

    render_nodes(
        nodes,
        compact,
        depth,
        0,
        limit,
        text_limit,
        "",
        &mut shown_count,
        &mut lines,
    );

    RenderResult {
        output: lines.join("\n"),
        shown_count,
        total_count,
    }
}

pub fn find_nodes(
    nodes: &[ElementNode],
    query: &str,
    interactive_only: bool,
    limit: Option<usize>,
) -> FindResult {
    let normalized_query = query.to_ascii_lowercase();
    let mut matches = Vec::new();
    let mut total_count = 0;

    fn walk(
        node: &ElementNode,
        normalized_query: &str,
        interactive_only: bool,
        limit: Option<usize>,
        matches: &mut Vec<FindMatch>,
        total_count: &mut usize,
    ) {
        let eligible = !interactive_only || node.interactive;
        if eligible && matches_query(node, normalized_query) {
            *total_count += 1;
            if limit.is_none_or(|value| matches.len() < value) {
                matches.push(FindMatch {
                    ref_id: node.ref_id,
                    tag: node.tag.clone(),
                    classes: node.classes.clone(),
                    text: node.text.clone(),
                    placeholder: node.placeholder.clone(),
                    selector: node.selector.clone(),
                    interactive: node.interactive,
                });
            }
        }

        for child in &node.children {
            walk(
                child,
                normalized_query,
                interactive_only,
                limit,
                matches,
                total_count,
            );
        }
    }

    for node in nodes {
        walk(
            node,
            &normalized_query,
            interactive_only,
            limit,
            &mut matches,
            &mut total_count,
        );
    }

    FindResult {
        shown_count: matches.len(),
        matches,
        total_count,
    }
}

pub fn render_find_matches(matches: &[FindMatch], text_limit: usize) -> String {
    matches
        .iter()
        .map(|item| {
            let ref_part = item
                .ref_id
                .map(|value| format!("%{value}"))
                .unwrap_or_else(|| "-".to_string());
            let tag_part = if item.interactive {
                format!("[{}]", item.tag)
            } else {
                item.tag.clone()
            };
            let class_part = item
                .classes
                .first()
                .map(|value| format!(".{value}"))
                .unwrap_or_default();
            let text_part = item
                .text
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(|value| format!(" \"{}\"", truncate_text(value, text_limit)))
                .unwrap_or_default();
            let placeholder_part = if text_part.is_empty() {
                item.placeholder
                    .as_deref()
                    .map(|value| format!(" placeholder=\"{}\"", truncate_text(value, text_limit)))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            format!("{ref_part} {tag_part}{class_part}{text_part}{placeholder_part}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_raw_nodes(source: &str) -> Vec<RawNode> {
    let mut roots = Vec::new();
    let mut stack: Vec<RawNode> = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        let remainder = &source[cursor..];
        if let Some(open_pos) = remainder.find('<') {
            let text = &remainder[..open_pos];
            push_text(&mut stack, text);
            cursor += open_pos;
        } else {
            push_text(&mut stack, remainder);
            break;
        }

        let tag_remainder = &source[cursor..];
        let Some(close_pos) = tag_remainder.find('>') else {
            break;
        };
        let token = &tag_remainder[1..close_pos];
        cursor += close_pos + 1;

        let token = token.trim();
        if token.is_empty() || token.starts_with('!') || token.starts_with('?') {
            continue;
        }

        if token.starts_with('/') {
            if let Some(node) = stack.pop() {
                attach_node(&mut stack, &mut roots, node);
            }
            continue;
        }

        let self_closing = token.ends_with('/');
        let token = token.trim_end_matches('/').trim();
        let (tag, attrs) = parse_tag_token(token);
        let node = RawNode {
            tag,
            attrs,
            text_fragments: Vec::new(),
            children: Vec::new(),
        };

        if self_closing {
            attach_node(&mut stack, &mut roots, node);
        } else {
            stack.push(node);
        }
    }

    while let Some(node) = stack.pop() {
        attach_node(&mut stack, &mut roots, node);
    }

    roots
}

fn attach_node(stack: &mut [RawNode], roots: &mut Vec<RawNode>, node: RawNode) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
    }
}

fn push_text(stack: &mut [RawNode], text: &str) {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return;
    }

    if let Some(node) = stack.last_mut() {
        node.text_fragments.push(normalized);
    }
}

fn parse_tag_token(token: &str) -> (String, BTreeMap<String, String>) {
    let mut chars = token.chars().peekable();
    let mut tag = String::new();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            break;
        }
        tag.push(ch);
        chars.next();
    }

    let mut attrs = BTreeMap::new();
    loop {
        skip_whitespace(&mut chars);
        if chars.peek().is_none() {
            break;
        }

        let mut key = String::new();
        while let Some(ch) = chars.peek().copied() {
            if ch.is_whitespace() || ch == '=' {
                break;
            }
            key.push(ch);
            chars.next();
        }

        skip_whitespace(&mut chars);
        let mut value = String::new();
        if chars.peek().is_some_and(|ch| *ch == '=') {
            chars.next();
            skip_whitespace(&mut chars);
            if let Some(quote) = chars.peek().copied() {
                if quote == '"' || quote == '\'' {
                    chars.next();
                    while let Some(ch) = chars.next() {
                        if ch == quote {
                            break;
                        }
                        value.push(ch);
                    }
                } else {
                    while let Some(ch) = chars.peek().copied() {
                        if ch.is_whitespace() {
                            break;
                        }
                        value.push(ch);
                        chars.next();
                    }
                }
            }
        }

        if !key.is_empty() {
            attrs.insert(key, value);
        }
    }

    (tag.to_ascii_lowercase(), attrs)
}

fn skip_whitespace<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
        chars.next();
    }
}

fn build_element_node(
    raw: &RawNode,
    parent_selector: &str,
    sibling_index: usize,
    depth: usize,
    max_depth: usize,
    event_fingerprints: Option<&HashSet<String>>,
) -> Option<ElementNode> {
    if matches!(raw.tag.as_str(), "wxs" | "script" | "style") {
        return None;
    }

    let classes = raw
        .attrs
        .get("class")
        .map(|value| {
            value
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selector = build_selector(&raw.tag, &raw.attrs, parent_selector, sibling_index);
    let text = get_text_content(raw);
    let placeholder = raw.attrs.get("placeholder").cloned();
    let attrs = raw
        .attrs
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "class" | "style" | "id"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut children = Vec::new();
    if depth < max_depth {
        let mut child_index = 0;
        for child in &raw.children {
            if let Some(node) = build_element_node(
                child,
                &selector,
                child_index,
                depth + 1,
                max_depth,
                event_fingerprints,
            ) {
                children.push(node);
                child_index += 1;
            }
        }
    }

    Some(ElementNode {
        tag: raw.tag.clone(),
        classes: classes.clone(),
        text,
        placeholder,
        attrs,
        interactive: is_interactive(&raw.tag, &raw.attrs, &classes, event_fingerprints),
        selector,
        ref_id: None,
        children,
    })
}

fn build_selector(
    tag: &str,
    attrs: &BTreeMap<String, String>,
    parent_selector: &str,
    _sibling_index: usize,
) -> String {
    if let Some(id) = attrs.get("id").filter(|value| !value.is_empty()) {
        return format!("#{id}");
    }

    let base = attrs
        .get("class")
        .and_then(|value| value.split_whitespace().next())
        .map(|value| format!("{tag}.{value}"))
        .unwrap_or_else(|| tag.to_string());

    if parent_selector.is_empty() {
        base
    } else {
        format!("{parent_selector} > {base}")
    }
}

fn get_text_content(raw: &RawNode) -> Option<String> {
    let mut parts = raw.text_fragments.clone();
    for child in &raw.children {
        if let Some(text) = get_text_content(child) {
            if !text.is_empty() {
                parts.push(text);
            }
        }
    }

    let joined = parts.join(" ").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn is_interactive(
    tag: &str,
    attrs: &BTreeMap<String, String>,
    classes: &[String],
    event_fingerprints: Option<&HashSet<String>>,
) -> bool {
    if INTERACTIVE_TAGS.contains(&tag) {
        return true;
    }

    for attr in INTERACTIVE_ATTRS {
        if attrs.contains_key(*attr) {
            return true;
        }
    }

    if attrs.contains_key("hover-start-time") || attrs.contains_key("hover-stay-time") {
        return true;
    }

    for key in attrs.keys() {
        if !key.starts_with("data-") {
            continue;
        }
        if key == "data-sid" || key.starts_with("data-v-") || key.starts_with("data-__") {
            continue;
        }
        return true;
    }

    if let Some(fingerprints) = event_fingerprints {
        if !classes.is_empty() {
            let mut normalized = classes.to_vec();
            normalized.sort();
            let fingerprint = format!("{tag}:{}", normalized.join(","));
            return fingerprints.contains(&fingerprint);
        }
    }

    false
}

fn count_interactive(nodes: &[ElementNode]) -> usize {
    nodes
        .iter()
        .map(|node| usize::from(node.interactive) + count_interactive(&node.children))
        .sum()
}

fn render_nodes(
    nodes: &[ElementNode],
    compact: bool,
    depth: usize,
    current_depth: usize,
    limit: Option<usize>,
    text_limit: usize,
    prefix: &str,
    shown_count: &mut usize,
    lines: &mut Vec<String>,
) {
    for (index, node) in nodes.iter().enumerate() {
        if compact && !has_interactive_descendant(node) {
            continue;
        }
        if current_depth > depth {
            continue;
        }
        if node.interactive && limit.is_some_and(|value| *shown_count >= value) {
            continue;
        }

        let last = index == nodes.len() - 1;
        let connector = if last { "└─" } else { "├─" };
        let child_prefix = format!("{prefix}{}", if last { "   " } else { "│  " });

        if node.interactive {
            *shown_count += 1;
        }

        let tag_part = if node.interactive {
            format!("[{}]", node.tag)
        } else {
            node.tag.clone()
        };
        let class_part = node
            .classes
            .first()
            .map(|value| format!(".{value}"))
            .unwrap_or_default();
        let text_part = node
            .text
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| format!(" \"{}\"", truncate_text(value, text_limit)))
            .unwrap_or_default();
        let placeholder_part = node
            .placeholder
            .as_deref()
            .map(|value| format!("  placeholder=\"{}\"", truncate_text(value, text_limit)))
            .unwrap_or_default();
        let ref_part = node
            .ref_id
            .map(|value| format!("  → %{value}"))
            .unwrap_or_default();

        lines.push(format!(
            "{prefix}{connector} {tag_part}{class_part}{text_part}{placeholder_part}{ref_part}"
        ));

        if !node.children.is_empty() && current_depth < depth {
            render_nodes(
                &node.children,
                compact,
                depth,
                current_depth + 1,
                limit,
                text_limit,
                &child_prefix,
                shown_count,
                lines,
            );
        }
    }
}

fn has_interactive_descendant(node: &ElementNode) -> bool {
    node.interactive || node.children.iter().any(has_interactive_descendant)
}

fn matches_query(node: &ElementNode, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return false;
    }

    node.tag.to_ascii_lowercase().contains(normalized_query)
        || node
            .classes
            .iter()
            .any(|class_name| class_name.to_ascii_lowercase().contains(normalized_query))
        || node
            .text
            .as_deref()
            .map(|text| text.to_ascii_lowercase().contains(normalized_query))
            .unwrap_or(false)
        || node
            .placeholder
            .as_deref()
            .map(|placeholder| placeholder.to_ascii_lowercase().contains(normalized_query))
            .unwrap_or(false)
}

fn truncate_text(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
