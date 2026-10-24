mod parser;

#[cfg(feature = "lex")]
mod tantivy;

use std::collections::BTreeMap;

use crate::types::Frame;
use parser::{Expr, FieldTerm, Term, TextTerm};

pub(crate) use parser::parse_query;
pub(crate) use parser::{DateRange, ParsedQuery};

#[cfg(feature = "lex")]
#[allow(unused_imports)]
pub(crate) use tantivy::{
    EmbeddedLexSegment, EmbeddedLexStorage, LexWalBatch, TantivyEngine, TantivySnapshot,
};

pub struct EvaluationContext<'a> {
    pub frame: &'a Frame,
    pub content_lower: &'a str,
}

impl ParsedQuery {
    pub fn evaluate(&self, ctx: &EvaluationContext<'_>) -> bool {
        self.expr.evaluate(ctx)
    }

    /// Check whether fields permit a consistent assignment of semantic text predicates.
    pub(crate) fn permits_semantic_match(&self, ctx: &EvaluationContext<'_>) -> bool {
        let mut polarities = BTreeMap::new();
        self.expr.collect_text_polarities(false, &mut polarities);
        let mut assignments = BTreeMap::new();
        let mut choices = Vec::new();
        for (key, (positive, negative)) in polarities {
            // A predicate used in only one polarity can always satisfy that
            // polarity. Ordinary positive queries need no assignment search.
            let value = if positive && negative {
                choices.push(key.clone());
                None
            } else {
                Some(positive)
            };
            assignments.insert(key, value);
        }

        // Branch only on predicates occurring in both polarities. Partial
        // evaluation prunes impossible branches; all occurrences share a value.
        // Iterative backtracking avoids adding stack depth per text predicate.
        let mut depth = 0;
        loop {
            let (can_be_true, can_be_false) = self.expr.semantic_truth(ctx, &assignments);
            if !can_be_false {
                return true;
            }
            if can_be_true && depth < choices.len() {
                assignments.insert(choices[depth].clone(), Some(false));
                depth += 1;
                continue;
            }
            while depth > 0 {
                let key = &choices[depth - 1];
                if assignments.get(key) == Some(&Some(false)) {
                    assignments.insert(key.clone(), Some(true));
                    break;
                }
                assignments.insert(key.clone(), None);
                depth -= 1;
            }
            if depth == 0 {
                return false;
            }
        }
    }

    pub fn text_tokens(&self) -> Vec<String> {
        self.expr.collect_tokens()
    }

    pub fn required_date_range(&self) -> Option<DateRange> {
        self.expr.required_date_range()
    }

    pub fn contains_field_terms(&self) -> bool {
        self.expr.contains_field_terms()
    }
}

impl TextTerm {
    // Word and phrase matching use the same case-folded substring predicate.
    // Wildcards retain their distinct regex semantics.
    fn semantic_key(&self) -> String {
        match self {
            TextTerm::Word(text) | TextTerm::Phrase(text) => {
                format!("literal:{}", text.to_lowercase())
            }
            TextTerm::Wildcard(pattern) => format!("wildcard:{}", pattern.regex.as_str()),
        }
    }

    pub(crate) fn matches(&self, haystack: &str) -> bool {
        match self {
            TextTerm::Word(word) => {
                let needle = word.to_lowercase();
                haystack.contains(&needle)
            }
            TextTerm::Phrase(phrase) => {
                let needle = phrase.to_lowercase();
                haystack.contains(&needle)
            }
            TextTerm::Wildcard(pattern) => pattern.regex.is_match(haystack),
        }
    }
}

impl FieldTerm {
    pub(crate) fn matches(&self, ctx: &EvaluationContext<'_>) -> bool {
        match self {
            FieldTerm::Uri(value) => ctx
                .frame
                .uri
                .as_deref()
                .is_some_and(|uri| uri.eq_ignore_ascii_case(value)),
            FieldTerm::Scope(prefix) => ctx
                .frame
                .uri
                .as_deref()
                .is_some_and(|uri| uri.starts_with(prefix)),
            FieldTerm::Track(track) => ctx
                .frame
                .track
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(track)),
            FieldTerm::Tag(tag) => ctx
                .frame
                .tags
                .iter()
                .any(|value| value.eq_ignore_ascii_case(tag)),
            FieldTerm::Label(label) => ctx
                .frame
                .labels
                .iter()
                .any(|value| value.eq_ignore_ascii_case(label)),
            FieldTerm::DateRange(range) => range.matches(ctx.frame),
        }
    }
}

impl DateRange {
    fn matches(&self, frame: &Frame) -> bool {
        if self.start.is_none() && self.end.is_none() {
            return true;
        }
        let mut candidates = Vec::new();
        candidates.push(frame.timestamp);
        #[cfg(feature = "temporal_track")]
        if let Some(anchor_ts) = frame.anchor_ts {
            candidates.push(anchor_ts);
        }
        for date_str in &frame.content_dates {
            if let Some(ts) = parser::parse_date_value(date_str) {
                candidates.push(ts);
            }
        }
        candidates.into_iter().any(|ts| self.contains(ts))
    }

    pub(crate) fn contains(&self, timestamp: i64) -> bool {
        if let Some(start) = self.start {
            if timestamp < start {
                return false;
            }
        }
        if let Some(end) = self.end {
            if timestamp > end {
                return false;
            }
        }
        true
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let start = match (self.start, other.start) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let end = match (self.end, other.end) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        match (self.start, self.end) {
            (Some(start), Some(end)) => start > end,
            _ => false,
        }
    }
}

impl Expr {
    fn collect_text_polarities(
        &self,
        negated: bool,
        polarities: &mut BTreeMap<String, (bool, bool)>,
    ) {
        match self {
            Expr::Term(Term::Text(text)) => {
                let entry = polarities.entry(text.semantic_key()).or_default();
                if negated {
                    entry.1 = true;
                } else {
                    entry.0 = true;
                }
            }
            Expr::Not(child) => child.collect_text_polarities(!negated, polarities),
            Expr::And(children) | Expr::Or(children) => {
                for child in children {
                    child.collect_text_polarities(negated, polarities);
                }
            }
            Expr::Term(Term::Field(_)) => {}
        }
    }

    // (can be true, can be false) under one shared, partial assignment.
    fn semantic_truth(
        &self,
        ctx: &EvaluationContext<'_>,
        assignments: &BTreeMap<String, Option<bool>>,
    ) -> (bool, bool) {
        match self {
            Expr::Term(Term::Text(text)) => assignments
                .get(&text.semantic_key())
                .copied()
                .flatten()
                .map_or((true, true), |value| (value, !value)),
            Expr::Term(Term::Field(field)) => {
                let matched = field.matches(ctx);
                (matched, !matched)
            }
            Expr::Not(child) => {
                let (yes, no) = child.semantic_truth(ctx, assignments);
                (no, yes)
            }
            Expr::And(children) => children.iter().fold((true, false), |(yes, no), child| {
                let (child_yes, child_no) = child.semantic_truth(ctx, assignments);
                (yes && child_yes, no || child_no)
            }),
            Expr::Or(children) => children.iter().fold((false, true), |(yes, no), child| {
                let (child_yes, child_no) = child.semantic_truth(ctx, assignments);
                (yes || child_yes, no && child_no)
            }),
        }
    }

    fn evaluate(&self, ctx: &EvaluationContext<'_>) -> bool {
        match self {
            Expr::Or(children) => children.iter().any(|child| child.evaluate(ctx)),
            Expr::And(children) => children.iter().all(|child| child.evaluate(ctx)),
            Expr::Not(child) => !child.evaluate(ctx),
            Expr::Term(term) => term.evaluate(ctx),
        }
    }

    fn collect_tokens(&self) -> Vec<String> {
        let mut tokens = Vec::new();
        self.collect_into(&mut tokens);
        tokens
    }

    fn collect_into(&self, tokens: &mut Vec<String>) {
        match self {
            Expr::Or(children) | Expr::And(children) => {
                for child in children {
                    child.collect_into(tokens);
                }
            }
            Expr::Not(child) => child.collect_into(tokens),
            Expr::Term(Term::Text(text)) => match text {
                TextTerm::Word(word) | TextTerm::Phrase(word) => tokens.push(word.clone()),
                TextTerm::Wildcard(pattern) => {
                    if let Some(seed) = pattern.seed() {
                        tokens.push(seed);
                    }
                }
            },
            Expr::Term(Term::Field(_)) => {}
        }
    }

    fn required_date_range(&self) -> Option<DateRange> {
        match self {
            Expr::Term(Term::Field(FieldTerm::DateRange(range))) => Some(range.clone()),
            Expr::And(children) => {
                let mut combined: Option<DateRange> = None;
                for child in children {
                    if let Some(child_range) = child.required_date_range() {
                        combined = Some(match combined {
                            Some(existing) => existing.intersection(&child_range),
                            None => child_range,
                        });
                    }
                }
                combined
            }
            Expr::Or(_) | Expr::Not(_) => None,
            Expr::Term(_) => None,
        }
    }

    fn contains_field_terms(&self) -> bool {
        match self {
            Expr::Or(children) | Expr::And(children) => {
                children.iter().any(parser::Expr::contains_field_terms)
            }
            Expr::Not(child) => child.contains_field_terms(),
            Expr::Term(term) => term.contains_field_terms(),
        }
    }
}

impl Term {
    fn evaluate(&self, ctx: &EvaluationContext<'_>) -> bool {
        match self {
            Term::Text(text) => text.matches(ctx.content_lower),
            Term::Field(field) => field.matches(ctx),
        }
    }

    fn contains_field_terms(&self) -> bool {
        matches!(self, Term::Field(_))
    }
}
