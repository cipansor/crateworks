//! The redaction itself.
//!
//! This is a port of the algorithm in npm `fast-redact`. The literal-path case
//! is simple; the wildcard case is not, and the shape of the wildcard walk is
//! preserved rather than "cleaned up", because the reference's own tests pin it
//! and the corpus in `tests/fixtures/` checks it case by case.
//!
//! # How a path is applied
//!
//! Every path is applied to a copy of the document, one after another, and the
//! copy is returned. The reference mutates the caller's object and offers
//! `restore` to undo it; a returned copy makes `restore` unnecessary, and the
//! output is the same.
//!
//! # Why the wildcard walk looks the way it does
//!
//! The reference splits a wildcard path at the **first** `*` into `before` and
//! `after`. `a.*.b` is therefore "reach `a`, then for each of its keys, walk
//! `b`" — not a general glob. Only a path with **two or more** stars puts a
//! star inside `after`, and that is the case the awkward code below exists for:
//! the reference's generated code does not descend past a star inside `after`,
//! so `*.*.*` redacts one level higher than `*.*.*` reads. That is the
//! reference's behaviour, it is covered by the reference's own test suite, and
//! it is reproduced here so that the corpus agrees.

use crate::json::Value;
use crate::path::Path;

/// The signature a censor function has: the original value and its path.
pub type CensorFn = dyn Fn(&Value, &[String]) -> Value;

/// What replaces a redacted value.
pub enum Censor {
    /// Replace the value with a fixed one, by default the string
    /// `"[REDACTED]"`.
    Value(Value),
    /// Remove the key. In an object the key disappears; in an array the element
    /// becomes `null`, which is what `JSON.stringify` does with the `undefined`
    /// the reference stores. This is the reference's `remove: true`.
    Remove,
    /// Replace the value with the result of a function, which is given the
    /// original value and the path to it. The reference offers two forms, one
    /// taking the value and one taking the value and the path; a single form
    /// covers both, because a caller who does not want the path ignores it.
    Function(Box<CensorFn>),
}

impl std::fmt::Debug for Censor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Censor::Value(value) => f.debug_tuple("Value").field(value).finish(),
            Censor::Remove => f.write_str("Remove"),
            Censor::Function(_) => f.write_str("Function(..)"),
        }
    }
}

impl Censor {
    /// The replacement for `value`, or `None` when the value is to be removed.
    fn apply(&self, value: &Value, path: &[String]) -> Option<Value> {
        match self {
            Censor::Value(replacement) => Some(replacement.clone()),
            Censor::Remove => None,
            Censor::Function(function) => Some(function(value, path)),
        }
    }
}

/// Redacts a JSON document according to a set of paths.
///
/// ```
/// use fast_redact::{Redactor, Value};
///
/// let redactor = Redactor::new(["headers.cookie"]).unwrap();
/// let document = Value::parse(r#"{"headers":{"host":"example.com","cookie":"secret"}}"#).unwrap();
/// assert_eq!(
///     redactor.redact(&document),
///     r#"{"headers":{"host":"example.com","cookie":"[REDACTED]"}}"#,
/// );
/// // The input is untouched.
/// assert_eq!(document.to_json(), r#"{"headers":{"host":"example.com","cookie":"secret"}}"#);
/// ```
#[derive(Debug)]
pub struct Redactor {
    paths: Vec<Path>,
    censor: Censor,
}

impl Redactor {
    /// Build a redactor from a list of paths.
    ///
    /// Duplicate paths are dropped. Every path is validated; the first invalid
    /// one is returned as an error, so a typo cannot silently redact nothing.
    ///
    /// ```
    /// use fast_redact::Redactor;
    /// assert!(Redactor::new(["a.b", "a.b"]).is_ok());
    /// assert!(Redactor::new(["a..b"]).is_err());
    /// ```
    pub fn new<I, S>(paths: I) -> Result<Self, crate::path::PathError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut parsed: Vec<Path> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for path in paths {
            let text = path.as_ref();
            if seen.iter().any(|s| s == text) {
                continue;
            }
            seen.push(text.to_string());
            parsed.push(crate::path::parse(text)?);
        }
        Ok(Self {
            paths: parsed,
            censor: Censor::Value(Value::String("[REDACTED]".to_string())),
        })
    }

    /// Use a fixed value as the replacement instead of `"[REDACTED]"`.
    #[must_use]
    pub fn with_censor(mut self, value: Value) -> Self {
        self.censor = Censor::Value(value);
        self
    }

    /// Remove redacted keys instead of replacing them.
    #[must_use]
    pub fn with_remove(mut self) -> Self {
        self.censor = Censor::Remove;
        self
    }

    /// Compute the replacement with a function.
    ///
    /// ```
    /// use fast_redact::{Redactor, Value};
    ///
    /// let redactor = Redactor::new(["token"])
    ///     .unwrap()
    ///     .with_censor_fn(|value, _path| {
    ///         let text = value.as_str().unwrap_or("");
    ///         Value::String(format!("<{} chars>", text.chars().count()))
    ///     });
    /// let document = Value::parse(r#"{"token":"abcdef"}"#).unwrap();
    /// assert_eq!(redactor.redact(&document), r#"{"token":"<6 chars>"}"#);
    /// ```
    #[must_use]
    pub fn with_censor_fn<F>(mut self, function: F) -> Self
    where
        F: Fn(&Value, &[String]) -> Value + 'static,
    {
        self.censor = Censor::Function(Box::new(function));
        self
    }

    /// The parsed paths, in the order they were given.
    #[must_use]
    pub fn paths(&self) -> &[Path] {
        &self.paths
    }

    /// Redact a document, returning the serialized result.
    ///
    /// This is the reference's default behaviour: it returns a JSON string.
    #[must_use]
    pub fn redact(&self, document: &Value) -> String {
        self.redact_value(document).to_json()
    }

    /// Redact a document, returning the redacted copy.
    ///
    /// The input is not modified.
    #[must_use]
    pub fn redact_value(&self, document: &Value) -> Value {
        let mut out = document.clone();
        // The reference applies every literal path first, then every wildcard
        // path, so that a literal and a wildcard that meet leave the same
        // result as the reference produces.
        for path in &self.paths {
            if let Path::Literal(segments) = path {
                self.redact_literal(&mut out, segments);
            }
        }
        for path in &self.paths {
            if let Path::Wildcard {
                before,
                after,
                nested,
            } = path
            {
                self.redact_wildcard(&mut out, before, after, *nested);
            }
        }
        out
    }

    /// Redact a path whose segments are all fixed: every hop must be present,
    /// and the leaf must not be `null`.
    fn redact_literal(&self, root: &mut Value, segments: &[String]) {
        let (leaf, parents) = segments.split_last().expect("a path has a leaf");
        // Walk to the parent. An intermediate that is missing or is not a
        // container ends the walk, which is what the reference's `o.a && o.b`
        // existence chain does.
        let mut parent = root;
        for segment in parents {
            match child_mut(parent, segment) {
                Some(next) if is_container(next) => parent = next,
                _ => return,
            }
        }
        let Some(current) = child(parent, leaf) else {
            return;
        };
        if current.is_null() {
            return;
        }
        let current = current.clone();
        match self.censor.apply(&current, segments) {
            Some(replacement) => set_child(parent, leaf, replacement),
            None => remove_child(parent, leaf),
        }
    }

    /// Redact a path that contains a `*`.
    fn redact_wildcard(&self, root: &mut Value, before: &[String], after: &[String], nested: bool) {
        let Some(target) = get_by_path(root, before) else {
            return;
        };
        if !is_container(target) {
            return;
        }
        if !nested {
            // The `a.*` shape: replace every key of the target.
            let keys = keys(target);
            for key in keys {
                let path = join(before, std::slice::from_ref(&key));
                if let Some(old) = get_by_path(root, &path).cloned() {
                    match self.censor.apply(&old, &path) {
                        Some(replacement) => set_by_path(root, &path, replacement),
                        None => remove_at(root, &path),
                    }
                }
            }
            return;
        }
        let keys = keys(target);
        for key in keys {
            let path = join(before, std::slice::from_ref(&key));
            let mut node_path = path.clone();
            self.special_set(root, &mut node_path, after, &join(&path, after));
        }
    }

    /// Walk `after` from the node at `node_path`, redacting as the reference's
    /// `specialSet` does.
    ///
    /// `full` is `before + key + after`, the path a censor function receives.
    ///
    /// The control flow mirrors the reference line for line, including the two
    /// pieces of state that look like they should reset and do not: `consecutive`
    /// stays set for the rest of the walk once a second star is seen, and a star
    /// that is not last advances without moving the node. Both are observable in
    /// the reference's own tests.
    fn special_set(
        &self,
        root: &mut Value,
        node_path: &mut Vec<String>,
        after: &[String],
        full: &[String],
    ) {
        if !get_by_path(root, node_path).is_some_and(is_container) {
            return;
        }
        let last = after.len() - 1;
        let mut i = 0;
        // `wc` is the reference's `wc === '*'`; `consecutive` marks a second
        // star directly after one, which is the case the level walk handles.
        let mut wc = false;
        let mut consecutive = false;
        while i < after.len() {
            let key = after[i].clone();
            let key_is_wildcard = key == "*";
            if !key_is_wildcard && !wc {
                let node = get_by_path(root, node_path).expect("node exists");
                if !is_container(node) || !has_key(node, &key) {
                    break;
                }
            }
            if key_is_wildcard {
                if wc {
                    consecutive = true;
                }
                wc = true;
                if i != last {
                    i += 1;
                    continue;
                }
            }
            if wc {
                let node = get_by_path(root, node_path).expect("node exists");
                for wck in keys(node) {
                    let child_path = join(node_path, std::slice::from_ref(&wck));
                    if consecutive {
                        self.iterate_nth_level(
                            root,
                            &child_path,
                            i as i64 - 1,
                            &key,
                            key_is_wildcard,
                            i == last,
                            full,
                        );
                    } else if key_is_wildcard {
                        if i == last {
                            self.replace_at(root, &child_path, full);
                        }
                    } else {
                        let child = get_by_path(root, &child_path);
                        let has = child.is_some_and(|c| is_container(c) && has_key(c, &key));
                        if has && i == last {
                            let deeper = join(&child_path, std::slice::from_ref(&key));
                            self.replace_at(root, &deeper, full);
                        }
                    }
                }
                wc = false;
            } else {
                let deeper = join(node_path, std::slice::from_ref(&key));
                if i == last {
                    self.replace_at(root, &deeper, full);
                }
                *node_path = deeper;
            }
            if !get_by_path(root, node_path).is_some_and(is_container) {
                break;
            }
            i += 1;
        }
    }

    /// The reference's `iterateNthLevel`: descend `level` container-children
    /// from the node at `node_path`, and at level zero redact.
    ///
    /// Two details matter and both are counter-intuitive. At level zero the
    /// redaction lands on the *descended* node, not the one the walk started
    /// at, which is why a run of stars redacts a level higher than the path
    /// reads. And the descent does not stop at level zero: the reference keeps
    /// descending with a negative level, redacting every matching descendant.
    /// A star redacts the descended node itself, so that node is no longer a
    /// container and the descent ends there; a named key leaves the node in
    /// place and the descent continues into its other children.
    #[allow(clippy::too_many_arguments)]
    fn iterate_nth_level(
        &self,
        root: &mut Value,
        node_path: &[String],
        level: i64,
        key: &str,
        key_is_wildcard: bool,
        is_last: bool,
        full: &[String],
    ) {
        if level == 0 && is_last {
            let node = get_by_path(root, node_path);
            let matches =
                key_is_wildcard || node.is_some_and(|n| is_container(n) && has_key(n, key));
            if matches {
                let target = if key_is_wildcard {
                    node_path.to_vec()
                } else {
                    join(node_path, std::slice::from_ref(&key.to_string()))
                };
                self.replace_at(root, &target, full);
            }
        }
        let children: Vec<String> = get_by_path(root, node_path).map_or_else(Vec::new, |node| {
            keys(node)
                .into_iter()
                .filter(|child| {
                    get_by_path(root, &join(node_path, std::slice::from_ref(child)))
                        .is_some_and(is_container)
                })
                .collect()
        });
        for child in children {
            let child_path = join(node_path, std::slice::from_ref(&child));
            self.iterate_nth_level(
                root,
                &child_path,
                level - 1,
                key,
                key_is_wildcard,
                is_last,
                full,
            );
        }
    }

    /// Apply the censor at `path`, or remove the value there.
    fn replace_at(&self, root: &mut Value, path: &[String], full: &[String]) {
        let Some(old) = get_by_path(root, path).cloned() else {
            return;
        };
        match self.censor.apply(&old, full) {
            Some(replacement) => set_by_path(root, path, replacement),
            None => remove_at(root, path),
        }
    }
}

/// `a` followed by `b`.
fn join(a: &[String], b: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    out.extend_from_slice(a);
    out.extend_from_slice(b);
    out
}

/// A value that can hold keys: an object or an array.
fn is_container(value: &Value) -> bool {
    matches!(value, Value::Object(_) | Value::Array(_))
}

/// The keys of a container, in order. An array's keys are its indices.
fn keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(pairs) => pairs.iter().map(|(key, _)| key.clone()).collect(),
        Value::Array(items) => (0..items.len()).map(|index| index.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// Whether a container has `key`.
fn has_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(pairs) => pairs.iter().any(|(name, _)| name == key),
        Value::Array(items) => key.parse::<usize>().is_ok_and(|index| index < items.len()),
        _ => false,
    }
}

/// The child of a container under `key`.
fn child<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(pairs) => pairs.iter().find(|(name, _)| name == key).map(|(_, v)| v),
        Value::Array(items) => key.parse::<usize>().ok().and_then(|index| items.get(index)),
        _ => None,
    }
}

/// The child of a container under `key`, mutably.
fn child_mut<'a>(value: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    match value {
        Value::Object(pairs) => pairs
            .iter_mut()
            .find(|(name, _)| name == key)
            .map(|(_, v)| v),
        Value::Array(items) => key
            .parse::<usize>()
            .ok()
            .and_then(|index| items.get_mut(index)),
        _ => None,
    }
}

/// The value at `path`, if every hop is present.
fn get_by_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = child(current, segment)?;
    }
    Some(current)
}

/// Write `replacement` at `path`, if the parent exists.
fn set_by_path(value: &mut Value, path: &[String], replacement: Value) {
    let Some((leaf, parents)) = path.split_last() else {
        return;
    };
    let mut parent = value;
    for segment in parents {
        match child_mut(parent, segment) {
            Some(next) => parent = next,
            None => return,
        }
    }
    set_child(parent, leaf, replacement);
}

/// Remove the value at `path`, or set an array element to `null`.
fn remove_at(value: &mut Value, path: &[String]) {
    let Some((leaf, parents)) = path.split_last() else {
        return;
    };
    let mut parent = value;
    for segment in parents {
        match child_mut(parent, segment) {
            Some(next) => parent = next,
            None => return,
        }
    }
    remove_child(parent, leaf);
}

/// Replace the child under `key`, if it exists.
fn set_child(parent: &mut Value, key: &str, replacement: Value) {
    match parent {
        Value::Object(pairs) => {
            if let Some(pair) = pairs.iter_mut().find(|(name, _)| name == key) {
                pair.1 = replacement;
            }
        }
        Value::Array(items) => {
            if let Some(index) = key.parse::<usize>().ok().filter(|i| *i < items.len()) {
                items[index] = replacement;
            }
        }
        _ => {}
    }
}

/// Remove the child under `key`. An array element becomes `null`, matching what
/// `JSON.stringify` does with the `undefined` the reference leaves behind.
fn remove_child(parent: &mut Value, key: &str) {
    match parent {
        Value::Object(pairs) => pairs.retain(|(name, _)| name != key),
        Value::Array(items) => {
            if let Some(index) = key.parse::<usize>().ok().filter(|i| *i < items.len()) {
                items[index] = Value::Null;
            }
        }
        _ => {}
    }
}
