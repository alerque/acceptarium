ACLOCAL_AMFLAGS = -I build-aux
AM_DISTCHECK_CONFIGURE_FLAGS = --enable-developer-mode

.ONESHELL:
.SECONDARY:
.SECONDEXPANSION:
.DELETE_ON_ERROR:

docdir = $(datarootdir)/doc/$(TRANSFORMED_PACKAGE_NAME)
licensedir = $(datarootdir)/licenses/$(TRANSFORMED_PACKAGE_NAME)
datadir = $(datarootdir)/$(TRANSFORMED_PACKAGE_NAME)
scriptsdir = $(datadir)/scripts
scriptlibdir = $(datadir)/lib

bin_PROGRAMS = acceptarium
acceptarium_SOURCES  = src/bin/acceptarium.rs src/config.rs src/cli.rs src/lib.rs src/error.rs src/types.rs src/assets.rs src/utils.rs
acceptarium_SOURCES += src/process.rs src/output.rs src/run.rs src/status.rs src/tui.rs
acceptarium_SOURCES += src/actions.rs src/storage.rs src/storage/filesystem.rs src/storage/git_annex.rs src/storage/git_tracker.rs
acceptarium_SOURCES += src/ingestable.rs src/ingestable/local_file.rs
acceptarium_SOURCES += src/defaults.toml
EXTRA_acceptarium_SOURCES =
EXTRA_DIST =
dist_doc_DATA = README.md CHANGELOG.md
dist_license_DATA = LICENSES/AGPL-3.0-only.txt
nodist_man_MANS =
dist_scripts_SCRIPTS = scripts/debug.zsh
dist_scriptlib_SCRIPTS = lib/functions.zsh

BUILT_SOURCES =

CLEANFILES = $(dist_scripts_SCRIPTS)

CARGO_FEATURE_ARGS = --features shells

if TUI
CARGO_FEATURE_ARGS += --features tui
endif

if GIT
CARGO_FEATURE_ARGS += --features git
endif

if GIT_ANNEX
CARGO_FEATURE_ARGS += --features git-annex
endif

if OLLAMA
CARGO_FEATURE_ARGS += --features ml,ollama
endif

if TESSERACT
CARGO_FEATURE_ARGS += --features ocr,tesseract
endif

if IMAGEMAGICK
CARGO_FEATURE_ARGS += --features imagemagick
endif

DISTCLEANFILES = @AMINCLUDE@

# A classical use of the autoconf-archive include macro would expand
# INC_AMINCLUDE here, but the perl script that inlines include statements
# runs before the automake that organizes logic and performs substitution.
# Consequentially with a substitution here it becomes impossible to use
# automake conditionals and substitutions in the included Makefile fragments.
# By entering the expanded value directly we are ready in time for the inlining
# functionality and hence can use conditionals in included makefile fragments.
include $(top_srcdir)/aminclude.am

.PHONY: build
build: $(bin_PROGRAMS)

PHONY_DEVELOPER_TARGETS = release-preview reuse-lint typos binstall
.PHONY: $(PHONY_DEVELOPER_TARGETS)

if DEVELOPER_MODE

TAG := v$(SEMVER)

binstall: @PACKAGE_NAME@$(EXEEXT)
	cp $< $(<)-v$(VERSION)-$(CARGO_TARGET_TRIPLE)

lint: reuse-lint typos

reuse-lint:
	$(REUSE) lint

typos:
	$(TYPOS) .

release-preview:
	$(GIT_CLIFF) --unreleased

CHANGELOG.md:
	$(SED) -i '1 i\\' $@
	$(GIT_CLIFF) -p $@ --unreleased -t $(TAG)

acceptarium-%.md: CHANGELOG.md
	$(SED) -e '/^## \[$*\]/,/^## \[/!d' $< | \
		$(SED) -e '1s/^/Release v$*\n/' | \
		$(SED) -e '/^#/d' -e '/^$$/d' | \
		$(SED) -e 's/\*//g' | \
		$(SED) -e '1a\\' > $@

else !DEVELOPER_MODE

$(PHONY_DEVELOPER_TARGETS):
	@: $(error "Please reconfigure using --enable-developer-mode to use developer tooling")

endif !DEVELOPER_MODE
