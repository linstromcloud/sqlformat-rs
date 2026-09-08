use crate::duckdb_words::{FUNCTIONS, RESERVED};
use crate::tokenizer::{Token, TokenKind};

/// Reclassify unreserved words by their use, rather than capitalizing identifiers
/// that happen to occur in another dialect's keyword list.
pub(crate) fn classify(tokens: &mut [Token<'_>]) {
    let indices: Vec<_> = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind != TokenKind::Whitespace)
        .map(|(index, _)| index)
        .collect();
    let mut level = 0usize;
    let mut cte_levels = Vec::new();
    let mut array_levels = Vec::new();
    let mut comprehension_levels = Vec::new();
    for (position, &index) in indices.iter().enumerate() {
        let previous = position.checked_sub(1).map(|p| &tokens[indices[p]]);
        let next = indices.get(position + 1).map(|&p| &tokens[p]);
        let token = &tokens[index];
        if token.kind == TokenKind::OpenParen {
            level += 1;
            if token.value == "[" {
                array_levels.push(level);
            }
        } else if token.kind == TokenKind::CloseParen {
            level = level.saturating_sub(1);
            cte_levels.retain(|&depth| depth <= level);
            array_levels.retain(|&depth| depth <= level);
            comprehension_levels.retain(|&depth| depth <= level);
        }
        if !matches!(
            token.kind,
            TokenKind::Word
                | TokenKind::Reserved
                | TokenKind::ReservedTopLevel
                | TokenKind::ReservedTopLevelNoIndent
                | TokenKind::ReservedNewline
        ) {
            continue;
        }
        let word = token.value.to_ascii_lowercase();
        if word.contains(char::is_whitespace) {
            continue;
        }
        let after =
            |value: &str| previous.is_some_and(|token| token.value.eq_ignore_ascii_case(value));
        let before =
            |value: &str| next.is_some_and(|token| token.value.eq_ignore_ascii_case(value));
        if word == "for" && array_levels.contains(&level) {
            comprehension_levels.push(level);
        }
        let cte_name = cte_levels.contains(&level)
            && ((after("with") && word != "recursive") || after("recursive") || after(","));
        if word == "with" {
            cte_levels.push(level);
        } else if matches!(word.as_str(), "select" | "from" | "values") {
            cte_levels.retain(|&depth| depth != level);
        }
        let kind = if cte_name
            || after(".")
            || (after("as") && !(word == "materialized" && before("(")))
            || before(":")
        {
            TokenKind::Word
        } else if (matches!(word.as_str(), "filter" | "over") && after(")"))
            || (word == "if" && comprehension_levels.contains(&level) && !before("("))
        {
            TokenKind::Reserved
        } else if before("(")
            && (FUNCTIONS.binary_search(&word.as_str()).is_ok()
            // DuckDB parses these conditional expressions without catalog functions.
            || matches!(word.as_str(), "if" | "ifnull" | "nullif" | "coalesce"))
        {
            TokenKind::Function
        } else if matches!(
            word.as_str(),
            "qualify" | "pivot" | "unpivot" | "pivot_wider" | "pivot_longer"
        ) {
            TokenKind::ReservedTopLevel
        } else if word == "join" && !after("select") && !after(",") {
            token.kind
        } else if (word == "recursive" && after("with"))
            || (word == "key" && after("using"))
            || (matches!(word.as_str(), "date" | "time" | "timestamp" | "interval")
                && next.is_some_and(|token| {
                    token.kind == TokenKind::String && !token.value.starts_with('"')
                }))
            || (word == "materialized" && (after("as") || after("not")))
        {
            TokenKind::Reserved
        } else if RESERVED.binary_search(&word.as_str()).is_ok()
            || (matches!(
                token.kind,
                TokenKind::ReservedTopLevel | TokenKind::ReservedTopLevelNoIndent
            ) && !after("select")
                && !after(","))
        {
            if token.kind == TokenKind::Word {
                TokenKind::Reserved
            } else {
                token.kind
            }
        } else {
            TokenKind::Word
        };
        tokens[index].kind = kind;
    }
}
