# Contributing to GamerRUST

Thanks for helping make GamerRUST better. Contributions of code, tests, examples, documentation, bug reports, and focused design discussions are welcome.

Because GamerRUST is still early in development, please open an issue before beginning a large feature or public API redesign. This helps confirm the direction and avoids duplicated work.

## Development setup

1. Install [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer.
2. Fork and clone the repository.
3. On Linux, install the native audio and gamepad headers:

   ```sh
   # Ubuntu / Debian
   sudo apt-get install libasound2-dev libudev-dev
   ```

4. Build the project and examples:

   ```sh
   cargo build --all-targets
   cargo build --examples
   ```

## Making a change

- Keep each pull request focused on one concern.
- Add or update tests when behavior changes.
- Update the README or `/docs` when the public API, setup, or examples change.
- Prefer clear, conventional Rust over clever abstractions.
- Avoid unrelated formatting or dependency changes.
- Do not commit generated build output from `target/`.

## Quality checks

Run these commands before opening a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --examples
```

CI runs the same checks on Linux, macOS, and Windows. Some GPU tests are headless, but the interactive examples should also be smoke-tested when a change affects rendering, input, audio, or windowing.

## Commit and pull request guidance

- Use a short, imperative commit subject, such as `Document Linux prerequisites`.
- Explain what changed and why in the pull request description.
- Mention platform-specific behavior and manual testing.
- Link related issues with `Fixes #123` or `Refs #123` when applicable.
- Keep review discussion respectful and technical.

## Documentation

Documentation should use plain language and runnable examples. Relative links are preferred so the same pages work in the repository, on GitHub Pages, and when adapted for the Wiki.

When adding a subsystem, update at least:

- the feature table in `README.md`;
- `docs/architecture.md`;
- `docs/examples.md` if there is a runnable demo;
- `docs/faq.md` when the change answers a recurring setup question.

## Reporting bugs

Open a GitHub issue with:

- operating system and version;
- Rust version (`rustc --version`);
- GPU and driver information for rendering problems;
- gamepad or audio device details when relevant;
- a minimal reproduction or the example that fails;
- the complete error output or backtrace.

Please search existing issues first and remove secrets, personal paths, and other sensitive information from logs.

## Licensing

By contributing, you agree that your contribution may be distributed under the repository's MIT OR Apache-2.0 license terms.
