AC_PREREQ([2.69])
AC_INIT([acceptarium], [m4_esyscmd(build-aux/git-version-gen .tarball-version)], [caleb@alerque.com])
AC_CONFIG_AUX_DIR([build-aux])
AC_CONFIG_MACRO_DIR([build-aux])
AM_INIT_AUTOMAKE([foreign tar-pax dist-zstd dist-zip no-dist-gzip no-dependencies color-tests])
AM_SILENT_RULES([yes])

# Load up re-usable m4 bits shared across Caleb's projects
QUE_GIT_VERSION
QUE_TRANSFORM_PACKAGE_NAME
QUE_DEVELOPER_MODE
QUE_DIST_CHECKSUMS

# Check for and set variables for tooling used during our build process
AC_PROG_AWK
AC_PROG_GREP
AC_PROG_SED

AX_REQUIRE_PROG(env)
AX_REQUIRE_PROG(zsh)

AC_ARG_WITH([git],
            AS_HELP_STRING([--without-git],
                           [Compile without git vcs support used to track projects]))
AM_CONDITIONAL([GIT], [test "x$with_git" != "xno"])

AC_ARG_WITH([git-annex],
            AS_HELP_STRING([--without-git-annex],
                           [Compile without git-annex support used as default storage driver]))
AM_CONDITIONAL([GIT_ANNEX], [test "x$with_git_annex" != "xno"])

AM_COND_IF([DEVELOPER_MODE], [
    AX_REQUIRE_PROG([just])
    # Release tooling
    AX_REQUIRE_PROG([cargo-deny])
    AX_REQUIRE_PROG([cargo-set-version])
    AX_REQUIRE_PROG([git-cliff])
    AX_REQUIRE_PROG([reuse])
    AX_REQUIRE_PROG([taplo])
    AX_REQUIRE_PROG([typos])
])

AM_COND_IF([GIT], [
    AX_REQUIRE_PROG([git])
])

AM_COND_IF([GIT_ANNEX], [
    AX_REQUIRE_PROG([git])
    AX_REQUIRE_PROG([git-annex])
])

QUE_RUST_BOILERPLATE

# Avoid the need for `--datarootdir=$(cd ..; pwd)` to run directly from
# an uninstalled build in the source directory
AM_COND_IF([DEVELOPER_MODE], [
    datarootdir="$(cd ..; pwd)"
])

AC_CONFIG_FILES([Makefile])

AC_DEFUN([ACCEPTARIUM_SCRIPT], [AC_CONFIG_FILES([scripts/$1], [chmod +x scripts/$1])])
ACCEPTARIUM_SCRIPT([debug.zsh])

AC_OUTPUT

# Avoid the perceived need to run automake again when building from source
# tarballs. The assorted AM include macros assume two passes are needed to
# generate a makefile since one pass has to generate the includes and the
# second one inlines them, but we're distributing a ready-to-use version and
# hence one pass is enough. We can skip ahead and assume the Makefile template
# is ready to go an this point, avoiding the need for autotools at all.
AM_COND_IF([SOURCE_IS_DIST], [touch -r aminclude.am Makefile.in])
