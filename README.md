# github-repo-open-issue-trends
Generate tab-separated value (.tsv) files with number of open issues over time in a GitHub repository, categorized by labels. Use your spreadsheet program of choice to turn the data into pretty diagrams.

A bit hacky.

# Example

```sh
RUST_LOG=info \
cargo run -- \
    --period month \
    --label-category "C-bug:bugs" \
    --label-category "C-cleanup:feature requests" \
    --label-category "C-enhancement:feature requests" \
    --label-category "C-feature-accepted:feature requests" \
    --label-category "C-feature-request:feature requests" \
    --label-category "C-future-compatibility:feature requests" \
    --label-category "C-optimization:feature requests" \
    --label-category "C-tracking-issue:feature requests" \
    --label-category "*:uncategorized" \
    --page-size 100 \
    --pages 9999 \
    rust-lang/rust
```
