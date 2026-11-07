# Personal fork workflow

This checkout uses `https://github.com/freshmea/inkboard` as `origin` and
`https://github.com/VictorLetirand/inkboard` as `upstream`.

- `main` follows `upstream/main`; do not commit personal features there.
- `codex/personal` contains the personal features and tracks `origin/codex/personal`.
- The local default push remote is `origin`. No pull request is needed.

## Everyday work

Work on `codex/personal`. Review and commit the intended files, then run `git push`.
Additional feature branches can start from `codex/personal` and merge back into it.
See [Details and frames](INSPECTOR.md) for the current features and file/sharing compatibility.

## Bring in upstream changes

Start with a clean working tree: commit unfinished work before switching branches.
Run these commands one at a time, and stop if any command fails:

```powershell
git fetch upstream
git switch main
git merge --ff-only upstream/main
git push origin main
git switch codex/personal
git merge main
```

If there are conflicts, edit the affected files to preserve the intended behavior,
stage only the resolved files with `git add`, and finish with `git commit`.
To cancel an unfinished merge, use `git merge --abort`.
If the fast-forward of `main` fails, inspect its history before proceeding; do not force-reset it.

Check the integrated code before pushing the personal branch:

```powershell
cargo fmt --all -- --check
cargo test --release --locked -p inkboard-core -p inkboard-app -p inkboard-session
git push origin codex/personal
```

Test any other crates affected by the upstream changes and manually check the frame/details
features when their behavior changes. Merges preserve commit history but still need behavior checks.

Upstream updates are fetched on demand; this setup does not schedule automatic merges.
The GitHub **Sync fork** action on `main` does not also update `codex/personal`.
