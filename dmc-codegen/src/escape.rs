/// HTML-escape text content per CommonMark reference output: `&`, `<`,
/// `>`, and `"` map to named entities. CM's spec runner encodes `"` in
/// text too so the diff stays clean.
pub fn escape_text(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for ch in s.chars() {
    match ch {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      _ => out.push(ch),
    }
  }
  out
}

/// Reject URLs carrying a dangerous scheme (`javascript:`, `data:`,
/// `vbscript:`, `file:`, ...). Allows relative URLs (no scheme) and the
/// absolute-scheme allowlist `{http, https, mailto, tel}`.
///
/// A URL has a scheme if it matches `^[a-zA-Z][a-zA-Z0-9+.-]*:` *before*
/// any `/`, `?`, or `#` — so `./foo:bar`, `/foo:bar`, `#frag` and
/// `?q=a:b` are correctly treated as schemeless (safe).
pub fn is_safe_url(url: &str) -> bool {
  let trimmed = url.trim_start_matches([' ', '\t', '\n', '\r']);
  let bytes = trimmed.as_bytes();
  // Find the scheme delimiter `:` before any path/query/fragment marker.
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'/' | b'?' | b'#' => return true, // schemeless -> relative -> safe
      b':' => {
        // `scheme` is bytes[..i]; must be a non-empty valid scheme.
        if i == 0 {
          return true; // leading `:` is not a scheme
        }
        let scheme = &trimmed[..i];
        let mut chars = scheme.bytes();
        let first_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
        let rest_ok = chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'.' | b'-'));
        if !(first_ok && rest_ok) {
          return true; // not a valid scheme token -> treat as relative
        }
        return matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "mailto" | "tel");
      },
      _ => i += 1,
    }
  }
  true // no `:` at all -> relative -> safe
}

/// Sanitize a link/image URL: returns the URL unchanged when it passes
/// [`is_safe_url`], otherwise the safe fallback `"#"`.
pub fn sanitize_url(url: &str) -> &str {
  if is_safe_url(url) { url } else { "#" }
}

/// Percent-encode the bytes that CM's reference renderer escapes inside
/// link destinations. Keeps already-encoded sequences (`%5C`) intact and
/// preserves the unreserved + sub-delims set per RFC 3986.
pub fn escape_url(s: &str) -> String {
  let bytes = s.as_bytes();
  let mut out = String::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    let b = bytes[i];
    // RFC 3986 unreserved + sub-delims + gen-delims CM treats as safe.
    let safe = b.is_ascii_alphanumeric()
      || matches!(
        b,
        b'-'
          | b'_'
          | b'.'
          | b'~'
          | b'!'
          | b'$'
          | b'\''
          | b'('
          | b')'
          | b'*'
          | b','
          | b';'
          | b'='
          | b'+'
          | b':'
          | b'@'
          | b'/'
          | b'?'
          | b'#'
      );
    if b == b'%' && i + 2 < bytes.len() && bytes[i + 1].is_ascii_hexdigit() && bytes[i + 2].is_ascii_hexdigit() {
      out.push('%');
      out.push(bytes[i + 1] as char);
      out.push(bytes[i + 2] as char);
      i += 3;
      continue;
    }
    if b == b'&' {
      out.push('&');
    } else if safe {
      out.push(b as char);
    } else {
      out.push_str(&format!("%{:02X}", b));
    }
    i += 1;
  }
  out
}

/// HTML-escape an attribute value. Same as [`escape_text`] plus `"` -> `&quot;`
/// since attribute values are quoted.
pub fn escape_attr(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for ch in s.chars() {
    match ch {
      '&' => out.push_str("&amp;"),
      '"' => out.push_str("&quot;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      _ => out.push(ch),
    }
  }
  out
}

#[cfg(test)]
mod url_safety_tests {
  use super::{is_safe_url, sanitize_url};

  #[test]
  fn rejects_dangerous_schemes() {
    for u in [
      "javascript:alert(1)",
      "JavaScript:alert(1)",
      "  javascript:alert(1)",
      "data:text/html,<script>x</script>",
      "vbscript:msgbox(1)",
      "file:///etc/passwd",
    ] {
      assert!(!is_safe_url(u), "should reject {u:?}");
      assert_eq!(sanitize_url(u), "#");
    }
  }

  #[test]
  fn allows_safe_schemes_and_relative() {
    for u in [
      "https://example.com/x",
      "http://example.com",
      "HTTPS://EXAMPLE.COM",
      "mailto:a@b.com",
      "tel:+15551234",
      "/abs/path",
      "./rel/path",
      "../up/path",
      "foo/bar",
      "#fragment",
      "?q=a:b",
      "./weird:name",
      "page#a:b",
    ] {
      assert!(is_safe_url(u), "should allow {u:?}");
      assert_eq!(sanitize_url(u), u);
    }
  }
}
