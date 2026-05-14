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
