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
set positional-arguments := true
set unstable := true

[default]
[private]
@list:
    {{ just }} --list --unsorted

nuke-n-pave:
    {{ git }} clean -dxff -e target -e completions
    ./bootstrap.sh

dev-conf: nuke-n-pave
    ./configure --enable-developer-mode --enable-debug --with-ollama --with-tesseract
    {{ make }}

rel-conf: nuke-n-pave
    ./configure --enable-developer-mode --with-ollama --with-tesseract
    {{ make }}

[parallel]
build:
    {{ make }} $0

check:
    {{ make }} $0

lint:
    {{ make }} $0

perfect:
    {{ make }} build check lint

restyle:
    {{ git }} ls-files '*.rs' '*.rs.in' | xargs {{ rustfmt }} --edition 2024 --config skip_children=true
    {{ git }} ls-files '*.toml' | xargs {{ taplo }} format

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

