# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Adding integration tests
- New themes
- Find and subscribe to Public channels
- Auth event [NIP-42](https://github.com/nostr-protocol/nips/blob/master/42.md)

### Changed
- No more pending message in the database, only in memory.
- Changend main views to use the Route trait
- ModalView trait to modals
- Better organization of the net mod file.

### Fixed
- Clippy fixes
- Top padding of settings view
- Padding of modals

### Removed
