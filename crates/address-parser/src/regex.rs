//! A small backtracking regular-expression engine.
//!
//! The reference parser is a single backtracking regular expression with named
//! capture groups, lookahead and lazy quantifiers. Rust's `regex` crate cannot
//! express it - it has no backtracking and no lookahead - and this workspace's
//! crates depend on the standard library alone. Rather than translate the
//! grammar into hand-written parsing code, where a difference from the reference
//! would be invisible, the crate compiles the reference's *own* pattern text and
//! runs it here. The grammar stays data; only this engine can be wrong.
//!
//! The syntax supported is exactly the subset the reference's patterns use:
//! literals, `.`, character classes (with ranges and the `\d \w \s` shorthands
//! and their negations), the anchors `^` and `$`, the word boundaries `\b` and
//! `\B`, alternation, the quantifiers `* + ? {n} {n,} {n,m}` in greedy and lazy
//! form, non-capturing groups, named and unnamed capture groups, and the
//! lookaheads `(?=...)` and `(?!...)`.
//!
//! Semantics follow JavaScript, which is where the reference runs: `\w` and `\d`
//! are ASCII, `\s` is JavaScript's set, `$` matches only at the very end of the
//! input, and `.` excludes the line terminators. `is_word` and `is_space` below
//! are the definitions that make those true.

/// A character class, as it appeared between `[` and `]`.
#[derive(Debug, Clone)]
pub(crate) struct ClassDef {
    /// Whether the class was written `[^...]`.
    pub(crate) negated: bool,
    pub(crate) items: Vec<ClassItem>,
}

/// One member of a character class.
#[derive(Debug, Clone)]
pub(crate) enum ClassItem {
    /// A single character.
    Char(char),
    /// An inclusive range such as `a-z`.
    Range(char, char),
    /// `\d` when `negated` is false, `\D` when true.
    Digit(bool),
    /// `\w` when `negated` is false, `\W` when true.
    Word(bool),
    /// `\s` when `negated` is false, `\S` when true.
    Space(bool),
}

/// One instruction of the compiled program.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Inst {
    Char(char),
    /// Index into the engine's class table.
    Class(usize),
    Any,
    Start,
    End,
    WordBoundary,
    NotWordBoundary,
    /// Try the first target; if the rest of the match fails, try the second.
    Split(usize, usize),
    Jump(usize),
    /// Record the current position in a capture slot.
    Save(usize),
    /// Run a sub-program without consuming input.
    Look {
        prog: usize,
        negative: bool,
    },
    /// Fail unless the input has advanced since `slot` was written, then jump
    /// back to `target`.
    ///
    /// This is what stops `(?:x?)*` from looping forever: a repetition whose
    /// body matched the empty string is not a repetition, and JavaScript stops
    /// rather than spinning. Failing here instead of looping lets the enclosing
    /// `Split` take its other branch, which is the same outcome.
    CheckProgress {
        slot: usize,
        target: usize,
    },
    Accept,
}

/// The parsed form of a pattern, before it is compiled.
#[derive(Debug, Clone)]
enum Ast {
    Empty,
    Char(char),
    Any,
    Class(ClassDef),
    Start,
    End,
    WordBoundary,
    NotWordBoundary,
    /// A group. `index` is `None` for `(?:...)`, which does not capture.
    Group {
        index: Option<usize>,
        node: Box<Ast>,
    },
    Look {
        node: Box<Ast>,
        negative: bool,
    },
    Alt(Vec<Ast>),
    Seq(Vec<Ast>),
    Repeat {
        node: Box<Ast>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
}

/// A compiled pattern, ready to run.
#[derive(Debug)]
pub(crate) struct Regex {
    /// `programs[0]` is the whole pattern; the rest are lookahead bodies.
    programs: Vec<Vec<Inst>>,
    classes: Vec<ClassDef>,
    /// Group names by group number; entry 0 is unused.
    names: Vec<Option<String>>,
    /// How many capturing groups the pattern has.
    ngroups: usize,
    /// How many slots the virtual machine needs, capture and progress together.
    slot_count: usize,
    case_insensitive: bool,
}

/// The capture slots a successful match filled.
#[derive(Debug, Clone)]
pub(crate) struct Captures {
    /// `(start, end)` in characters, or `None` if the group did not participate.
    pub(crate) slots: Vec<Option<(usize, usize)>>,
    names: Vec<Option<String>>,
}

impl Captures {
    /// The named groups that participated, in the order they were declared.
    ///
    /// A name can be declared more than once - the reference's pattern has
    /// `street_0`, `street_1` and `street_2` - and the last one that matched
    /// wins, which is what assigning into a map in declaration order does.
    pub(crate) fn named(&self) -> Vec<(String, (usize, usize))> {
        let mut out: Vec<(String, (usize, usize))> = Vec::new();
        for group in 1..=self.names.len().saturating_sub(1) {
            let Some(Some(name)) = self.names.get(group) else {
                continue;
            };
            let Some(Some(value)) = self.slots.get(group) else {
                continue;
            };
            if let Some(existing) = out.iter_mut().find(|(n, _)| n == name) {
                existing.1 = *value;
            } else {
                out.push((name.clone(), *value));
            }
        }
        out
    }
}

/// Whether `c` is a word character, using JavaScript's ASCII definition.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Whether `c` is whitespace, using JavaScript's definition.
///
/// This is wider than `char::is_whitespace`: it includes the byte-order mark and
/// the no-break spaces, which a document pasted out of a word processor can
/// carry.
fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// Whether `c` ends a line, which is what `.` refuses to match.
fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// A recursive-descent parser for the pattern subset described above.
struct Parser {
    chars: Vec<char>,
    pos: usize,
    names: Vec<Option<String>>,
}

impl Parser {
    fn new(pattern: &str) -> Self {
        Self {
            chars: pattern.chars().collect(),
            pos: 0,
            // Slot 0 is the whole match, which the caller tracks itself.
            names: vec![None],
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_alternation(&mut self) -> Ast {
        let mut branches = vec![self.parse_sequence()];
        while self.eat('|') {
            branches.push(self.parse_sequence());
        }
        if branches.len() == 1 {
            branches.pop().unwrap_or(Ast::Empty)
        } else {
            Ast::Alt(branches)
        }
    }

    fn parse_sequence(&mut self) -> Ast {
        let mut nodes = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            nodes.push(self.parse_repeat());
        }
        match nodes.len() {
            0 => Ast::Empty,
            1 => nodes.pop().unwrap_or(Ast::Empty),
            _ => Ast::Seq(nodes),
        }
    }

    fn parse_repeat(&mut self) -> Ast {
        let atom = self.parse_atom();
        let (min, max) = match self.peek() {
            Some('*') => {
                self.pos += 1;
                (0, None)
            }
            Some('+') => {
                self.pos += 1;
                (1, None)
            }
            Some('?') => {
                self.pos += 1;
                (0, Some(1))
            }
            Some('{') => match self.try_brace() {
                Some(bounds) => bounds,
                None => return atom,
            },
            _ => return atom,
        };
        // A trailing `?` makes the quantifier lazy, so the shortest match is
        // tried first.
        let greedy = !self.eat('?');
        Ast::Repeat {
            node: Box::new(atom),
            min,
            max,
            greedy,
        }
    }

    /// Parses `{n}`, `{n,}` or `{n,m}`, or rewinds if it is not a quantifier.
    fn try_brace(&mut self) -> Option<(u32, Option<u32>)> {
        let start = self.pos;
        self.pos += 1;
        let Some(min) = self.parse_number() else {
            self.pos = start;
            return None;
        };
        if self.eat('}') {
            return Some((min, Some(min)));
        }
        if !self.eat(',') {
            self.pos = start;
            return None;
        }
        if self.eat('}') {
            return Some((min, None));
        }
        let Some(max) = self.parse_number() else {
            self.pos = start;
            return None;
        };
        if !self.eat('}') {
            self.pos = start;
            return None;
        }
        Some((min, Some(max)))
    }

    fn parse_number(&mut self) -> Option<u32> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse()
            .ok()
    }

    fn parse_atom(&mut self) -> Ast {
        match self.bump() {
            None => Ast::Empty,
            Some('(') => self.parse_group(),
            Some('[') => Ast::Class(self.parse_class()),
            Some('.') => Ast::Any,
            Some('^') => Ast::Start,
            Some('$') => Ast::End,
            Some('\\') => self.parse_escape(),
            Some(c) => Ast::Char(c),
        }
    }

    fn parse_group(&mut self) -> Ast {
        // `(?` introduces a named group, a non-capturing group or a lookahead.
        let (index, negative_look) = if self.eat('?') {
            match self.peek() {
                Some(':') => {
                    self.pos += 1;
                    (None, None)
                }
                Some('=') => {
                    self.pos += 1;
                    (None, Some(false))
                }
                Some('!') => {
                    self.pos += 1;
                    (None, Some(true))
                }
                Some('<') => {
                    self.pos += 1;
                    let name = self.parse_name();
                    self.names.push(Some(name));
                    (Some(self.names.len() - 1), None)
                }
                _ => (None, None),
            }
        } else {
            self.names.push(None);
            (Some(self.names.len() - 1), None)
        };

        let node = self.parse_alternation();
        // A group that is never closed still yields its body, which keeps a
        // truncated pattern from looping forever.
        self.eat(')');

        if let Some(negative) = negative_look {
            return Ast::Look {
                node: Box::new(node),
                negative,
            };
        }
        Ast::Group {
            index,
            node: Box::new(node),
        }
    }

    fn parse_name(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(|c| c != '>') {
            self.pos += 1;
        }
        let name: String = self.chars[start..self.pos].iter().collect();
        self.eat('>');
        name
    }

    fn parse_escape(&mut self) -> Ast {
        match self.bump() {
            None => Ast::Char('\\'),
            Some('d') => Ast::Class(single(ClassItem::Digit(false))),
            Some('D') => Ast::Class(single(ClassItem::Digit(true))),
            Some('w') => Ast::Class(single(ClassItem::Word(false))),
            Some('W') => Ast::Class(single(ClassItem::Word(true))),
            Some('s') => Ast::Class(single(ClassItem::Space(false))),
            Some('S') => Ast::Class(single(ClassItem::Space(true))),
            Some('b') => Ast::WordBoundary,
            Some('B') => Ast::NotWordBoundary,
            Some('n') => Ast::Char('\n'),
            Some('r') => Ast::Char('\r'),
            Some('t') => Ast::Char('\t'),
            Some('f') => Ast::Char('\u{000C}'),
            Some('v') => Ast::Char('\u{000B}'),
            Some('0') => Ast::Char('\0'),
            // Any other escaped character is that character, which is how `\.`
            // and `\ ` (an escaped space, kept by the extended-mode stripper)
            // become literals.
            Some(c) => Ast::Char(c),
        }
    }

    fn parse_class(&mut self) -> ClassDef {
        let negated = self.eat('^');
        let mut items = Vec::new();
        // A `]` in the first position is a literal, not the end of the class.
        let mut first = true;
        while let Some(c) = self.peek() {
            if c == ']' && !first {
                self.pos += 1;
                break;
            }
            first = false;
            let Some(low) = self.class_item() else { break };
            // A range only forms between two single characters.
            if self.peek() == Some('-')
                && self.peek_at(1).is_some_and(|n| n != ']')
                && let ClassItem::Char(start) = low
            {
                self.pos += 1;
                if let Some(ClassItem::Char(high)) = self.class_item() {
                    items.push(ClassItem::Range(start, high));
                    continue;
                }
                // The `-` was not part of a range after all.
                items.push(low);
                items.push(ClassItem::Char('-'));
                continue;
            }
            items.push(low);
        }
        ClassDef { negated, items }
    }

    fn class_item(&mut self) -> Option<ClassItem> {
        match self.bump()? {
            '\\' => Some(match self.bump()? {
                'd' => ClassItem::Digit(false),
                'D' => ClassItem::Digit(true),
                'w' => ClassItem::Word(false),
                'W' => ClassItem::Word(true),
                's' => ClassItem::Space(false),
                'S' => ClassItem::Space(true),
                'n' => ClassItem::Char('\n'),
                'r' => ClassItem::Char('\r'),
                't' => ClassItem::Char('\t'),
                'f' => ClassItem::Char('\u{000C}'),
                'v' => ClassItem::Char('\u{000B}'),
                // Inside a class `\b` is a backspace, not a word boundary.
                'b' => ClassItem::Char('\u{0008}'),
                c => ClassItem::Char(c),
            }),
            c => Some(ClassItem::Char(c)),
        }
    }
}

/// A class holding exactly one item, used for the `\d \w \s` shorthands.
fn single(item: ClassItem) -> ClassDef {
    ClassDef {
        negated: false,
        items: vec![item],
    }
}

/// Where compilation writes programs and the class table.
struct Compiler {
    programs: Vec<Vec<Inst>>,
    classes: Vec<ClassDef>,
    /// How many capture slots are in use, including the progress slots that
    /// unbounded quantifiers add.
    slots: usize,
}

impl Compiler {
    /// Appends `node`'s instructions to `out`.
    fn emit(&mut self, node: &Ast, out: &mut Vec<Inst>) {
        match node {
            Ast::Empty => {}
            Ast::Char(c) => out.push(Inst::Char(*c)),
            Ast::Any => out.push(Inst::Any),
            Ast::Start => out.push(Inst::Start),
            Ast::End => out.push(Inst::End),
            Ast::WordBoundary => out.push(Inst::WordBoundary),
            Ast::NotWordBoundary => out.push(Inst::NotWordBoundary),
            Ast::Class(def) => {
                self.classes.push(def.clone());
                out.push(Inst::Class(self.classes.len() - 1));
            }
            Ast::Seq(nodes) => {
                for child in nodes {
                    self.emit(child, out);
                }
            }
            Ast::Alt(branches) => {
                let mut jumps = Vec::new();
                for (i, branch) in branches.iter().enumerate() {
                    if i + 1 == branches.len() {
                        self.emit(branch, out);
                    } else {
                        let split = out.len();
                        out.push(Inst::Split(0, 0));
                        let body = out.len();
                        self.emit(branch, out);
                        jumps.push(out.len());
                        out.push(Inst::Jump(0));
                        out[split] = Inst::Split(body, out.len());
                    }
                }
                let end = out.len();
                for jump in jumps {
                    out[jump] = Inst::Jump(end);
                }
            }
            Ast::Group { index, node } => {
                if let Some(group) = index {
                    out.push(Inst::Save(2 * group));
                }
                self.emit(node, out);
                if let Some(group) = index {
                    out.push(Inst::Save(2 * group + 1));
                }
            }
            Ast::Look { node, negative } => {
                // Programs are numbered with the whole pattern at index 0, so a
                // lookahead body starts one past the compiler's own list.
                let index = self.programs.len() + 1;
                self.programs.push(Vec::new());
                let mut body = Vec::new();
                self.emit(node, &mut body);
                body.push(Inst::Accept);
                self.programs[index - 1] = body;
                out.push(Inst::Look {
                    prog: index,
                    negative: *negative,
                });
            }
            Ast::Repeat {
                node,
                min,
                max,
                greedy,
            } => self.emit_repeat(node, *min, *max, *greedy, out),
        }
    }

    /// Emits a quantifier.
    ///
    /// `{n,m}` is unrolled into `n` copies followed by `m - n` optional copies,
    /// and an unbounded quantifier into its mandatory copies followed by a loop.
    /// That keeps the program free of counters: the backtracking stack is the
    /// only state.
    fn emit_repeat(
        &mut self,
        node: &Ast,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        out: &mut Vec<Inst>,
    ) {
        for _ in 0..min {
            self.emit(node, out);
        }
        match max {
            // `{n}` is exactly `n` copies and nothing else.
            Some(max) if max <= min => {}
            // `{n,m}`: `m - n` optional copies, each able to skip to the end.
            Some(max) => {
                let mut splits = Vec::new();
                for _ in min..max {
                    let split = out.len();
                    out.push(Inst::Split(0, 0));
                    splits.push(split);
                    let body = out.len();
                    self.emit(node, out);
                    // The two targets are patched once the end is known.
                    out[split] = Inst::Split(body, body);
                }
                let end = out.len();
                for split in splits {
                    let Inst::Split(body, _) = out[split] else {
                        continue;
                    };
                    out[split] = if greedy {
                        Inst::Split(body, end)
                    } else {
                        Inst::Split(end, body)
                    };
                }
            }
            // `*` and `+`: a loop back to the split. A dedicated progress slot
            // records where the previous iteration ended, so an iteration that
            // consumed nothing ends the loop instead of repeating forever.
            None => {
                let slot = self.slots;
                self.slots += 2;
                let split = out.len();
                out.push(Inst::Split(0, 0));
                let body = out.len();
                out.push(Inst::Save(slot));
                self.emit(node, out);
                out.push(Inst::CheckProgress {
                    slot,
                    target: split,
                });
                let end = out.len();
                out[split] = if greedy {
                    Inst::Split(body, end)
                } else {
                    Inst::Split(end, body)
                };
            }
        }
    }
}

impl Regex {
    /// Compiles `pattern`. `case_insensitive` corresponds to JavaScript's `i`.
    pub(crate) fn compile(pattern: &str, case_insensitive: bool) -> Self {
        let mut parser = Parser::new(pattern);
        let ast = parser.parse_alternation();
        let mut compiler = Compiler {
            programs: Vec::new(),
            classes: Vec::new(),
            slots: 2 * (parser.names.len()),
        };
        let mut top = Vec::new();
        compiler.emit(&ast, &mut top);
        top.push(Inst::Accept);
        let mut programs = vec![top];
        programs.extend(compiler.programs);
        let ngroups = parser.names.len() - 1;
        Self {
            programs,
            classes: compiler.classes,
            names: parser.names,
            ngroups,
            slot_count: compiler.slots,
            case_insensitive,
        }
    }

    /// Whether `text` matches anywhere, which is what JavaScript's `test` does.
    pub(crate) fn is_match(&self, text: &[char]) -> bool {
        (0..=text.len()).any(|start| self.find_at(text, start).is_some())
    }

    /// The leftmost match, which is what JavaScript's `exec` returns.
    pub(crate) fn exec(&self, text: &[char]) -> Option<Captures> {
        for start in 0..=text.len() {
            if let Some((end, slots)) = self.find_at(text, start) {
                let mut out = vec![None; self.ngroups + 1];
                out[0] = Some((start, end));
                for group in 1..=self.ngroups {
                    out[group] = match (slots[2 * group], slots[2 * group + 1]) {
                        (Some(s), Some(e)) => Some((s, e)),
                        _ => None,
                    };
                }
                return Some(Captures {
                    slots: out,
                    names: self.names.clone(),
                });
            }
        }
        None
    }

    /// Runs the program from `start`, returning where the match ended and the
    /// capture slots it filled.
    fn find_at(&self, text: &[char], start: usize) -> Option<(usize, Vec<Option<usize>>)> {
        let mut vm = Vm {
            regex: self,
            text,
            slots: vec![None; self.slot_count],
            trail: Vec::new(),
        };
        vm.run(0, 0, start).map(|end| (end, vm.slots))
    }
}

/// A backtracking matcher over a compiled program.
struct Vm<'a> {
    regex: &'a Regex,
    text: &'a [char],
    /// Capture slots: group `g` uses `2g` for its start and `2g + 1` for its end.
    slots: Vec<Option<usize>>,
    /// Every slot write, so a failed branch can be undone without copying.
    trail: Vec<(usize, Option<usize>)>,
}

impl Vm<'_> {
    /// Undoes every slot write made since `mark`.
    fn undo(&mut self, mark: usize) {
        while self.trail.len() > mark {
            let (slot, old) = self.trail.pop().unwrap_or((0, None));
            self.slots[slot] = old;
        }
    }

    fn run(&mut self, prog: usize, mut pc: usize, mut pos: usize) -> Option<usize> {
        loop {
            let inst = *self.regex.programs[prog].get(pc)?;
            match inst {
                Inst::Accept => return Some(pos),
                Inst::Char(c) => {
                    if pos < self.text.len() && self.eq(c, self.text[pos]) {
                        pos += 1;
                        pc += 1;
                    } else {
                        return None;
                    }
                }
                Inst::Class(index) => {
                    if pos < self.text.len() && self.class_matches(index, self.text[pos]) {
                        pos += 1;
                        pc += 1;
                    } else {
                        return None;
                    }
                }
                Inst::Any => {
                    if pos < self.text.len() && !is_line_terminator(self.text[pos]) {
                        pos += 1;
                        pc += 1;
                    } else {
                        return None;
                    }
                }
                Inst::Start => {
                    if pos == 0 {
                        pc += 1;
                    } else {
                        return None;
                    }
                }
                Inst::End => {
                    if pos == self.text.len() {
                        pc += 1;
                    } else {
                        return None;
                    }
                }
                Inst::WordBoundary | Inst::NotWordBoundary => {
                    let before = pos > 0 && is_word(self.text[pos - 1]);
                    let after = pos < self.text.len() && is_word(self.text[pos]);
                    let boundary = before != after;
                    let want = matches!(inst, Inst::WordBoundary);
                    if boundary == want {
                        pc += 1;
                    } else {
                        return None;
                    }
                }
                Inst::CheckProgress { slot, target } => {
                    // A repetition that consumed nothing has to stop, so fail
                    // here and let the enclosing `Split` choose its other branch.
                    if self.slots[slot] == Some(pos) {
                        return None;
                    }
                    self.trail.push((slot, self.slots[slot]));
                    self.slots[slot] = Some(pos);
                    pc = target;
                }
                Inst::Jump(target) => pc = target,
                Inst::Save(slot) => {
                    self.trail.push((slot, self.slots[slot]));
                    self.slots[slot] = Some(pos);
                    pc += 1;
                }
                Inst::Split(first, second) => {
                    let mark = self.trail.len();
                    if let Some(end) = self.run(prog, first, pos) {
                        return Some(end);
                    }
                    self.undo(mark);
                    pc = second;
                }
                Inst::Look {
                    prog: sub,
                    negative,
                } => {
                    let mark = self.trail.len();
                    let matched = self.run(sub, 0, pos).is_some();
                    if matched != negative {
                        // A positive lookahead that matched keeps whatever it
                        // captured, which is what JavaScript does; a negative
                        // one never captured anything worth keeping.
                        pc += 1;
                    } else {
                        // A lookahead consumes no input, so a failed one leaves
                        // no trace behind.
                        self.undo(mark);
                        return None;
                    }
                }
            }
        }
    }

    fn eq(&self, pattern: char, text: char) -> bool {
        if self.regex.case_insensitive {
            pattern.eq_ignore_ascii_case(&text)
        } else {
            pattern == text
        }
    }

    fn class_matches(&self, index: usize, c: char) -> bool {
        let def = &self.regex.classes[index];
        let hit = def.items.iter().any(|item| match item {
            ClassItem::Char(p) => self.eq(*p, c),
            ClassItem::Range(low, high) => self.in_range(*low, *high, c),
            ClassItem::Digit(negated) => c.is_ascii_digit() != *negated,
            ClassItem::Word(negated) => is_word(c) != *negated,
            ClassItem::Space(negated) => is_space(c) != *negated,
        });
        hit != def.negated
    }

    fn in_range(&self, low: char, high: char, c: char) -> bool {
        if (low..=high).contains(&c) {
            return true;
        }
        if self.regex.case_insensitive {
            let lower = c.to_ascii_lowercase();
            let upper = c.to_ascii_uppercase();
            return (low..=high).contains(&lower) || (low..=high).contains(&upper);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Regex;

    /// The leftmost match, as `[whole, group1, group2, ...]`, with an empty
    /// string for a group that did not participate.
    fn find(pattern: &str, case_insensitive: bool, text: &str) -> Option<Vec<String>> {
        let regex = Regex::compile(pattern, case_insensitive);
        let chars: Vec<char> = text.chars().collect();
        let captures = regex.exec(&chars)?;
        Some(
            captures
                .slots
                .iter()
                .map(|slot| match slot {
                    Some((start, end)) => chars[*start..*end].iter().collect(),
                    None => String::new(),
                })
                .collect(),
        )
    }

    /// The expectations in this module were taken from JavaScript's own
    /// `RegExp`, so a disagreement is a bug in this engine rather than a
    /// difference of opinion about what a pattern means.
    #[test]
    fn alternation_and_groups() {
        assert_eq!(find("a(b|c)d", false, "abcd"), None);
        assert_eq!(
            find("a(b|c)d", false, "acd"),
            Some(vec!["acd".into(), "c".into()])
        );
        assert_eq!(
            find("a(b|c)d", false, "abd"),
            Some(vec!["abd".into(), "b".into()])
        );
    }

    #[test]
    fn named_groups_are_reported_in_declaration_order() {
        let regex = Regex::compile("(?<d>N|S)\\s+", true);
        let chars: Vec<char> = "N Main".chars().collect();
        let captures = regex.exec(&chars).unwrap();
        assert_eq!(captures.named(), vec![("d".to_string(), (0, 1))]);
    }

    /// A later group with the same name overwrites an earlier one, which is what
    /// assigning into a map in declaration order does.
    #[test]
    fn a_repeated_name_keeps_the_last_match() {
        let regex = Regex::compile("(?<a>x)|(?<a>y)", false);
        let chars: Vec<char> = "y".chars().collect();
        let captures = regex.exec(&chars).unwrap();
        assert_eq!(captures.named(), vec![("a".to_string(), (0, 1))]);
    }

    #[test]
    fn lookahead_does_not_consume() {
        assert_eq!(
            find("^(?<d>N|S)\\s+(?=\\S)", true, "N Main"),
            Some(vec!["N ".into(), "N".into()])
        );
        assert_eq!(
            find("^(?<d>N|S)\\s+(?=\\S)", true, "N  Main"),
            Some(vec!["N  ".into(), "N".into()])
        );
        // The lookahead fails on trailing whitespace, so nothing matches.
        assert_eq!(find("^(?<d>N|S)\\s+(?=\\S)", true, "N "), None);
    }

    #[test]
    fn negative_lookahead() {
        assert_eq!(find("(?!foo)\\w+", false, "bar"), Some(vec!["bar".into()]));
        // `foo` itself fails, so the match starts one character later.
        assert_eq!(find("(?!foo)\\w+", false, "foo"), Some(vec!["oo".into()]));
    }

    #[test]
    fn repetition_counts() {
        // A bounded repeat takes as much as it can, and needs at least the low
        // bound, which is why `1` alone does not match `\d{2,4}`.
        assert_eq!(find("\\d{2,4}", false, "12345"), Some(vec!["1234".into()]));
        assert_eq!(find("\\d{2,4}", false, "1"), None);
        assert_eq!(find("a{3}", false, "baaab"), Some(vec!["aaa".into()]));
    }

    #[test]
    fn lazy_quantifiers_stop_early() {
        assert_eq!(find("a.*?b", false, "axxbxxb"), Some(vec!["axxb".into()]));
        assert_eq!(
            find("(?:ab)+?c", false, "ababc"),
            Some(vec!["ababc".into()])
        );
    }

    #[test]
    fn word_boundaries() {
        assert_eq!(
            find("\\bfoo\\b", false, "a foo b"),
            Some(vec!["foo".into()])
        );
        assert_eq!(find("\\bfoo\\b", false, "afoob"), None);
    }

    #[test]
    fn classes() {
        assert_eq!(find("x[^a-c]z", false, "xdz"), Some(vec!["xdz".into()]));
        assert_eq!(find("x[^a-c]z", false, "xbz"), None);
        assert_eq!(find("\\w+@\\w+", false, "a@b"), Some(vec!["a@b".into()]));
        // A `-` at the end of a class is a literal, not a range.
        assert_eq!(find("[\\d-]+", false, "12-3"), Some(vec!["12-3".into()]));
        assert_eq!(find("[a-]", false, "-"), Some(vec!["-".into()]));
    }

    #[test]
    fn whitespace_and_dot() {
        assert_eq!(find("\\s+", false, "  a"), Some(vec!["  ".into()]));
        // JavaScript's `.` does not match a line terminator.
        assert_eq!(find(".", false, "\n"), None);
        assert_eq!(find(".", false, "a"), Some(vec!["a".into()]));
    }

    #[test]
    fn the_leftmost_match_wins() {
        assert_eq!(find("\\d+", false, "ab123cd456"), Some(vec!["123".into()]));
        assert_eq!(find("^a", false, "ba"), None);
        assert_eq!(find("a$", false, "ba"), Some(vec!["a".into()]));
    }

    /// A quantifier whose body can match nothing must still terminate.
    ///
    /// Without the engine's progress check, `(?:a?)*b` loops on the empty match
    /// forever rather than failing.
    #[test]
    fn a_zero_width_repeat_terminates() {
        assert_eq!(find("(?:a?)*b", false, "b"), Some(vec!["b".into()]));
        assert_eq!(find("(?:a?)*b", false, "aab"), Some(vec!["aab".into()]));
        assert_eq!(
            find("(a*)*b", false, "b"),
            Some(vec!["b".into(), "".into()])
        );
        assert_eq!(find("(?:)*x", false, "x"), Some(vec!["x".into()]));
    }

    #[test]
    fn case_insensitive_folds_ascii() {
        assert_eq!(find("abc", true, "ABC"), Some(vec!["ABC".into()]));
        assert_eq!(find("abc", false, "ABC"), None);
    }

    #[test]
    fn is_match_searches_anywhere() {
        let regex = Regex::compile("Main", false);
        let text: Vec<char> = "1005 Main St".chars().collect();
        assert!(regex.is_match(&text));
        let absent: Vec<char> = "1005 Oak St".chars().collect();
        assert!(!regex.is_match(&absent));
    }
}
