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
    "EXPECTATION": "A AcsText value like category-class \"WAT\" must raise a diagnostic if \"WAT\" is not a key in category-class.txt.",
    "NEW INSTRUCTION": "WHEN validating IsValidCategoryClass and value not in mapping THEN emit an error diagnostic at the value range"
}

[2026-04-07 14:58] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Category class validation",
    "EXPECTATION": "A AcsText value like category-class \"WAT\" must raise a diagnostic if \"WAT\" is not a key in category-class.txt.",
    "NEW INSTRUCTION": "WHEN validating IsValidCategoryClass and value not in mapping THEN emit an error diagnostic at the value range"
}

[2026-04-07 15:25] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "AcsText validation keys",
    "EXPECTATION": "TagArray, subpossibilities, and array-element are validation rule identifiers, not mandatory AcsText keys.",
    "NEW INSTRUCTION": "WHEN validating AcsText object keys THEN do not require TagArray, subpossibilities, or array-element"
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
    "NEW INSTRUCTION": "WHEN matching 'TagArray' in AcsText metadata THEN match case-insensitively"
}

[2026-04-07 22:25] - Updated by Junie
{
    "TYPE": "preference",
    "CATEGORY": "Wiki kind source",
    "EXPECTATION": "Use the Asset_KIND category page to determine additional kind names and their correct Wiki casing.",
    "NEW INSTRUCTION": "WHEN generating or validating kind Wiki links THEN consult Category:Asset_KIND for official casing"
}

[2026-04-07 22:37] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Category-class completions",
    "EXPECTATION": "Completion on a category-class value should return suggestions sourced from the validation file.",
    "NEW INSTRUCTION": "WHEN completion requested at category-class value THEN return options from category-class.txt"
}

[2026-04-07 22:43] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "AcsText completions empty",
    "EXPECTATION": "Completion in AcsText files should return context-appropriate suggestions instead of an empty list.",
    "NEW INSTRUCTION": "WHEN handling textDocument/completion for AcsText THEN initialize validators and AST before computing items"
}

[2026-04-08 12:56] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "AcsText kind hover/completion",
    "EXPECTATION": "At the kind \"lib\" value in a AcsText file, the user expects value completions and a hover to appear.",
    "NEW INSTRUCTION": "WHEN cursor is inside a quoted AcsText value for a known key THEN provide value completions and hover from its validator"
}

[2026-04-08 12:58] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "AcsText kind hover/completion",
    "EXPECTATION": "At the kind \"lib\" value in a AcsText file, completions and a hover should appear.",
    "NEW INSTRUCTION": "WHEN cursor is inside the quoted value of key kind THEN provide value completions and a hover from its validator"
}

[2026-04-08 13:02] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Kind value correction",
    "EXPECTATION": "An invalid kind value like \"lib\" should trigger a completion suggesting \"library\" to correct it.",
    "NEW INSTRUCTION": "WHEN cursor inside quoted kind value and text invalid THEN suggest closest valid kind values including library"
}

[2026-04-08 14:10] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Kind value completion",
    "EXPECTATION": "At the kind value position, completions should list valid kind values (e.g., \"library\"), not the key name.",
    "NEW INSTRUCTION": "WHEN cursor is inside the quoted value of key kind THEN suggest valid kind values not keys"
}

[2026-04-08 14:19] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Kind value fuzzy completion",
    "EXPECTATION": "At kind \"l\" the user expects fuzzy-matched kind value suggestions instead of an empty list.",
    "NEW INSTRUCTION": "WHEN inside quoted kind value with prefix THEN suggest fuzzy-matched valid kind values"
}

[2026-04-08 14:29] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "AcsText kind completion context",
    "EXPECTATION": "Inside the quoted value of key kind, value completions should appear (e.g., library), not an empty list.",
    "NEW INSTRUCTION": "WHEN cursor is inside the value range for key kind THEN compute value completions from its validator and skip top-level key suggestions"
}

[2026-04-08 15:43] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Contextual key suggestions",
    "EXPECTATION": "Only keys that are valid for the current AcsText container and position should be suggested.",
    "NEW INSTRUCTION": "WHEN computing key completions in a AcsText container THEN suggest only keys allowed by its active validator"
}

[2026-04-08 16:39] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Category-region validation",
    "EXPECTATION": "A AcsText value like category-region \"AC\" must raise a diagnostic if \"AC\" is not a key in category-region.txt.",
    "NEW INSTRUCTION": "WHEN validating category-region and value not in mapping THEN emit an error diagnostic at the value range"
}

[2026-04-08 16:52] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Context-aware value completions",
    "EXPECTATION": "Value completions should be offered not only for kind but also for category-class, category-region, category-era, and other validated enum or semicolon-separated keys, with awareness of whether the cursor is on a key or a value.",
    "NEW INSTRUCTION": "WHEN cursor inside quoted value or semicolon item for validated key THEN suggest validator values with case-insensitive fuzzy filtering and correct insertion"
}

[2026-04-08 16:54] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Context-aware value completions",
    "EXPECTATION": "Completions should appear for category-class, category-region, category-era, and other validated enum or semicolon-separated keys, not just kind, with awareness of whether the cursor is on a key or a value.",
    "NEW INSTRUCTION": "WHEN cursor inside a value of a validated enum/array key THEN suggest validator values with case-insensitive fuzzy filtering and correct insertion"
}

[2026-04-08 19:58] - Updated by Junie
{
    "TYPE": "correction",
    "CATEGORY": "Value completions missing",
    "EXPECTATION": "Completions should appear for values (not just keys) based on validators when the cursor is inside a value.",
    "NEW INSTRUCTION": "WHEN cursor is inside a validated value range THEN suggest validator-backed value completions"
}

