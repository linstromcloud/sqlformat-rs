use sqlformat::{Dialect, FormatOptions, QueryParams, format};

fn compact() -> FormatOptions<'static> {
    FormatOptions {
        uppercase: Some(true),
        max_inline_block: 100,
        max_inline_arguments: Some(100),
        max_inline_top_level: Some(100),
        joins_as_top_level: true,
        dialect: Dialect::PostgreSql,
        ..Default::default()
    }
}

#[test]
fn dollar_quotes_preserve_contents_and_parameter_spelling() {
    for literal in [
        "$$hello world$$",
        "$tag$hello world$tag$",
        "$$a  b$$",
        "$tag$' -- /* ; $1 $name$tag$",
    ] {
        for options in [FormatOptions::default(), compact()] {
            let query = format!("select {literal} as txt, $1;");
            let result = format(&query, &QueryParams::Indexed(vec!["42".into()]), &options);
            assert!(result.contains(literal), "{literal}: {result}");
            assert!(result.contains("42"));
            assert_eq!(result, format(&result, &QueryParams::None, &options));
        }
    }
}

#[test]
fn nested_comments_preserve_contents() {
    let comment = "/* outer\n  /* inner */ SELECT  a  b -- text\n*/";
    let query = format!("select 1 {comment} + 2 as n;");
    let result = format(&query, &QueryParams::None, &compact());
    assert!(result.contains(comment), "{result}");
    assert_eq!(result, format(&result, &QueryParams::None, &compact()));
}

#[test]
fn inline_preserves_line_comment_boundaries() {
    let options = FormatOptions {
        inline: true,
        ..compact()
    };
    for query in [
        "select 1 -- keep addition\n + 1 as n;",
        "-- leading\nselect 1;",
        "select 1; -- next\nselect 2;",
    ] {
        let result = format(query, &QueryParams::None, &options);
        assert!(result.contains('\n'), "{result}");
        assert_eq!(result, format(&result, &QueryParams::None, &options));
    }
}

#[test]
fn compact_keyword_classification_is_case_independent() {
    for query in [
        "with recursive state(id, step) using key (id) as (select 1, 0 union all select id, step + 1 from state where step < 3) select * from state;",
        "pivot (select country, id from users) on country using sum(id);",
    ] {
        let result = format(query, &QueryParams::None, &compact());
        assert_eq!(result, format(&result, &QueryParams::None, &compact()));
    }
}

#[test]
fn incomplete_literals_and_comments_keep_editor_text() {
    for query in [
        "select $$unfinished  ",
        "select $tag$unfinished  ",
        "select 'unfinished  ",
        "select 1 /* unfinished /* comment  ",
    ] {
        assert_eq!(
            format(query, &QueryParams::None, &FormatOptions::duckdb()),
            query
        );
    }
}

#[test]
fn duckdb_preserves_format_control_comments() {
    let query = "select 1;\n-- fmt: off\nselect  2+3;\n-- fmt: on\nselect 4;";
    let result = format(query, &QueryParams::None, &FormatOptions::duckdb());
    assert!(
        result.contains("-- fmt: off\nselect  2+3;\n-- fmt: on"),
        "{result}"
    );
    assert_eq!(
        result,
        format(&result, &QueryParams::None, &FormatOptions::duckdb())
    );
}
