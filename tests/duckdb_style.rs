use sqlformat::{FormatOptions, QueryParams, format};

#[test]
fn distinguishes_comprehension_keywords_from_function_names() {
    let query = "select [x * 2 for x in [1, 2, 3] if x > 1] as xs, IF(true, 1, 0) as n;";
    let expected = "SELECT [x * 2 FOR x IN [1, 2, 3] IF x > 1] AS xs, if(TRUE, 1, 0) AS n;";
    assert_eq!(
        format(query, &QueryParams::None, &FormatOptions::duckdb()),
        expected
    );
}

#[test]
fn separates_ctes_and_keeps_predicates_at_one_indent() {
    let expected = "WITH first AS (\n  SELECT id, country\n  FROM users\n),\n\nsecond AS (\n  SELECT user_id, amount\n  FROM purchases\n)\nSELECT first.id, second.amount\nFROM first\nJOIN second ON first.id = second.user_id\nWHERE second.amount > 0\n  AND first.country = 'SE'\n  AND first.id > 0;";
    assert_eq!(
        format(expected, &QueryParams::None, &FormatOptions::duckdb()),
        expected
    );
}

#[test]
fn preserves_identifier_names_that_match_functions() {
    let query = "WITH SUM(Amount) AS (SELECT 1), COUNT(Total) AS (SELECT 2) SELECT SUM.Amount, COUNT.Total, SUM(SUM.Amount) AS Day FROM SUM CROSS JOIN COUNT GROUP BY ALL;";
    let result = format(query, &QueryParams::None, &FormatOptions::duckdb());
    for identifier in [
        "WITH SUM(Amount)",
        "COUNT(Total)",
        "SUM.Amount",
        "COUNT.Total",
        "AS Day",
        "FROM SUM",
    ] {
        assert!(
            result.contains(identifier),
            "missing {identifier}: {result}"
        );
    }
    assert!(result.contains("sum(SUM.Amount)"), "{result}");
}

#[test]
fn long_column_lists_and_whitespace_variants_are_stable() {
    let queries = [
        "select 0 as col_0,1 as col_1,2 as col_2,3 as col_3,4 as col_4,5 as col_5,6 as col_6,7 as col_7;",
        "SELECT 0 AS col_0, 1 AS col_1, 2 AS col_2, 3 AS col_3, 4 AS col_4, 5 AS col_5, 6 AS col_6, 7 AS col_7;",
    ];
    let options = FormatOptions::duckdb();
    let result = format(queries[0], &QueryParams::None, &options);
    assert_eq!(result, format(queries[1], &QueryParams::None, &options));
    assert_eq!(result, format(&result, &QueryParams::None, &options));
    assert!(
        result.lines().all(|line| line.chars().count() <= 100),
        "{result}"
    );
}

#[test]
fn approved_compact_examples() {
    for expected in [
        r###"SELECT id, country FROM users WHERE id = 1;"###,
        r###"WITH daily AS (
  SELECT date_trunc('day', created_at) AS day, user_id, sum(amount) AS revenue
  FROM purchases
  WHERE status = 'completed'
    AND created_at >= DATE '2026-01-01'
  GROUP BY day, user_id
)
SELECT d.day, u.country, sum(d.revenue) AS revenue
FROM daily AS d
JOIN users AS u ON u.id = d.user_id
GROUP BY d.day, u.country
ORDER BY d.day, revenue DESC;"###,
        r###"SELECT
  date_trunc('month', created_at) AS month,
  count(DISTINCT user_id) AS active_users,
  sum(amount) FILTER (WHERE status = 'completed') AS revenue
FROM purchases
GROUP BY month;"###,
    ] {
        let result = format(expected, &QueryParams::None, &FormatOptions::duckdb());
        assert_eq!(result, expected);
    }
}

#[test]
fn duckdb_corpus_is_idempotent_and_preserves_protected_text() {
    for (name, sql, protected) in [
        (
            "short_query",
            r###"select id, country from users where id = 1;"###,
            &[][..],
        ),
        (
            "approved_cte",
            r###"WITH daily AS (
  SELECT date_trunc('day', created_at) AS day, user_id, sum(amount) AS revenue
  FROM purchases
  WHERE status = 'completed'
    AND created_at >= DATE '2026-01-01'
  GROUP BY day, user_id
)
SELECT d.day, u.country, sum(d.revenue) AS revenue
FROM daily AS d
JOIN users AS u ON u.id = d.user_id
GROUP BY d.day, u.country
ORDER BY d.day, revenue DESC;"###,
            &[][..],
        ),
        (
            "approved_long_list",
            r###"SELECT
  date_trunc('month', created_at) AS month,
  count(DISTINCT user_id) AS active_users,
  sum(amount) FILTER (WHERE status = 'completed') AS revenue
FROM purchases
GROUP BY month;"###,
            &[][..],
        ),
        (
            "two_ctes",
            r###"WITH a AS (
  SELECT id, country FROM users
),

b AS (
  SELECT user_id, sum(amount) AS revenue FROM purchases GROUP BY user_id
)
SELECT a.id, a.country, b.revenue
FROM a
JOIN b ON a.id = b.user_id;"###,
            &[][..],
        ),
        (
            "joins_predicates",
            r###"select u.id, p.amount from users as u left join purchases as p on p.user_id = u.id and p.status = 'completed' where u.country = 'SE' and (p.amount > 1 or p.amount is null) order by u.id, p.amount;"###,
            &[][..],
        ),
        (
            "short_case_cast_math",
            r###"select id, case when country = 'SE' then 'local' else 'foreign' end as region, id::integer * 2 + 1 as score from users;"###,
            &[][..],
        ),
        (
            "window_qualify",
            r###"select user_id, amount, row_number() over (partition by user_id order by amount desc) as rn from purchases qualify rn = 1 order by user_id;"###,
            &[][..],
        ),
        (
            "function_casing",
            r###"select SUM(amount) as Total, CoUnT(*) as N, date_trunc('day', min(created_at)) as Day from purchases;"###,
            &[r###"Total"###, r###" N"###, r###"Day"###][..],
        ),
        (
            "short_functions",
            r###"select coalesce(null, 1) as a, nullif(1, 2) as b, round(3.14, 1) as c;"###,
            &[][..],
        ),
        (
            "comments_literals",
            r###"-- Keep MiXeD comment
select 'select  FROM -- Keep' as "MiXeD Alias", 'it''s fine' as note /* inline MiXeD */;
-- trailing Keep"###,
            &[
                r###"-- Keep MiXeD comment"###,
                r###"'select  FROM -- Keep'"###,
                r###""MiXeD Alias""###,
                r###"'it''s fine'"###,
                r###"/* inline MiXeD */"###,
                r###"-- trailing Keep"###,
            ][..],
        ),
        (
            "line_comment_predicate",
            r###"select id from users where id = 1 -- this must not swallow AND
and country = 'SE';"###,
            &[r###"-- this must not swallow AND"###][..],
        ),
        (
            "nested_block_comment",
            r###"select 1 /* outer /* inner */ still outer */ + 2 as n;"###,
            &[r###"/* outer /* inner */ still outer */"###][..],
        ),
        (
            "dollar_strings",
            r###"select $$hello ' -- world
SELECT FROM$$ as txt, $tag$MiXeD ; ' " text$tag$ as tagged;"###,
            &[
                r###"$$hello ' -- world
SELECT FROM$$"###,
                r###"$tag$MiXeD ; ' " text$tag$"###,
            ][..],
        ),
        (
            "escaped_string",
            r###"select E'line\nnext\t\\text' as txt;"###,
            &[r###"E'line\nnext\t\\text'"###][..],
        ),
        (
            "unicode_identifiers",
            r###"select År, "Mätvärde" from (values (2026, 'räksmörgås')) as t(År, "Mätvärde");"###,
            &[r###"År"###, r###""Mätvärde""###, r###"'räksmörgås'"###][..],
        ),
        (
            "list_subscript",
            r###"select [10, 20, 30][2] as n, [1, 2, 3][1:2] as slice;"###,
            &[][..],
        ),
        (
            "list_comprehension",
            r###"select [x * 2 for x in [1, 2, 3] if x > 1] as xs;"###,
            &[][..],
        ),
        (
            "lambda",
            r###"select list_transform([1, 2, 3], lambda x: x + 1) as xs;"###,
            &[][..],
        ),
        (
            "arrow_lambda",
            r###"select list_transform([1, 2, 3], x -> x + 1) as xs;"###,
            &[][..],
        ),
        (
            "struct_map",
            r###"select {'MiXeD': 1, 'other': 2}.MiXeD as n, map(['a', 'b'], [1, 2])['a'] as a;"###,
            &[r###"'MiXeD'"###, r###"MiXeD"###][..],
        ),
        (
            "json_arrow",
            r###"select ('{"field":42}'::json ->> 'field')::integer as n;"###,
            &[r###"'{"field":42}'"###][..],
        ),
        (
            "integer_division",
            r###"select 5 // 2 as n, 2 ** 3 as power, 5 % 2 as rem;"###,
            &[][..],
        ),
        (
            "star_exclude_replace",
            r###"select * exclude (status) replace (amount * 2 as amount) from purchases order by user_id, created_at;"###,
            &[][..],
        ),
        (
            "columns_expression",
            r###"select columns('^(id|country)$') from users order by id;"###,
            &[r###"'^(id|country)$'"###][..],
        ),
        (
            "from_first_group_all",
            r###"from purchases select user_id, sum(amount) as total group by all order by user_id;"###,
            &[][..],
        ),
        (
            "prefix_alias",
            r###"select total: sum(amount), count: count(*) from purchases;"###,
            &[][..],
        ),
        (
            "recursive_cte",
            r###"with recursive nums(n) as (select 1 union all select n + 1 from nums where n < 4) select n from nums order by n;"###,
            &[][..],
        ),
        (
            "using_key",
            r###"with recursive state(id, step) using key (id) as (select 1, 0 union all select id, step + 1 from state where step < 3) select * from state;"###,
            &[][..],
        ),
        (
            "recurring_cte",
            r###"with recursive state(id, step) using key (id) as (select 1, 0 union all select s.id, r.step + 1 from state as s join recurring.state as r using (id) where r.step < 3) select * from state;"###,
            &[][..],
        ),
        (
            "union_by_name",
            r###"select 1 as a, 2 as b union all by name select 3 as b, 4 as a;"###,
            &[][..],
        ),
        (
            "named_arguments",
            r###"select struct_pack(a := 1, b := 'value') as s;"###,
            &[][..],
        ),
        (
            "pivot",
            r###"pivot (select country, id from users) on country using sum(id);"###,
            &[][..],
        ),
        (
            "interval_extract",
            r###"select date '2026-01-01' + interval '2 days' as day, extract(year from date '2026-01-01') as year;"###,
            &[][..],
        ),
        (
            "multiple_statements",
            r###"select 1 as first;
-- second query
select 2 as second;"###,
            &[r###"-- second query"###][..],
        ),
        (
            "long_literal",
            r###"select 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' as long_text;"###,
            &[
                r###"'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'"###,
            ][..],
        ),
        (
            "parameters",
            r###"select $name as "Name", $1::integer as value where $1 > 0;"###,
            &[r###"$name"###, r###"$1"###, r###""Name""###][..],
        ),
        (
            "incomplete_editor",
            r###"select id, from users where ("###,
            &[][..],
        ),
        (
            "dollar_plain",
            r###"select $$hello world$$ as txt;"###,
            &[r###"$$hello world$$"###][..],
        ),
        (
            "dollar_tagged",
            r###"select $tag$hello world$tag$ as txt;"###,
            &[r###"$tag$hello world$tag$"###][..],
        ),
        (
            "dollar_spaces",
            r###"select $$a  b$$ as txt;"###,
            &[r###"$$a  b$$"###][..],
        ),
        (
            "dollar_comment",
            r###"select $$a -- b$$ as txt;"###,
            &[r###"$$a -- b$$"###][..],
        ),
        (
            "compact_line_comment",
            r###"select 1 -- keep addition
 + 1 as n;"###,
            &[r###"-- keep addition"###][..],
        ),
        (
            "near_width_limit",
            r###"select 0 as col_0, 1 as col_1, 2 as col_2, 3 as col_3, 4 as col_4, 5 as col_5, 6 as col_6, 7 as col_7;"###,
            &[][..],
        ),
    ] {
        let options = FormatOptions::duckdb();
        let result = format(sql, &QueryParams::None, &options);
        assert_eq!(
            result,
            format(&result, &QueryParams::None, &options),
            "{name}"
        );
        for text in protected {
            assert!(
                result.contains(text),
                "{name}: missing {text:?} in {result}"
            );
        }
    }
}
