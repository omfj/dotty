module.exports = grammar({
  name: "dotty",

  word: ($) => $.identifier,

  extras: ($) => [/\s/, $.comment],

  rules: {
    source_file: ($) => repeat($._statement),

    _statement: ($) =>
      choice(
        $.link_statement,
        $.do_statement,
        $.if_statement,
        $.print_statement,
        $.assignment,
      ),

    link_statement: ($) =>
      seq("link", field("source", $.string), "to", field("destination", $.string)),

    do_statement: ($) =>
      seq("do", field("arg1", $.string), optional(field("arg2", $.string))),

    print_statement: ($) => seq("print", field("value", $._atom)),

    if_statement: ($) =>
      seq(
        "if",
        field("condition", $._condition),
        field("body", $.block),
        optional($.else_clause),
      ),

    else_clause: ($) =>
      seq(
        "else",
        choice(field("body", $.block), field("alternative", $.if_statement)),
      ),

    assignment: ($) =>
      seq(field("name", $.identifier), "=", field("value", $.string)),

    block: ($) => seq("{", repeat($._statement), "}"),

    _condition: ($) =>
      choice(
        $.or_condition,
        $.and_condition,
        $.is_condition,
        $.is_not_condition,
        $.not_condition,
        $._atom,
      ),

    or_condition: ($) =>
      prec.left(
        1,
        seq(field("left", $._condition), "or", field("right", $._condition)),
      ),

    and_condition: ($) =>
      prec.left(
        2,
        seq(field("left", $._condition), "and", field("right", $._condition)),
      ),

    is_condition: ($) =>
      prec(
        1,
        seq(field("left", $._atom), "is", field("right", $._atom)),
      ),

    is_not_condition: ($) =>
      prec(
        2,
        seq(field("left", $._atom), "is", "not", field("right", $._atom)),
      ),

    not_condition: ($) => seq("not", field("operand", $._atom)),

    _atom: ($) =>
      choice(
        $.env_expr,
        $.test_expr,
        $.exists_expr,
        $.string,
        $.identifier,
      ),

    env_expr: ($) => seq("env", field("name", $.string)),

    test_expr: ($) => seq("test", field("command", $.string)),

    exists_expr: ($) => seq("exists", field("path", $.string)),

    string: ($) =>
      choice(
        seq('"', repeat(choice($.string_content, $.interpolation)), '"'),
        seq("'", repeat(choice($.string_single_content, $.interpolation)), "'"),
      ),

    string_content: (_) => /[^"$\\]+/,
    string_single_content: (_) => /[^'$\\]+/,

    interpolation: ($) => seq("$", $.identifier),

    identifier: (_) => /[a-zA-Z_][a-zA-Z0-9_\-]*/,

    comment: (_) => token(seq("#", /.*/)),
  },
});
