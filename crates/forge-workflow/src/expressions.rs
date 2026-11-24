//! Expression evaluation for `${{ }}` workflow syntax.
//!
//! Supported contexts:
//! - `${{ secrets.NAME }}`
//! - `${{ env.NAME }}`
//! - `${{ variables.NAME }}`
//!
//! Unknown references resolve to an empty string (matching GitHub Actions).

use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct ExpressionContext {
    pub env: BTreeMap<String, String>,
    pub secrets: BTreeMap<String, String>,
    pub variables: BTreeMap<String, String>,
}

impl ExpressionContext {
    pub fn new(
        env: BTreeMap<String, String>,
        secrets: BTreeMap<String, String>,
        variables: BTreeMap<String, String>,
    ) -> Self {
        Self {
            env,
            secrets,
            variables,
        }
    }

    fn lookup(&self, path: &str) -> Option<&str> {
        let (namespace, key) = path.split_once('.')?;
        let key = key.trim();
        let value = match namespace.trim() {
            "secrets" => self.secrets.get(key)?,
            "env" => self.env.get(key)?,
            "variables" | "vars" => self.variables.get(key)?,
            _ => return None,
        };
        Some(value.as_str())
    }
}

/// Evaluate all `${{ ... }}` expressions in `template`.
pub fn evaluate_template(template: &str, context: &ExpressionContext) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' && i + 2 < bytes.len() && bytes[i + 1] == b'{' && bytes[i + 2] == b'{' {
            if let Some((value, next)) = try_eval_expression(template, i, context) {
                out.push_str(&value);
                i = next;
                continue;
            }
        }
        // Safe: we only advance on ASCII `$`/`{` boundaries or copy whole chars.
        let ch_len = utf8_char_len(bytes[i]);
        out.push_str(&template[i..i + ch_len]);
        i += ch_len;
    }

    out
}

/// If a complete `${{ ... }}` starts at `start`, evaluate it and return
/// (value, index_after_closing_braces). Otherwise return None.
fn try_eval_expression(
    template: &str,
    start: usize,
    context: &ExpressionContext,
) -> Option<(String, usize)> {
    // `${{` is three bytes; stepping over only `${` leaves the second `{` in
    // the expression body and prevents every namespace lookup from matching.
    let after_open = start + 3;
    let rest = template.get(after_open..)?;
    // Find the closing `}}` — expression body must not contain `}}`.
    let close_rel = rest.find("}}")?;
    let body = rest[..close_rel].trim();
    let end = after_open + close_rel + 2;

    let value = if body.is_empty() {
        String::new()
    } else {
        context.lookup(body).unwrap_or("").to_string()
    };
    Some((value, end))
}

fn utf8_char_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

/// True if the string contains at least one `${{` opener.
pub fn contains_expression(template: &str) -> bool {
    template.contains("${{")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ExpressionContext {
        let mut env = BTreeMap::new();
        env.insert("HOME".into(), "/tmp/forge".into());
        env.insert("GREETING".into(), "hello".into());
        let mut secrets = BTreeMap::new();
        secrets.insert("TOKEN".into(), "s3cret".into());
        let mut variables = BTreeMap::new();
        variables.insert("BRANCH".into(), "main".into());
        ExpressionContext::new(env, secrets, variables)
    }

    #[test]
    fn evaluates_secret_env_and_variables() {
        let c = ctx();
        assert_eq!(
            evaluate_template("tok=${{ secrets.TOKEN }}", &c),
            "tok=s3cret"
        );
        assert_eq!(
            evaluate_template("${{ env.GREETING }} world", &c),
            "hello world"
        );
        assert_eq!(evaluate_template("b=${{ variables.BRANCH }}", &c), "b=main");
        // `vars` alias
        assert_eq!(evaluate_template("${{ vars.BRANCH }}", &c), "main");
    }

    #[test]
    fn unknown_references_resolve_empty() {
        let c = ctx();
        assert_eq!(evaluate_template("[${{ secrets.MISSING }}]", &c), "[]");
        assert_eq!(evaluate_template("${{ nope.KEY }}", &c), "");
        assert_eq!(evaluate_template("${{ env.MISSING }}", &c), "");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        let c = ctx();
        assert_eq!(evaluate_template("echo hi", &c), "echo hi");
        assert_eq!(evaluate_template("", &c), "");
        // Non-UTF8-safe path: multi-byte chars pass through
        assert_eq!(evaluate_template("héllo →", &c), "héllo →");
    }

    #[test]
    fn multiple_expressions_in_one_template() {
        let c = ctx();
        assert_eq!(
            evaluate_template("${{ env.GREETING }}-${{ secrets.TOKEN }}", &c),
            "hello-s3cret"
        );
    }

    #[test]
    fn malformed_expression_left_as_is() {
        let c = ctx();
        assert_eq!(evaluate_template("${{ env.HOME", &c), "${{ env.HOME");
        assert_eq!(evaluate_template("${{}}", &c), "");
    }
}
