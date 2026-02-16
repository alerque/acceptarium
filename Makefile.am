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

bin_PROGRAMS = acceptarium
acceptarium_SOURCES  = src/bin/acceptarium.rs src/config.rs src/cli.rs src/lib.rs src/types.rs
acceptarium_SOURCES += src/run.rs src/status.rs
acceptarium_SOURCES += src/storage.rs src/storage/filesystem.rs src/storage/git_annex.rs
EXTRA_acceptarium_SOURCES =
EXTRA_DIST =
dist_doc_DATA = README.md CHANGELOG.md
dist_license_DATA = LICENSES/AGPL-3.0-only.txt
nodist_man_MANS =
dist_scripts_SCRIPTS = scripts/debug.sh

BUILT_SOURCES =

CLEANFILES = $(dist_scripts_SCRIPTS)

CARGO_FEATURE_ARGS = --features shells

if GIT_ANNEX
CARGO_FEATURE_ARGS += --features git-annex
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
