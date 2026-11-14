use crate::request::{CookiePlacement, Pseudo, Request};
use crate::{Error, Result};

pub(crate) struct Target<'a> {
    pub http2: bool,
    pub authority: &'a str,
    pub scheme: &'a str,
    pub pseudo_order: Option<[Pseudo; 4]>,
}

// request headers + generated pseudo-headers or `Host` + jar cookies, pseudo-headers
// in the target's order
pub(crate) fn build(
    request: &Request,
    jar: &[(String, String)],
    target: &Target,
) -> Result<Vec<(String, String)>> {
    let mut headers = request.headers.clone();

    if target.http2 {
        if !headers.iter().any(|(name, _)| name.starts_with(':')) {
            let order = target.pseudo_order.unwrap_or(Pseudo::CHROME);
            let generated = order.iter().map(|&pseudo| {
                let value = match pseudo {
                    Pseudo::Method => request.method.as_str(),
                    Pseudo::Authority => target.authority,
                    Pseudo::Scheme => target.scheme,
                    Pseudo::Path => request.path.as_str(),
                };
                (pseudo.name().to_string(), value.to_string())
            });
            headers.splice(0..0, generated);
        }
    } else if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("host"))
    {
        headers.insert(0, ("Host".to_string(), target.authority.to_string()));
    }

    if request.cookies != CookiePlacement::Off {
        let cookies = cookie_headers(jar, &request.cookie_order, target.http2);
        let at = match &request.cookies {
            CookiePlacement::Off => unreachable!(),
            CookiePlacement::First => headers
                .iter()
                .rposition(|(name, _)| name.starts_with(':') || name.eq_ignore_ascii_case("host"))
                .map_or(0, |i| i + 1),
            CookiePlacement::Last => headers.len(),
            CookiePlacement::After(after) => headers
                .iter()
                .position(|(name, _)| name.eq_ignore_ascii_case(after))
                .map(|i| i + 1)
                .ok_or_else(|| {
                    Error::InvalidRequest(format!("cookies go after `{after}`, which is not set"))
                })?,
        };
        headers.splice(at..at, cookies);
    }

    if let (true, Some(order)) = (target.http2, target.pseudo_order) {
        let rank = |name: &str| {
            order
                .iter()
                .position(|p| p.name() == name)
                .unwrap_or(order.len())
        };
        let (mut pseudos, rest): (Vec<_>, Vec<_>) = headers
            .into_iter()
            .partition(|(name, _)| name.starts_with(':'));
        pseudos.sort_by_key(|(name, _)| rank(name));
        headers = pseudos.into_iter().chain(rest).collect();
    }

    Ok(headers)
}

// names in `order` first, then jar order; HTTP/2 sends one `cookie` header per cookie
// like browsers, HTTP/1.1 joins them into one `Cookie`
fn cookie_headers(
    jar: &[(String, String)],
    order: &[String],
    http2: bool,
) -> Vec<(String, String)> {
    let mut names: Vec<&str> = order.iter().map(String::as_str).collect();
    for (name, _) in jar {
        if !names.contains(&name.as_str()) {
            names.push(name);
        }
    }

    let pairs: Vec<String> = names
        .iter()
        .flat_map(|&wanted| {
            jar.iter()
                .filter(move |(name, _)| name == wanted)
                .map(|(name, value)| format!("{name}={value}"))
        })
        .collect();

    if http2 {
        pairs
            .into_iter()
            .map(|pair| ("cookie".to_string(), pair))
            .collect()
    } else if pairs.is_empty() {
        Vec::new()
    } else {
        vec![("Cookie".to_string(), pairs.join("; "))]
    }
}

// one `name: value` per line; HTTP/1.1 lines go on the wire as is, hence CRLF
pub(crate) fn join(headers: &[(String, String)], http2: bool) -> String {
    headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join(if http2 { "\n" } else { "\r\n" })
}
