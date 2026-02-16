use super::db_handler::is_multi_statement;

#[test]
fn single_statement() {
    assert!(!is_multi_statement("SELECT 1"));
}

#[test]
fn single_with_trailing_semicolon() {
    assert!(!is_multi_statement("SELECT 1;"));
}

#[test]
fn single_with_trailing_whitespace() {
    assert!(!is_multi_statement("SELECT 1;  \n  "));
}

#[test]
fn two_statements() {
    assert!(is_multi_statement(
        "CREATE TABLE t(id INT); INSERT INTO t VALUES(1)"
    ));
}

#[test]
fn semicolon_in_single_quotes() {
    assert!(!is_multi_statement("INSERT INTO t VALUES ('a;b')"));
}

#[test]
fn semicolon_in_double_quotes() {
    assert!(!is_multi_statement("SELECT \"col;name\" FROM t"));
}

#[test]
fn line_comment_with_semicolon() {
    assert!(!is_multi_statement("-- drop; all\nSELECT 1"));
}

#[test]
fn block_comment_with_semicolon() {
    assert!(!is_multi_statement("SELECT * FROM t /* filter; todo */"));
}

#[test]
fn block_comment_between_statements() {
    assert!(is_multi_statement("SELECT 1; /* comment */ SELECT 2"));
}

#[test]
fn empty_between_semicolons() {
    assert!(is_multi_statement("SELECT 1;; SELECT 2"));
}

#[test]
fn only_comments() {
    assert!(!is_multi_statement("-- just a comment\n-- another one"));
}

#[test]
fn multi_with_line_comments() {
    assert!(is_multi_statement(
        "-- migration\nCREATE TABLE t(id INT);\n-- next\nINSERT INTO t VALUES(1);"
    ));
}
