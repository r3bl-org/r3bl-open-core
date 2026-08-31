set -gx EMPTY_VAR '';
set -e EXISTING_VAR;
set -gx MULTILINE 'line 1
line 2
line 3';
set -gx QUOTE_DOUBLE 'said "hello"';
set -gx QUOTE_SINGLE 'don\'t fail';
set -gx WITH_BACKSLASH 'path\\to\\dir';
set -gx WITH_SEMICOLON 'foo; bar; baz';
