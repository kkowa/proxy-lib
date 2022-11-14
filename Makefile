#!/usr/bin/env make -f

MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --silent

SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
.DEFAULT_GOAL := help
help: Makefile
	@grep -E '(^[a-zA-Z_-]+:.*?##.*$$)|(^##)' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[32m%-30s\033[0m %s\n", $$1, $$2}' | sed -e 's/\[32m##/[33m/'

LOCBIN := ${PWD}/.bin
PATH := ${PATH}:${LOCBIN}


# =============================================================================
# Common
# =============================================================================
install:  ## Install the app locally
	mkdir -p "${LOCBIN}"

	cargo install cargo-watch grcov
	cargo fetch
.PHONY: install

init:  ## Initialize project repository
	git submodule update --init
	pre-commit autoupdate
	pre-commit install --install-hooks --hook-type pre-commit --hook-type commit-msg
.PHONY: init

run:  ## Run development server
	cargo watch --no-gitignore --why --exec "run -- --verbosity debug"
.PHONY: run


# =============================================================================
# CI
# =============================================================================
ci: lint test scan  ## Run CI tasks
.PHONY: ci

format:  ## Run autoformatters
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged --allow-no-vcs
.PHONY: format

lint:  ## Run all linters
	cargo fmt --check
	cargo clippy
.PHONY: lint

# https://doc.rust-lang.org/rustc/instrument-coverage.html
# https://github.com/mozilla/grcov
test:  ## Run tests
	mkdir -p .report .coverage
	RUSTFLAGS='-C instrument-coverage' LLVM_PROFILE_FILE='.profile/proxy-%p-%m.profraw' \
		cargo test --workspace --target-dir target/.coverage -- -Z unstable-options --format junit --report-time > .report/raw

	split -l1 -d --additional-suffix=.xml .report/raw .report/partial.

	echo 'Generating coverage report in HTML format'
	grcov . \
		--llvm \
		--branch \
		--source-dir . \
		--ignore-not-existing \
		--ignore 'target/*' \
		--ignore 'examples/*' \
		--ignore 'tests/*' \
		--binary-path target/.coverage/debug/ \
		--output-type html \
		--output-path .coverage/html/
.PHONY: test

scan:  ## Run all scans

.PHONY: scan


# =============================================================================
# Handy Scripts
# =============================================================================
clean:  ## Remove temporary files
	rm -rf .coverage/ .profile/ .report/
	find . -path '*.log*' -delete
.PHONY: clean
