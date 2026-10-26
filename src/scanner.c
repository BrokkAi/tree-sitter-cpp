#include "tree_sitter/alloc.h"
#include "tree_sitter/parser.h"

#include <assert.h>
#include <stdbool.h>
#include <string.h>
#include <wctype.h>

enum TokenType { RAW_STRING_DELIMITER, RAW_STRING_CONTENT, MACRO_ARGUMENTS };

/// The spec limits delimiters to 16 chars
#define MAX_DELIMITER_LENGTH 16

typedef struct {
    uint8_t delimiter_length;
    wchar_t delimiter[MAX_DELIMITER_LENGTH];
} Scanner;

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

static inline void reset(Scanner *scanner) {
    scanner->delimiter_length = 0;
    memset(scanner->delimiter, 0, sizeof scanner->delimiter);
}

/// Scan the raw string delimiter in R"delimiter(content)delimiter"
static bool scan_raw_string_delimiter(Scanner *scanner, TSLexer *lexer) {
    if (scanner->delimiter_length > 0) {
        // Closing delimiter: must exactly match the opening delimiter.
        // We already checked this when scanning content, but this is how we
        // know when to stop. We can't stop at ", because R"""hello""" is valid.
        for (int i = 0; i < scanner->delimiter_length; ++i) {
            if (lexer->lookahead != scanner->delimiter[i]) {
                return false;
            }
            advance(lexer);
        }
        reset(scanner);
        return true;
    }

    // Opening delimiter: record the d-char-sequence up to (.
    // d-char is any basic character except parens, backslashes, and spaces.
    for (;;) {
        if (scanner->delimiter_length >= MAX_DELIMITER_LENGTH || lexer->eof(lexer) || lexer->lookahead == '\\' ||
            iswspace(lexer->lookahead)) {
            return false;
        }
        if (lexer->lookahead == '(') {
            // Rather than create a token for an empty delimiter, we fail and
            // let the grammar fall back to a delimiter-less rule.
            return scanner->delimiter_length > 0;
        }
        scanner->delimiter[scanner->delimiter_length++] = lexer->lookahead;
        advance(lexer);
    }
}

/// Scan the raw string content in R"delimiter(content)delimiter"
static bool scan_raw_string_content(Scanner *scanner, TSLexer *lexer) {
    // The progress made through the delimiter since the last ')'.
    // The delimiter may not contain ')' so a single counter suffices.
    for (int delimiter_index = -1;;) {
        // If we hit EOF, consider the content to terminate there.
        // This forms an incomplete raw_string_literal, and models the code
        // well.
        if (lexer->eof(lexer)) {
            lexer->mark_end(lexer);
            return true;
        }

        if (delimiter_index >= 0) {
            if (delimiter_index == scanner->delimiter_length) {
                if (lexer->lookahead == '"') {
                    return true;
                }
                delimiter_index = -1;
            } else {
                if (lexer->lookahead == scanner->delimiter[delimiter_index]) {
                    delimiter_index += 1;
                } else {
                    delimiter_index = -1;
                }
            }
        }

        if (delimiter_index == -1 && lexer->lookahead == ')') {
            // The content doesn't include the )delimiter" part.
            // We must still scan through it, but exclude it from the token.
            lexer->mark_end(lexer);
            delimiter_index = 0;
        }

        advance(lexer);
    }
}

/// The longest word the macro-argument lookahead needs to classify.
#define MAX_WORD_LENGTH 32

/// How many following semicolon-free lines the macro-argument lookahead reads.
#define MAX_INVOCATION_CHAIN 16

static inline bool is_word_start(int32_t c) { return iswalpha(c) || c == '_' || c == '$'; }

static inline bool is_word_char(int32_t c) { return iswalnum(c) || c == '_' || c == '$'; }

static inline bool is_newline(int32_t c) { return c == '\n' || c == '\r'; }

/// Skip a comment whose leading '/' has been consumed. Returns false if the
/// '/' did not start a comment. Sets *newline when the comment ends a line.
static bool skip_comment(TSLexer *lexer, bool *newline) {
    if (lexer->lookahead == '/') {
        while (!lexer->eof(lexer) && !is_newline(lexer->lookahead)) {
            advance(lexer);
        }
        *newline = true;
        return true;
    }
    if (lexer->lookahead == '*') {
        advance(lexer);
        while (!lexer->eof(lexer)) {
            if (lexer->lookahead == '*') {
                advance(lexer);
                if (lexer->lookahead == '/') {
                    advance(lexer);
                    return true;
                }
            } else {
                if (is_newline(lexer->lookahead)) {
                    *newline = true;
                }
                advance(lexer);
            }
        }
        return true;
    }
    return false;
}

/// Skip whitespace and comments. Returns false on a lone '/'. Sets *newline
/// when a line ends on the way.
static bool skip_space(TSLexer *lexer, bool *newline) {
    for (;;) {
        if (is_newline(lexer->lookahead)) {
            *newline = true;
            advance(lexer);
        } else if (iswspace(lexer->lookahead)) {
            advance(lexer);
        } else if (lexer->lookahead == '/') {
            advance(lexer);
            if (!skip_comment(lexer, newline)) {
                return false;
            }
        } else {
            return true;
        }
    }
}

/// Consume a balanced parenthesized token sequence starting at '('. String
/// and character literals and comments may contain unbalanced parentheses.
/// A preprocessor directive inside the parentheses rejects the sequence.
static bool scan_balanced_parens(TSLexer *lexer) {
    if (lexer->lookahead != '(') {
        return false;
    }
    unsigned depth = 0;
    bool line_start = false;
    while (!lexer->eof(lexer)) {
        int32_t c = lexer->lookahead;
        if (is_newline(c)) {
            line_start = true;
            advance(lexer);
            continue;
        }
        if (iswspace(c)) {
            advance(lexer);
            continue;
        }
        if (c == '#' && line_start) {
            return false;
        }
        line_start = false;
        if (c == '"' || c == '\'') {
            advance(lexer);
            while (lexer->lookahead != c) {
                if (lexer->eof(lexer) || is_newline(lexer->lookahead)) {
                    return false;
                }
                if (lexer->lookahead == '\\') {
                    advance(lexer);
                }
                advance(lexer);
            }
            advance(lexer);
            continue;
        }
        if (c == '/') {
            bool newline = false;
            advance(lexer);
            skip_comment(lexer, &newline);
            line_start = newline;
            continue;
        }
        advance(lexer);
        if (c == '(') {
            depth++;
        } else if (c == ')') {
            depth--;
            if (depth == 0) {
                return true;
            }
        }
    }
    return false;
}

/// Read a word into buffer (truncated to MAX_WORD_LENGTH characters).
static unsigned scan_word(TSLexer *lexer, char *buffer) {
    unsigned length = 0;
    while (is_word_char(lexer->lookahead)) {
        if (length < MAX_WORD_LENGTH) {
            buffer[length] = lexer->lookahead < 128 ? (char)lexer->lookahead : '?';
        }
        length++;
        advance(lexer);
    }
    buffer[length < MAX_WORD_LENGTH ? length : MAX_WORD_LENGTH] = '\0';
    return length;
}

/// Keywords and builtin type names that can only begin a new top-level item,
/// never continue a declaration whose declarator is `NAME(...)`.
static const char *const ITEM_START_WORDS[] = {
    "_Bool",       "_Static_assert", "__extension__", "__inline", "__inline__",   "__thread",
    "_Thread_local", "auto",         "bool",          "char",     "char16_t",     "char32_t",
    "char8_t",     "charptr_t",      "class",         "concept",  "consteval",    "constexpr",
    "constinit",   "const",          "double",        "enum",     "export",       "extern",
    "float",       "import",         "inline",        "int",      "int16_t",      "int32_t",
    "int64_t",     "int8_t",         "intptr_t",      "long",     "max_align_t",  "module",
    "namespace",   "nullptr_t",      "ptrdiff_t",     "register", "short",        "signed",
    "size_t",      "ssize_t",        "static",        "static_assert", "struct",  "template",
    "thread_local", "typedef",       "uint16_t",      "uint32_t", "uint64_t",     "uint8_t",
    "uintptr_t",   "union",          "unsigned",      "using",    "void",         "volatile",
};

static bool is_item_start_word(const char *word) {
    for (size_t i = 0; i < sizeof ITEM_START_WORDS / sizeof ITEM_START_WORDS[0]; i++) {
        if (strcmp(word, ITEM_START_WORDS[i]) == 0) {
            return true;
        }
    }
    return false;
}

/// Scan the parenthesized arguments of a top-level macro invocation that has
/// no trailing semicolon, e.g. `LIST_TYPES(deps, struct Aggregate *, 32)`.
///
/// The token is only produced when the invocation ends its line and what
/// follows can only begin a new item: end of input, a preprocessor directive,
/// a closing brace, or a declaration keyword or builtin type, possibly after
/// lines that hold only a word or another such invocation. Anything else (`{`, `;`, `:`, `const`, a
/// declarator such as `name(void);` after a return-type macro, ...) leaves the
/// arguments to the ordinary grammar.
///
/// The '(' must follow the macro name directly. Error recovery can skip a
/// token between an identifier and a '(' (`__BEGIN_DECLS` `#if` `(...)`), and
/// such a skipped token always leaves whitespace before the '('.
static bool scan_macro_arguments(TSLexer *lexer) {
    if (!scan_balanced_parens(lexer)) {
        return false;
    }
    lexer->mark_end(lexer);

    bool newline = false;
    if (!skip_space(lexer, &newline)) {
        return false;
    }
    if (lexer->eof(lexer)) {
        return true;
    }
    if (!newline) {
        return false;
    }

    // Follow lines that hold only a word or another semicolon-free
    // invocation until one starts an item for certain.
    for (unsigned link = 0; link < MAX_INVOCATION_CHAIN; link++) {
        int32_t c = lexer->lookahead;
        if (c == '#' || c == '}') {
            return true;
        }
        if (!is_word_start(c)) {
            return false;
        }

        char word[MAX_WORD_LENGTH + 1];
        scan_word(lexer, word);
        if (is_item_start_word(word)) {
            return true;
        }

        // Any other word must end its line, either alone (an object-like
        // macro) or as another invocation. Otherwise it is the type or the
        // declarator of an ordinary declaration, e.g. `name(void);` after a
        // return-type macro.
        newline = false;
        if (!skip_space(lexer, &newline)) {
            return false;
        }
        if (!newline && !lexer->eof(lexer)) {
            if (lexer->lookahead != '(' || !scan_balanced_parens(lexer)) {
                return false;
            }
            if (!skip_space(lexer, &newline)) {
                return false;
            }
        }
        if (lexer->eof(lexer)) {
            return true;
        }
        if (!newline) {
            return false;
        }
    }
    return false;
}

void *tree_sitter_cpp_external_scanner_create() {
    Scanner *scanner = (Scanner *)ts_calloc(1, sizeof(Scanner));
    memset(scanner, 0, sizeof(Scanner));
    return scanner;
}

bool tree_sitter_cpp_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
    Scanner *scanner = (Scanner *)payload;

    if (valid_symbols[RAW_STRING_DELIMITER] && valid_symbols[RAW_STRING_CONTENT]) {
        // we're in error recovery
        return false;
    }

    // No skipping leading whitespace: raw-string grammar is space-sensitive.
    if (valid_symbols[RAW_STRING_DELIMITER]) {
        lexer->result_symbol = RAW_STRING_DELIMITER;
        return scan_raw_string_delimiter(scanner, lexer);
    }

    if (valid_symbols[RAW_STRING_CONTENT]) {
        lexer->result_symbol = RAW_STRING_CONTENT;
        return scan_raw_string_content(scanner, lexer);
    }

    if (valid_symbols[MACRO_ARGUMENTS]) {
        lexer->result_symbol = MACRO_ARGUMENTS;
        return scan_macro_arguments(lexer);
    }

    return false;
}

unsigned tree_sitter_cpp_external_scanner_serialize(void *payload, char *buffer) {
    static_assert(MAX_DELIMITER_LENGTH * sizeof(wchar_t) < TREE_SITTER_SERIALIZATION_BUFFER_SIZE,
                  "Serialized delimiter is too long!");

    Scanner *scanner = (Scanner *)payload;
    size_t size = scanner->delimiter_length * sizeof(wchar_t);
    memcpy(buffer, scanner->delimiter, size);
    return (unsigned)size;
}

void tree_sitter_cpp_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    assert(length % sizeof(wchar_t) == 0 && "Can't decode serialized delimiter!");

    Scanner *scanner = (Scanner *)payload;
    scanner->delimiter_length = length / sizeof(wchar_t);
    if (length > 0) {
        memcpy(&scanner->delimiter[0], buffer, length);
    }
}

void tree_sitter_cpp_external_scanner_destroy(void *payload) {
    Scanner *scanner = (Scanner *)payload;
    ts_free(scanner);
}
