# ziplookup

Search a file system tree and the ZIP files it contains for a file name matching a specific pattern.

## Usage

`ziplookup [--trace|--trace-some] STARTDIR SEARCHNAME`

Searches within `STARTDIR` and its subtree for a file named `SEARCHNAME`.

If `--trace` is given, outputs progress information for every file on `stderr`.

If `--trace-some` is given, periodically outputs progress information on `stderr`.
