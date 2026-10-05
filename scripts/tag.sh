#!/usr/bin/env bash
# Create the annotated release tag for the version committed at HEAD, and with
# --push send the branch and tag to origin in one atomic push. Never edits files.
set -euo pipefail

die() { printf '%s\n' "error: $*" >&2; exit 1; }

push=0
for arg in "$@"; do
    case "$arg" in
        --push) push=1 ;;
        --help|-h)
            printf '%s\n' 'Usage: scripts/tag.sh [--push]' \
                'Tags HEAD as v<Cargo.toml version>. With --push, pushes the branch and the tag, which publishes.'
            exit 0
            ;;
        *) die "Unknown argument: $arg" ;;
    esac
done

cd "$(dirname "${BASH_SOURCE[0]}")/.."
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die 'Run from a Git checkout'
commit=$(git rev-parse --verify HEAD 2>/dev/null) || die 'Commit the release first'
[[ -z "$(git status --porcelain --untracked-files=all)" ]] || die 'Working tree must be clean, including untracked files'

# Parse Cargo's metadata rather than guessing at TOML syntax.
version=$(cargo metadata --no-deps --locked --format-version 1 | python3 -c '
import json, sys
from pathlib import Path
data = json.load(sys.stdin)
manifest = Path("Cargo.toml").resolve()
packages = [p for p in data["packages"] if Path(p["manifest_path"]).resolve() == manifest]
if len(packages) != 1:
    sys.exit("error: expected one root package")
print(packages[0]["version"])
')
tag=v$version
git check-ref-format "refs/tags/$tag" >/dev/null || die "Invalid tag name: $tag"

! git rev-parse -q --verify "refs/tags/$tag" >/dev/null || die "Tag $tag already exists locally"
set +e
remote=$(git ls-remote --exit-code --refs origin "refs/tags/$tag" 2>&1)
status=$?
set -e
case $status in
    0) die "Tag $tag already exists on origin" ;;
    2) ;;
    *) die "Cannot query origin: $remote" ;;
esac

python3 - "$version" <<'PY'
import re, sys
from pathlib import Path
version = sys.argv[1]
headings = [l for l in Path("CHANGELOG.md").read_text().splitlines() if l.startswith("## ")]
if not headings or not re.fullmatch(rf"## {re.escape(version)} — \d{{4}}-\d{{2}}-\d{{2}}", headings[0]):
    found = headings[0] if headings else "<none>"
    sys.exit(f"error: the first CHANGELOG.md section must be '## {version} — YYYY-MM-DD', found '{found}'")
PY

branch=$(git symbolic-ref --quiet --short HEAD) || branch=
if (( push )) && [[ -z "$branch" ]]; then
    die 'Detached HEAD: check out a branch, or tag without --push and push HEAD:refs/heads/<branch> yourself'
fi

git tag -a "$tag" -m "Release $version"
push_cmd="git push --atomic origin HEAD:refs/heads/${branch:-<branch>} refs/tags/$tag"
if (( push )); then
    git push --atomic origin "HEAD:refs/heads/$branch" "refs/tags/$tag" || die "Push rejected; $tag remains local. Retry with: $push_cmd"
    printf 'Pushed %s and %s. The release workflow now publishes %s.\n' "$branch" "$tag" "$version"
else
    printf 'Created %s at %s. Pushing the tag publishes the release:\n  %s\n' "$tag" "$commit" "$push_cmd"
fi
