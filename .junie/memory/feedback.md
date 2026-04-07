[2026-04-06 10:18] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges bug",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) should produce correct folding ranges when traversing the AST.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 16:29] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges bug",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) should produce correct folding ranges when traversing the AST.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 16:52] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges bug",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) should produce correct folding ranges when traversing the AST.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 17:02] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges bug",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) should produce correct folding ranges when traversing the AST.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 18:07] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges bug",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) should produce correct folding ranges when traversing the AST.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 18:20] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges bug",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) should produce correct folding ranges when traversing the AST.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 18:57] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 19:01] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 19:43] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 20:10] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 21:46] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges traversal",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 22:18] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges traversal",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 22:57] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges traversal",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 23:09] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges traversal",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-06 23:45] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Group range span bug",
    "EXPECTATION": "Group comment ranges should span from the first line's start to the last line's end.",
    "NEW INSTRUCTION": "WHEN aggregating contiguous line comments into GroupComment THEN set range first.start to last.end"
}

[2026-04-07 00:08] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Hover range",
    "EXPECTATION": "The hover response should include the source range of the hovered symbol/definition.",
    "NEW INSTRUCTION": "WHEN constructing a Hover result THEN set its range to the symbol's definition range"
}

[2026-04-07 02:13] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Hover range",
    "EXPECTATION": "The hover response should include the source range of the hovered symbol/definition.",
    "NEW INSTRUCTION": "WHEN constructing a Hover result THEN set its range to the symbol's definition range"
}

[2026-04-07 02:21] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Extension activation/loading",
    "EXPECTATION": "The VS Code extension should activate for GS files and start the LSP so features are visible.",
    "NEW INSTRUCTION": "WHEN configuring activationEvents THEN use onLanguage with the correct GS language id"
}

[2026-04-07 08:46] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges traversal",
    "EXPECTATION": "Statement blocks (if/else, loops, switch cases, wait/on, and standalone blocks) must produce correct folding ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a statement with a Block body THEN add its range and recurse into it"
}

[2026-04-07 11:09] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Folding ranges - class body",
    "EXPECTATION": "class_body blocks should produce foldable ranges during AST traversal.",
    "NEW INSTRUCTION": "WHEN visiting a class_body node with a Block THEN add its range and recurse"
}

[2026-04-07 14:57] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Category class validation",
    "EXPECTATION": "A Soup value like category-class \"WAT\" must raise a diagnostic if \"WAT\" is not a key in category-class.txt.",
    "NEW INSTRUCTION": "WHEN validating IsValidCategoryClass and value not in mapping THEN emit an error diagnostic at the value range"
}

[2026-04-07 14:58] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Category class validation",
    "EXPECTATION": "A Soup value like category-class \"WAT\" must raise a diagnostic if \"WAT\" is not a key in category-class.txt.",
    "NEW INSTRUCTION": "WHEN validating IsValidCategoryClass and value not in mapping THEN emit an error diagnostic at the value range"
}

[2026-04-07 15:25] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Soup validation keys",
    "EXPECTATION": "TagArray, subpossibilities, and array-element are validation rule identifiers, not mandatory Soup keys.",
    "NEW INSTRUCTION": "WHEN validating Soup object keys THEN do not require TagArray, subpossibilities, or array-element"
}

[2026-04-07 15:49] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Compulsory requirement bug",
    "EXPECTATION": "Only keys with compulsory >= 1 are required, including at top level; 'author' is optional.",
    "NEW INSTRUCTION": "WHEN checking for missing required keys THEN require only rules with compulsory >= 1"
}

[2026-04-07 18:20] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "TagArray casing",
    "EXPECTATION": "Both 'TagArray' and 'tagarray' should be recognized as the same metadata identifier.",
    "NEW INSTRUCTION": "WHEN matching 'TagArray' in Soup metadata THEN match case-insensitively"
}

