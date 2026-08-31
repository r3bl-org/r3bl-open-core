Remove-Item -Path 'env:EXISTING_VAR' -ErrorAction SilentlyContinue;
$env:QUOTE_SINGLE = 'don''t fail';
$env:WITH_BACKSLASH = 'path\to\dir';
$env:WITH_SEMICOLON = 'foo; bar; baz';
