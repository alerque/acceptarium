berg := require('berg')
cargo := require('cargo')
cargo-set-version := require('cargo-set-version')
git := require('git')
gpg := require('gpg')
just := just_executable()
make := require('make')
rustfmt := require('rustfmt')
taplo := require('taplo')

set script-interpreter := ['zsh', '+o', 'nomatch', '-eu']
set shell := ['zsh', '+o', 'nomatch', '-ecu']
set default-list
set default-script
set positional-arguments
set unstable

export ACCEPTARIUM_BINARY := justfile_directory() + "/acceptarium"

# With positional arguments enabled, we can pass all the arguments to the bash
# shell in a way that will get expanded to the original 'word' breakdown. However,
# when we do this blindly in all cases and the job's positional arguments happen
# to be empty the shell decides we must have wanted a placeholder for an empty
# string argument — a construct that is invalid for many of our commands. The
# solution is to decide up front whether we have any positional arguments at all
# and then either not pass anything or pass them in a way that will get expanded
# properly. As a caveat we can't use this workaround for nested jobs that pass
# positional arguments to other jobs since one layer of quoting is lost, but we
# don't need to because none of those happen to use spaces in arguments anyway.
maybe-pass(args) := if args != "" { '"$@"' } else { "" }

nuke-n-pave:
    {{ git }} clean -dxff -e target -e completions
    ./bootstrap.sh

dev-conf: nuke-n-pave
    ./configure --enable-developer-mode --enable-debug --with-ollama --with-tesseract --with-imagemagick
    {{ make }}

rel-conf: nuke-n-pave
    ./configure --enable-developer-mode --with-ollama --with-tesseract --with-imagemagick
    {{ make }}

[parallel]
build:
    {{ make }} {{ recipe_name() }}

check:
    {{ make }} {{ recipe_name() }}

lint:
    {{ make }} {{ recipe_name() }}

perfect:
    {{ make }} build check lint

restyle:
    {{ git }} ls-files '*.rs' '*.rs.in' | xargs {{ rustfmt }} +nightly
    {{ git }} ls-files '*.toml' | xargs {{ taplo }} format

# This task will build (if necessary) and run the Acceptarium CLI via the development version from this repository.
# This is especially useful for manual testing and can be used from anywhere by invoking the Justfile externally,
# e.g. with an alias such as:
#
#     alias acceptarium='just --justfile ~/path/to/acceptarium/Justfile acceptarium'
#
# Build and run Acceptarium for testing (can be used from outside the project).
[no-cd]
acceptarium *ARGS: build
    $ACCEPTARIUM_BINARY {{ maybe-pass(ARGS) }}

[doc('Block execution if Git working tree isn’t pristine.')]
[private]
pristine:
    # Make sure Git's status cache is warmed up
    {{ git }} diff --shortstat
    # Ensure there are no changes in staging
    {{ git }} diff-index --quiet --cached HEAD || exit 1
    # Ensure there are no changes in the working tree
    {{ git }} diff-files --quiet || exit 1

[doc('Block execution if we don’t have access to private keys.')]
[private]
keys:
    {{ gpg }} -a --sign > /dev/null <<< 'test'

release semver: pristine keys
    {{ cargo-set-version }} set-version {{ semver }}
    {{ taplo }} format Cargo.toml
    {{ make }} SEMVER={{ semver }} CHANGELOG.md acceptarium-{{ semver }}.md -B
    {{ git }} add -f Cargo.{toml,lock} README.md CHANGELOG.md
    {{ git }} commit -m 'chore: Release v{{ semver }}'
    {{ git }} tag -s v{{ semver }} -F acceptarium-{{ semver }}.md
    {{ git }} diff-files --quiet || exit 1
    ./config.status && {{ make }}
    {{ git }} push --atomic origin master v{{ semver }}
    {{ cargo }} publish --locked

post-release semver: keys
    # {{ berg }} release download v{{ semver }} --skip-existing
    ls acceptarium-{{ semver }}.{tar.zst,zip} | xargs -n1 {{ gpg }} -a --detach-sign
    # {{ berg }} release upload v{{ semver }} acceptarium-{{ semver }}.{tar.zst,zip}.asc
