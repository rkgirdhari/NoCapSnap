//! robots.txt as RFC 9309 defines it: the group for our product token (or
//! `*`), longest matching rule wins, and a tie goes to Allow.

#[derive(Debug, Clone, PartialEq, Eq)]
enum Rule {
    Allow(String),
    Disallow(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Robots {
    rules: Vec<Rule>,
    /// Set when robots.txt could not be read for a server-side reason
    /// (RFC 9309 §2.3.1.4: treat as complete disallow).
    disallow_all: bool,
}

impl Robots {
    /// No robots.txt (a 4xx answer): everything may be read.
    pub fn allow_all() -> Self {
        Self {
            rules: Vec::new(),
            disallow_all: false,
        }
    }

    /// robots.txt unreachable or a 5xx answer: nothing may be read.
    pub fn disallow_all() -> Self {
        Self {
            rules: Vec::new(),
            disallow_all: true,
        }
    }

    /// True when nothing may be read because robots.txt itself couldn't be.
    pub fn is_unavailable(&self) -> bool {
        self.disallow_all
    }

    pub fn parse(text: &str, product_token: &str) -> Self {
        let token = product_token.to_ascii_lowercase();
        // (agents, rules) per group; consecutive user-agent lines share a group.
        let mut groups: Vec<(Vec<String>, Vec<Rule>)> = Vec::new();
        let mut last_was_agent = false;
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
            match key.as_str() {
                "user-agent" => {
                    if !last_was_agent || groups.is_empty() {
                        groups.push((Vec::new(), Vec::new()));
                    }
                    groups
                        .last_mut()
                        .unwrap()
                        .0
                        .push(value.to_ascii_lowercase());
                    last_was_agent = true;
                }
                "allow" | "disallow" => {
                    last_was_agent = false;
                    let Some(group) = groups.last_mut() else {
                        continue;
                    };
                    if value.is_empty() {
                        continue; // "Disallow:" with no path allows everything
                    }
                    group.1.push(if key == "allow" {
                        Rule::Allow(value.to_owned())
                    } else {
                        Rule::Disallow(value.to_owned())
                    });
                }
                _ => last_was_agent = false,
            }
        }
        // Our own group(s) if any name us, otherwise the `*` group(s).
        let wanted = if groups.iter().any(|(agents, _)| agents.contains(&token)) {
            token
        } else {
            "*".to_owned()
        };
        let rules = groups
            .iter()
            .filter(|(agents, _)| agents.contains(&wanted))
            .flat_map(|(_, rules)| rules.clone())
            .collect();
        Self {
            rules,
            disallow_all: false,
        }
    }

    /// `path` is the URL path plus `?query`, as sent in the request line.
    pub fn allows(&self, path: &str) -> bool {
        if self.disallow_all {
            return false;
        }
        if path == "/robots.txt" {
            return true;
        }
        let mut best: Option<(usize, bool)> = None;
        for rule in &self.rules {
            let (pattern, allow) = match rule {
                Rule::Allow(p) => (p, true),
                Rule::Disallow(p) => (p, false),
            };
            if matches(pattern.as_bytes(), path.as_bytes()) {
                let len = pattern.len();
                best = match best {
                    Some((l, a)) if l > len || (l == len && a) => Some((l, a)),
                    _ => Some((len, allow)),
                };
            }
        }
        best.is_none_or(|(_, allow)| allow)
    }
}

/// Prefix match with `*` (any run of characters) and a trailing `$` (end of
/// path). Greedy with single backtrack point: O(pattern × path) at worst, so
/// a hostile robots.txt full of `*` can't make matching explode.
fn matches(pattern: &[u8], path: &[u8]) -> bool {
    let (pat, anchored) = match pattern.split_last() {
        Some((b'$', head)) => (head, true),
        _ => (pattern, false),
    };
    let (mut p, mut s) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while s < path.len() {
        if p == pat.len() && !anchored {
            return true; // prefix matched
        }
        if p < pat.len() && pat[p] == b'*' {
            star = Some((p, s));
            p += 1;
        } else if p < pat.len() && pat[p] == path[s] {
            p += 1;
            s += 1;
        } else if let Some((sp, ss)) = star {
            p = sp + 1;
            s = ss + 1;
            star = Some((sp, ss + 1));
        } else {
            return false;
        }
    }
    pat[p..].iter().all(|&c| c == b'*')
}

#[cfg(test)]
mod tests {
    use super::Robots;

    const TOKEN: &str = "NoCapSnapBot";

    #[test]
    fn our_group_beats_the_wildcard_group() {
        let r = Robots::parse(
            "User-agent: *\nDisallow: /\n\nUser-agent: nocapsnapbot\nDisallow: /private\n",
            TOKEN,
        );
        assert!(r.allows("/menu"));
        assert!(!r.allows("/private/x"));
    }

    #[test]
    fn wildcard_group_applies_when_we_are_not_named() {
        let r = Robots::parse(
            "User-agent: Googlebot\nDisallow:\n\nUser-agent: *\nDisallow: /menu\n",
            TOKEN,
        );
        assert!(!r.allows("/menu"));
        assert!(!r.allows("/menu/dinner"));
        assert!(r.allows("/about"));
    }

    #[test]
    fn longest_match_wins_and_ties_go_to_allow() {
        let r = Robots::parse(
            "User-agent: *\nDisallow: /menu\nAllow: /menu/dinner\nAllow: /x\nDisallow: /x\n",
            TOKEN,
        );
        assert!(r.allows("/menu/dinner"));
        assert!(!r.allows("/menu/lunch"));
        assert!(r.allows("/x"));
    }

    #[test]
    fn wildcards_and_end_anchor() {
        let r = Robots::parse(
            "User-agent: *\nDisallow: /*.pdf$\nDisallow: /*?print\n",
            TOKEN,
        );
        assert!(!r.allows("/menus/dinner.pdf"));
        assert!(r.allows("/menus/dinner.pdf.html"));
        assert!(!r.allows("/menu?print=1"));
        assert!(r.allows("/menu"));
    }

    #[test]
    fn pathological_patterns_stay_fast() {
        let pattern = format!("/{}b", "*a".repeat(40));
        let r = Robots::parse(&format!("User-agent: *\nDisallow: {pattern}\n"), TOKEN);
        let path = format!("/{}", "a".repeat(4000));
        let start = std::time::Instant::now();
        assert!(r.allows(&path));
        assert!(
            start.elapsed() < std::time::Duration::from_millis(500),
            "{:?}",
            start.elapsed()
        );
    }

    #[test]
    fn empty_disallow_comments_and_robots_itself() {
        let r = Robots::parse("# hi\nUser-agent: * # everyone\nDisallow:\n", TOKEN);
        assert!(r.allows("/anything"));
        assert!(!Robots::disallow_all().allows("/"));
        assert!(Robots::parse("User-agent: *\nDisallow: /\n", TOKEN).allows("/robots.txt"));
    }
}
