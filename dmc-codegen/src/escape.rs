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

/// True for ASCII control chars (`0x00..=0x1f`, `0x7f`). Browsers strip
/// TAB/LF/CR (and treat other control chars erratically) from URLs
/// *before* scheme matching, so `java\tscript:` resolves to
/// `javascript:`. We mirror that by stripping all C0 + DEL control
/// chars before scheme detection. Note: only ASCII C0/DEL is stripped
/// (the SEC-009 set); non-ASCII (incl. C1) passes through, since `char`
/// iteration keeps multibyte UTF-8 intact.
fn is_url_control_char(c: char) -> bool {
  (c as u32) <= 0x1f || c == '\u{7f}'
}

/// Returns `true` when `url` contains any ASCII control char.
fn has_control_char(url: &str) -> bool {
  url.chars().any(is_url_control_char)
}

/// Strip every ASCII control char from `url` — the browser-effective
/// form used for scheme matching. SEC-011: iterates over `char`s, not
/// raw bytes, so multibyte UTF-8 sequences survive intact.
fn strip_control_chars(url: &str) -> String {
  url.chars().filter(|c| !is_url_control_char(*c)).collect()
}

/// Reject URLs carrying a dangerous scheme (`javascript:`, `data:`,
/// `vbscript:`, `file:`, ...). Allows relative URLs (no scheme) and the
/// absolute-scheme allowlist `{http, https, mailto, tel}`.
///
/// A URL has a scheme if it matches `^[a-zA-Z][a-zA-Z0-9+.-]*:` *before*
/// any `/`, `?`, or `#` — so `./foo:bar`, `/foo:bar`, `#frag` and
/// `?q=a:b` are correctly treated as schemeless (safe).
///
/// The input is checked in its control-char-stripped form (see
/// [`strip_control_chars`]) — a pre-`:` token holding a control byte
/// (TAB/LF/CR/NUL/...) would otherwise look schemeless here while a
/// browser collapses it back into a live `javascript:` scheme.
pub fn is_safe_url(url: &str) -> bool {
  let stripped = strip_control_chars(url);
  let trimmed = stripped.trim_start_matches([' ', '\t', '\n', '\r']);
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
          // A non-canonical scheme token in the *stripped* form is not a
          // recognized scheme — treat as relative. (Control chars are
          // already gone, so this can no longer be exploited.)
          return true;
        }
        return matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "mailto" | "tel");
      },
      _ => i += 1,
    }
  }
  true // no `:` at all -> relative -> safe
}

/// Sanitize a link/image URL. Returns the URL unchanged when it is both
/// safe ([`is_safe_url`]) *and* free of control chars; otherwise returns
/// the safe fallback `"#"`.
///
/// A control-char-bearing URL is never passed through verbatim — even if
/// its stripped form is safe — because the raw bytes can be re-interpreted
/// downstream. When in doubt, fall back to `#`.
pub fn sanitize_url(url: &str) -> String {
  if has_control_char(url) {
    // Re-check the browser-effective (stripped) form; never emit the raw
    // control-char-bearing URL.
    if is_safe_url(url) { strip_control_chars(url) } else { "#".to_string() }
  } else if is_safe_url(url) {
    url.to_string()
  } else {
    "#".to_string()
  }
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
      "JAVASCRIPT:alert(1)",
      "  javascript:alert(1)",
      "data:text/html,<script>x</script>",
      "vbscript:msgbox(1)",
      "file:///etc/passwd",
    ] {
      assert!(!is_safe_url(u), "should reject {u:?}");
      assert_eq!(sanitize_url(u), "#");
    }
  }

  /// SEC-009: control chars inside the scheme token must not bypass the
  /// allowlist. Browsers strip TAB/LF/CR/NUL before scheme matching, so
  /// `java\tscript:` collapses to a live `javascript:`. `sanitize_url`
  /// must fall back to `#` and never emit the raw control-char URL.
  #[test]
  fn rejects_control_char_scheme_bypass() {
    for u in [
      "java\tscript:alert(1)",
      "java\nscript:alert(1)",
      "java\rscript:alert(1)",
      "\u{0}javascript:alert(1)",
      "java\u{0}script:alert(1)",
      "jav\u{1}ascript:alert(1)",
      "javascript\t:alert(1)",
      "\tjavascript:alert(1)",
    ] {
      assert!(!is_safe_url(u), "should reject {u:?}");
      assert_eq!(sanitize_url(u), "#", "should fall back to # for {u:?}");
    }
  }

  /// A control-char-bearing URL whose stripped form is *safe* must still
  /// not be emitted verbatim — emit the stripped form instead.
  #[test]
  fn strips_control_chars_from_otherwise_safe_url() {
    assert_eq!(sanitize_url("https://exa\tmple.com"), "https://example.com");
    assert_eq!(sanitize_url("/re\nl/path"), "/rel/path");
  }

  /// SEC-011: stripping control chars must not mangle multibyte UTF-8.
  /// The TAB is removed; the `ä` (and other non-ASCII) survives intact.
  #[test]
  fn strip_preserves_multibyte_utf8() {
    assert_eq!(sanitize_url("https://exämple.com/\tpath"), "https://exämple.com/path");
    // non-ASCII bytes are never reinterpreted as Latin-1 chars
    assert!(is_safe_url("https://exämple.com/\tpath"));
    assert_eq!(sanitize_url("/café\n/menu"), "/café/menu");
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
