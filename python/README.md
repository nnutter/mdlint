# mdlint

[![CI](https://github.com/nnutter/mdlint/actions/workflows/ci.yml/badge.svg)](https://github.com/nnutter/mdlint/actions/workflows/ci.yml)

An opinionated Markdown formatter and linter, written in Rust.

What [ruff](https://github.com/astral-sh/ruff/) did for Python and [gofmt](https://pkg.go.dev/cmd/gofmt) did for Go,
`mdlint` aims to do for Markdown: enforce a single, consistent canonical style so that style debates disappear and diffs
stay meaningful.
As AI coding agents increasingly read and write Markdown, well-structured files matter more than ever.
Run `mdlint format` and stop thinking about it.

**Project Status**: Active development, but no one's top priority.

## Features

- **Formatter first**: `mdlint format` rewrites files to a canonical style — no configuration required
- **Linter second**: `mdlint check` reports violations; fixable rules are auto-corrected by `mdlint format` or
  `mdlint check --fix`
- **Fast**: written in Rust for performance
- **Portable**: single, small, 0-dependency binary (Linux x86_64/ARM64, macOS Intel/Apple Silicon, Windows)
- **Git-aware**: respects `.gitignore` files by default
- **Git-friendly style**: stable list markers, compact tables, preserved references, and conservative sentence breaks keep content edits local

## Quickstart

Build this fork's `opinionated` branch with [Rust](https://rustup.rs/):

```shell
git clone --branch opinionated https://github.com/nnutter/mdlint.git
cd mdlint
cargo install --path .
mdlint format
mdlint check
```

## Installation

Install from a checkout of this fork with `cargo install --path .`, as shown above.
When fork releases are available, download platform binaries from the [fork's releases page](https://github.com/nnutter/mdlint/releases).

**Distribution notice:** Packages named `markdownlint-rs` on crates.io, npm, and PyPI are upstream distributions, not builds of this fork.
This fork does not publish packages or container images to those registries.
The executable name remains `mdlint`.

To use Docker, build an image from your fork checkout instead of pulling an upstream image:

```shell
docker build -t mdlint-local .
docker run --rm -v "$PWD:/workspace" -w /workspace mdlint-local check
```

### pre-commit framework

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/nnutter/mdlint
    # choose a fork release tag or commit
    rev: v0.3.24
    hooks:
      - id: mdlint-format
      - id: mdlint-check
```

Inherited upstream tags do not include this fork's unreleased changes.
To use those changes, replace `rev` with a commit from the `opinionated` branch.

Or use additional arguments, e.g. to disable auto-fix:

```yaml
    hooks:
      - id: mdlint-format
        args: [--check]
      - id: mdlint-check
        args: [--no-fix]
```

## Usage

### mdlint check

Lint Markdown files and report issues.

```text
Usage: mdlint check [OPTIONS] [FILES]...

Arguments:
  [FILES]...                       Files or directories to check (defaults to current directory)

Options:
      --fix                        Apply auto-fixes where possible
      --no-fix                     Disable auto-fix even if enabled in config
      --output-format <FORMAT>     Output format [default: default] [possible values: default, json]
      --select <RULE_CODE>,...     Enable only the specified rules (comma-separated, or ALL)
      --ignore <RULE_CODE>,...     Disable the specified rules (comma-separated)
      --exclude <PATH>             Exclude files or directories from analysis
      --no-respect-ignore          Do not respect .gitignore files
      --parallel                   Lint files in parallel (experimental)
  -h, --help                       Print help

Global options:
      --config <CONFIG>            Path to TOML configuration file
      --no-config                  Ignore all configuration files
  -v, --verbose                    Enable verbose logging
  -q, --quiet                      Print diagnostics only
  -s, --silent                     Disable all logging (exit code still reflects result)
      --color <COLOR>              Control colors in output [default: auto] [possible values: auto, always, never]
```

### mdlint format

Format Markdown files with opinionated style.

```text
Usage: mdlint format [OPTIONS] [FILES]...

Arguments:
  [FILES]...                       Files or directories to format (defaults to current directory)

Options:
      --check                      Check formatting only; do not modify files (exits 1 if any file would change)
      --exclude <PATH>             Exclude files or directories
      --no-respect-ignore          Do not respect .gitignore files
  -h, --help                       Print help
```

### Formatter style for useful diffs

Formatting favors small, understandable Git diffs without changing Markdown meaning.

- **Prose:** Add conservative sentence-oriented soft breaks, without a column limit or width-based reflow.
  Preserve existing clause breaks and hard line breaks; leave ambiguous boundaries alone.
- **Ordered lists:** Keep the first number, including an intentional non-1 start, and use `1.` for later items.
  Inserting an item must not renumber the remaining list.
- **References:** Preserve inline, full-reference, collapsed-reference, and shortcut links and images.
  Keep descriptive labels and definition order; never generate sequential labels or convert every link to a reference.
- **Tables:** Use compact rows without column-width padding, while preserving alignment semantics.
  Widening one cell must not resize unrelated rows.
- **Code:** Preserve content whitespace, tabs, and repeated blank lines.
  Choose fences that cannot terminate on the code itself, including tilde fences when the info string contains backticks.
- **Emphasis:** Prefer asterisks, but retain underscores when needed to keep adjacent spans separate or preserve reference identity.

See [FORMAT_SPEC.md](https://github.com/nnutter/mdlint/blob/main/FORMAT_SPEC.md) for the complete style contract and correctness exceptions.

### mdlint migrate

Migrate a configuration from another Markdown tool to `mdlint.toml`.
Currently supported sources (`--from`):
`markdownlint-cli2` (default).

```text
Usage: mdlint migrate [OPTIONS] [INPUT]

Arguments:
  [INPUT]                          Path to the configuration to migrate (auto-detected if omitted)

Options:
      --from <SOURCE>               Tool to migrate the configuration from [default: markdownlint-cli2]
      --output <PATH>               Output path for the generated config [default: mdlint.toml]
      --force                       Overwrite the output file if it already exists
      --dry-run                     Print the generated config without writing it
  -h, --help                        Print help
```

#### markdownlint-cli2

Supports `.markdownlint-cli2.{json,jsonc,yaml,yml}` and standalone `.markdownlint.{json,jsonc,yaml,yml}` rule
configs, and falls back to the `"markdownlint-cli2"` field in `package.json` if no dedicated config file is found.
Rule names and their common aliases (e.g. `line-length` for `MD013`) are both recognized.
The `gitignore`,
`noInlineConfig`, and `frontMatterPattern` cli2 options map onto mdlint's equivalent `gitignore`, `no_inline_config`,
and `front_matter` settings. `.cjs`/`.mjs` configs are evaluated with a Node.js runtime when one is found on `PATH`
(the same thing `markdownlint-cli2` itself would do when loading them), correctly resolving `require()`, spread
syntax, and computed values.
If Node isn't available, mdlint falls back to a best-effort text scrape and warns that
dynamic values may not have been resolved; if a config can't be parsed either way, migration fails with a message
asking you to export it with `console.log(JSON.stringify(config))` and migrate the resulting JSON file instead.
Rules with no mdlint implementation, and cli2-specific fields with no mdlint equivalent (`globs`, `customRules`,
`outputFormatters`), are skipped with a warning rather than failing the migration.

### Examples

```bash
# check all Markdown files and apply auto-fixes
mdlint check --fix

# check specific files
mdlint check README.md docs/

# check with JSON output (for CI integrations)
mdlint check --output-format json

# enable only specific rules
mdlint check --select MD001,MD022

# disable specific default-enabled rules for this run
mdlint check --ignore MD025,MD040

# format all files
mdlint format

# verify formatting without modifying files (for CI)
mdlint format --check

# format specific files
mdlint format README.md docs/

# use a custom config file
mdlint check --config path/to/mdlint.toml

# ignore all config files
mdlint check --no-config

# migrate a markdownlint-cli2 config to mdlint.toml (auto-detected)
mdlint migrate

# migrate a specific config file to a custom output path
mdlint migrate --from markdownlint-cli2 .markdownlint-cli2.jsonc --output mdlint.toml
```

## Configuration

mdlint uses TOML configuration files, discovered by searching upward from the current directory.
The tool searches for
these files in order (first found wins per directory level), walking up from the current directory:

1. `mdlint.toml`
1. `.mdlint.toml`

Planned: `package.json` and `pyproject.toml` support.

### Configuration hierarchy

Configs are discovered by walking up the directory tree.
Scalar values from closer configs override those farther away;
arrays are extended.
Priority order (highest to lowest):

1. `--config` flag on the CLI
1. `mdlint.toml` / `.mdlint.toml` in the current directory
1. Config files in parent directories (walking up to the filesystem root)
1. Built-in defaults

### Default lint profile

The default profile avoids document policies that cause unnecessary edits or reject valid Markdown fragments.
MD013 (line length), MD026 (heading punctuation), MD033 (HTML), MD041 (required top-level title), and MD043 (heading templates) are disabled by default.
MD024 checks duplicate headings only within the same parent section.
MD009, MD010, and MD012 leave code whitespace unchanged by default.
MD060 allows independent table and column alignments.

Opt into optional rules with an explicit rule section or `--select MD013`, for example.
`--select ALL` opts into all rules while retaining explicit rule settings, including explicit disables.
Formatter style is not configurable; lint rule overrides remain available for project-specific checks.

### Global options

| Option | Default | Description |
| --- | --- | --- |
| `default_enabled` | `true` | Enable the default lint profile; explicit rule settings override it. Use `false` to enable only configured rules. |
| `gitignore` | `true` | Respect `.gitignore` files when discovering Markdown files. |
| `no_inline_config` | `false` | Ignore all `<!-- mdlint-disable -->` comments. |
| `fix` | `true` | `mdlint check` automatically applies all fixable violations; equivalent to passing `--fix` on the CLI. |
| `front_matter` | auto | Front matter delimiter. Auto-detects `---` (YAML) and `+++` (TOML). Set to `"---"` to accept YAML only. |
| `exclude` | `[]` | Paths/glob patterns excluded from discovery; merged with any `--exclude` CLI flags. |
| `custom_rules` | `[]` | Paths to external rule modules (future feature). |

### Rule configuration

Each rule is configured in its own `[rules.MDxxx]` section.
Providing parameters enables the rule unless that section also sets `enabled = false`.
Use `enabled = false` to disable a rule regardless of the profile.
Rule parameters do not configure the formatter.

```toml
# disable a default-enabled rule
[rules.MD040]
enabled = false

# configure parameters (also enables the rule)
[rules.MD013]
line_length = 100
code_blocks = false

# combine enabled flag with parameters
[rules.MD044]
enabled = true
names = ["JavaScript", "TypeScript", "GitHub"]
```

See [`mdlint.default.toml`](https://github.com/nnutter/mdlint/blob/main/mdlint.default.toml) for every option with its default value.

### Inline configuration

Rules can be suppressed for specific lines using HTML comments.
These directives suppress already-enabled rules; they do not enable optional policies.

```markdown
<!-- mdlint-disable-next-line MD013 -->
This line may be longer than the configured limit.

<!-- mdlint-disable MD033 -->
<div>Raw HTML block that needs to stay as-is</div>
<!-- mdlint-enable MD033 -->
```

| Comment | Effect |
| --- | --- |
| `<!-- mdlint-disable MD001 -->` | Disable rule from this line onward |
| `<!-- mdlint-enable MD001 -->` | Re-enable rule from this line onward |
| `<!-- mdlint-disable-next-line MD001 -->` | Disable rule for the next line only |
| `<!-- mdlint-disable -->` | Disable all rules from this line onward |
| `<!-- mdlint-enable -->` | Re-enable all rules |

Multiple rules: `<!-- mdlint-disable MD001 MD013 -->` — space-separate rule codes.
Set `no_inline_config = true`
in `mdlint.toml` to ignore all inline comments project-wide.

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | Success — no lint violations (or files are already formatted with `format --check`) |
| `1` | Lint violations found (or files need formatting with `format --check`) |
| `2` | Runtime error (invalid config, file not found, etc.) |

## Rules

Rules marked ✓ in the **Fix** column are auto-corrected by `mdlint check --fix` and `mdlint format`.
Rules without ✓
are reported by `mdlint check` only and require manual correction. **Default** shows mdlint's configured default for
the rule's key parameter(s); **markdownlint** shows the
[original markdownlint](https://github.com/DavidAnson/markdownlint/blob/main/doc/Rules.md) default where it differs from
mdlint's.
`off` marks optional policies disabled by default; their parameter defaults are documented in `mdlint.default.toml`.
`—` means the rule has no configurable parameters.

| Rule | Fix | Default | markdownlint | Description | Notes |
| --- | --- | --- | --- | --- | --- |
| [MD001](https://github.com/DavidAnson/markdownlint/blob/main/doc/md001.md) |  | — | — | Heading levels should only increment by one level at a time | Catches accidental heading skips (e.g. h1 → h3 without h2) |
| [MD003](https://github.com/DavidAnson/markdownlint/blob/main/doc/md003.md) |  | `atx` | `consistent` | Heading style should be consistent throughout the document | Config: `style` — `atx` (`# Heading`), `setext`, `atx_closed`, `consistent` |
| [MD004](https://github.com/DavidAnson/markdownlint/blob/main/doc/md004.md) | ✓ | `dash` | `consistent` | Unordered list style should be consistent | Config: `style` — `dash`, `asterisk`, `plus`, `consistent` |
| [MD005](https://github.com/DavidAnson/markdownlint/blob/main/doc/md005.md) |  | — | — | Inconsistent indentation for list items at the same level | Catches copy-paste errors where sibling items have different indentation |
| [MD007](https://github.com/DavidAnson/markdownlint/blob/main/doc/md007.md) |  | `indent: 2` |  | Unordered list indentation | Config: `indent` — spaces under unordered parents; ordered parents use their marker width |
| [MD009](https://github.com/DavidAnson/markdownlint/blob/main/doc/md009.md) | ✓ | `br_spaces: 2, code_blocks: false` |  | Trailing spaces | Preserve code whitespace by default. Two trailing spaces mean a hard line break; format converts them to `\` syntax. Config: `br_spaces`, `strict`, `code_blocks` |
| [MD010](https://github.com/DavidAnson/markdownlint/blob/main/doc/md010.md) | ✓ | `code_blocks: false` | `code_blocks: true` | Hard tabs | Check prose tabs without changing code blocks by default. Config: `code_blocks` |
| [MD011](https://github.com/DavidAnson/markdownlint/blob/main/doc/md011.md) |  | — | — | Reversed link syntax | Catches the common typo of swapped parentheses and brackets; should always be enabled |
| [MD012](https://github.com/DavidAnson/markdownlint/blob/main/doc/md012.md) | ✓ | `maximum: 1` |  | Multiple consecutive blank lines | Config: `maximum` — max consecutive blank lines allowed |
| [MD013](https://github.com/DavidAnson/markdownlint/blob/main/doc/md013.md) |  | `off` | `line: 80` | Line length | Opt-in width limits; no width-based prose reflow. Config: `line_length`, `heading_line_length`, `code_blocks`, `tables`, `headings` |
| [MD014](https://github.com/DavidAnson/markdownlint/blob/main/doc/md014.md) | ✓ | — | — | Dollar signs used before commands without showing output | `$`-prefixed shell commands cannot be copy-pasted; omit the `$` prompt |
| [MD018](https://github.com/DavidAnson/markdownlint/blob/main/doc/md018.md) | ✓ | — | — | No space after hash on atx style heading | `#Title` renders inconsistently; format inserts the required space |
| [MD019](https://github.com/DavidAnson/markdownlint/blob/main/doc/md019.md) | ✓ | — | — | Multiple spaces after hash on atx style heading | `#  Title` → `# Title`; format normalises to one space |
| [MD020](https://github.com/DavidAnson/markdownlint/blob/main/doc/md020.md) | ✓ | — | — | No space inside hashes on closed atx style heading | Only relevant if using `#Title#` style headings |
| [MD021](https://github.com/DavidAnson/markdownlint/blob/main/doc/md021.md) | ✓ | — | — | Multiple spaces inside hashes on closed atx style heading | Only relevant if using `#Title#` style headings |
| [MD022](https://github.com/DavidAnson/markdownlint/blob/main/doc/md022.md) | ✓ | — | — | Headings should be surrounded by blank lines | Required by many renderers for correct parsing; format inserts blank lines |
| [MD023](https://github.com/DavidAnson/markdownlint/blob/main/doc/md023.md) | ✓ | — | — | Headings must start at the beginning of the line | Indented headings are treated as code or paragraphs in CommonMark |
| [MD024](https://github.com/DavidAnson/markdownlint/blob/main/doc/md024.md) |  | `siblings_only: true` | `siblings_only: false` | Multiple headings with the same content | Allow repeated headings in different sections. Config: `siblings_only` |
| [MD025](https://github.com/DavidAnson/markdownlint/blob/main/doc/md025.md) |  | — | — | Multiple top-level headings in the same document | Disable for document fragments that intentionally lack a single top-level title |
| [MD026](https://github.com/DavidAnson/markdownlint/blob/main/doc/md026.md) |  | `off` | `".,;:!。，；：！"` | Trailing punctuation in heading | Opt-in punctuation policy; question headings are allowed by default. Config: `punctuation` |
| [MD027](https://github.com/DavidAnson/markdownlint/blob/main/doc/md027.md) | ✓ | — | — | Multiple spaces after blockquote symbol | `>  text` → `> text`; format normalises |
| [MD028](https://github.com/DavidAnson/markdownlint/blob/main/doc/md028.md) |  | — | — | Blank line inside blockquote | Blank lines split blockquotes into separate elements in CommonMark; may be intentional |
| [MD029](https://github.com/DavidAnson/markdownlint/blob/main/doc/md029.md) | ✓ | `one` | `one_or_ordered` | Ordered list item prefix | Preserve the first number and use `1.` for subsequent items. Config: `style` — `ordered` (1. 2. 3.), `one` (first number, then 1s), `one_or_ordered` |
| [MD030](https://github.com/DavidAnson/markdownlint/blob/main/doc/md030.md) | ✓ | `all: 1` |  | Spaces after list markers | Config: `ul_single`, `ul_multi`, `ol_single`, `ol_multi` — spaces after marker per context |
| [MD031](https://github.com/DavidAnson/markdownlint/blob/main/doc/md031.md) | ✓ | — | — | Fenced code blocks should be surrounded by blank lines | Some renderers require blank lines around fences to parse correctly |
| [MD032](https://github.com/DavidAnson/markdownlint/blob/main/doc/md032.md) |  | — | — | Lists should be surrounded by blank lines | Consistent blank lines around lists improve rendering across processors |
| [MD033](https://github.com/DavidAnson/markdownlint/blob/main/doc/md033.md) |  | `off` | `allowed_elements: []` | Inline HTML | HTML is allowed by default; restrictions are opt-in. Config: `allowed_elements` — add e.g. `["details", "summary"]` |
| [MD034](https://github.com/DavidAnson/markdownlint/blob/main/doc/md034.md) |  | — | — | Bare URL used | Plain URLs don't render as links in all Markdown processors; use `[text](url)` |
| [MD035](https://github.com/DavidAnson/markdownlint/blob/main/doc/md035.md) | ✓ | `---` | `consistent` | Horizontal rule style | Config: `style` — `---`, `***`, `___`, `consistent` |
| [MD036](https://github.com/DavidAnson/markdownlint/blob/main/doc/md036.md) |  | `".,;:!?。，；：！？"` |  | Emphasis used instead of a heading | Bold/italic-only lines won't appear in a table of contents. Config: `punctuation` |
| [MD037](https://github.com/DavidAnson/markdownlint/blob/main/doc/md037.md) |  | — | — | Spaces inside emphasis markers | `* text *` and `** text **` do not render as emphasis in CommonMark |
| [MD038](https://github.com/DavidAnson/markdownlint/blob/main/doc/md038.md) |  | — | — | Spaces inside code span elements | `` ` text ` `` is technically valid but inconsistent with expected style |
| [MD039](https://github.com/DavidAnson/markdownlint/blob/main/doc/md039.md) |  | — | — | Spaces inside link text | `[ text ]` is valid but inconsistent |
| [MD040](https://github.com/DavidAnson/markdownlint/blob/main/doc/md040.md) |  | — | — | Fenced code blocks should have a language specified | Language tags enable syntax highlighting. Config: `allowed_languages` |
| [MD041](https://github.com/DavidAnson/markdownlint/blob/main/doc/md041.md) |  | `off` | `level: 1` | First line in file should be a top-level heading | Opt in to require a title; fragments and badge-first documents are allowed by default |
| [MD042](https://github.com/DavidAnson/markdownlint/blob/main/doc/md042.md) |  | — | — | No empty links | `[text]()` is almost always a mistake |
| [MD043](https://github.com/DavidAnson/markdownlint/blob/main/doc/md043.md) |  | `off` | — | Required heading structure | Opt-in heading templates. Config: `headings` |
| [MD044](https://github.com/DavidAnson/markdownlint/blob/main/doc/md044.md) |  | `names: []` |  | Proper names should have the correct capitalization | Requires configuration to be useful. Config: `names`, `code_blocks` |
| [MD045](https://github.com/DavidAnson/markdownlint/blob/main/doc/md045.md) |  | — | — | Images should have alternate text (alt text) | Alt text is required for accessibility; screen readers depend on it |
| [MD046](https://github.com/DavidAnson/markdownlint/blob/main/doc/md046.md) |  | `fenced` | `consistent` | Code block style | Config: `style` — `fenced`, `indented`, `consistent` |
| [MD047](https://github.com/DavidAnson/markdownlint/blob/main/doc/md047.md) | ✓ | — | — | Files should end with a single newline character | POSIX standard; prevents "no newline at end of file" noise in git diffs |
| [MD048](https://github.com/DavidAnson/markdownlint/blob/main/doc/md048.md) |  | `backtick` | `consistent` | Code fence style | Safe tilde fences are accepted when the info string contains backticks. Config: `style` — `backtick`, `tilde`, `consistent` |
| [MD049](https://github.com/DavidAnson/markdownlint/blob/main/doc/md049.md) | ✓ | `asterisk` | `consistent` | Emphasis style should be consistent | Accept underscores needed for adjacent spans or reference identity. Config: `style` — `asterisk`, `underscore`, `consistent` |
| [MD050](https://github.com/DavidAnson/markdownlint/blob/main/doc/md050.md) | ✓ | `asterisk` | `consistent` | Strong style should be consistent | Accept underscores needed for adjacent spans or reference identity. Config: `style` — `asterisk`, `underscore`, `consistent` |
| [MD051](https://github.com/DavidAnson/markdownlint/blob/main/doc/md051.md) |  | — | — | Link fragments should be valid | Broken `#anchor` links are invisible to parsers but silently break in-page navigation |
| [MD052](https://github.com/DavidAnson/markdownlint/blob/main/doc/md052.md) |  | — | — | Reference links and images should use a label that is defined | Undefined reference links silently render as plain text instead of a link |
| [MD053](https://github.com/DavidAnson/markdownlint/blob/main/doc/md053.md) |  | — | — | Link and image reference definitions should be needed | Cleans up leftover link definitions after references are removed |
| [MD054](https://github.com/DavidAnson/markdownlint/blob/main/doc/md054.md) |  | — | — | Link and image style | Enforces consistent use of inline vs reference link syntax |
| [MD055](https://github.com/DavidAnson/markdownlint/blob/main/doc/md055.md) |  | `leading_and_trailing` | `consistent` | Table pipe style | Config: `style` — `leading_and_trailing`, `leading_only`, `trailing_only`, `no_leading_or_trailing`, `consistent` |
| [MD056](https://github.com/DavidAnson/markdownlint/blob/main/doc/md056.md) |  | — | — | Table column count | Mismatched column counts cause unpredictable table rendering across processors |
| [MD058](https://github.com/DavidAnson/markdownlint/blob/main/doc/md058.md) |  | — | — | Tables should be surrounded by blank lines | Blank lines ensure tables are consistently parsed across Markdown processors |
| [MD059](https://github.com/DavidAnson/markdownlint/blob/main/doc/md059.md) |  | — | — | Link text should be descriptive | "click here" and "read more" are inaccessible; use meaningful link text |
| [MD060](https://github.com/DavidAnson/markdownlint/blob/main/doc/md060.md) |  | `any` |  | Table column style | Preserve independent alignments by default. Config: `style` — `any`, `consistent`, `default`, `left`, `right`, `center` |

## Contributing

Contributions are welcome!

### Development setup

Prerequisites: [mise](https://mise.jdx.dev/) and [Rust](https://rustup.rs/).
Optionally, Docker is needed for
Dockerfile linting. [uv](https://docs.astral.sh/uv/) is required only if working on the Python package.

```bash
git clone --branch opinionated https://github.com/nnutter/mdlint.git
cd mdlint
mise install   # installs prek, tombi, hadolint
cargo build
```

### Code quality

All quality checks run via `prek run -a`.
This must pass before submitting a pull request.

### Pull request process

1. Create a feature branch from `main`
1. Make focused commits with clear messages
1. Add tests for new functionality
1. Run `prek run -a` and fix any failures
1. Submit a PR with a description of what changed and why

### Release process

Releases use [`cargo-release`](https://github.com/crate-ci/cargo-release), which bumps all package manifests in sync
and pushes the tag that triggers CI to build binaries and publish a GitHub release:

```bash
cargo release patch --execute   # or minor / major
```

Once the tag is pushed, CI verifies manifest versions, builds binaries for all 7 platforms, and publishes the GitHub release with those binaries attached.
Package and container registries are not published to automatically.
After publishing a release, the tag workflow calls `.github/workflows/update-tap.yaml` to update `mdlint` in `nnutter/homebrew-tap`.
The tap workflow also supports manual runs for an existing published release tag.

### Homebrew Tap Setup

Before enabling tap updates:

1. Create `Formula/mdlint.rb` in `nnutter/homebrew-tap`, with a stable source archive URL from `nnutter/mdlint`.
   The tap's `bin/update-formula` script must be available.
1. Set the repository variable `APP_CLIENT_ID` and secret `APP_PRIVATE_KEY` in `nnutter/mdlint`.
1. Install that GitHub App on `nnutter/homebrew-tap` with Contents and Pull requests write permissions.
1. Enable auto-merge and configure required checks in the tap repository.

The workflow runs only from `nnutter/mdlint` and limits its App token to `nnutter/homebrew-tap`.
It pushes a formula-update branch, opens a pull request, and enables auto-merge.
It skips updates when the formula already uses the requested tag.
A missing formula or a formula pointing to another source repository causes a failure before any branch push.

## License

The Unlicense - see [LICENSE](./LICENSE) for details.

## Acknowledgments

- [mdlint upstream](https://github.com/swanysimon/mdlint) — the original project that this fork builds on
- [markdownlint](https://github.com/DavidAnson/markdownlint) by David Anson — original rule definitions
- [markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2) also by David Anson - most people's first
  frontend to markdownlint
- [mdformat](https://github.com/hukkin/mdformat) — inspiration for the formatter-first approach
- [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) — Markdown parsing

## Resources

- [Documentation](./README.md)
- [Issue Tracker](https://github.com/nnutter/mdlint/issues)
- [Releases](https://github.com/nnutter/mdlint/releases)
- [markdownlint Rules Reference](https://github.com/DavidAnson/markdownlint/blob/main/doc/Rules.md)
