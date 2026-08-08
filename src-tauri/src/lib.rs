use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
#[cfg(debug_assertions)]
use tauri::Manager;
use tauri::State;

// In-memory session state. Nothing here ever touches disk except via the
// dialog plugin's folder picker and the real git2-rs calls below.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Session {
    pub repo_path: String,
    pub username: String,
    pub pat: String,
    pub author_name: String,
    pub author_email: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct CommitEntry {
    pub hash: String,
    pub message: String,
    pub time_ago: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub status: String,
}

#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<Session>>,
}

#[tauri::command]
fn save_session(state: State<AppState>, session: Session) -> Result<Session, String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    *guard = Some(session.clone());
    Ok(session)
}

#[tauri::command]
fn get_session(state: State<AppState>) -> Result<Option<Session>, String> {
    let guard = state.session.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
fn clear_session(state: State<AppState>) -> Result<(), String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

// Real check: does the given path already contain a git repository?
#[tauri::command]
fn is_git_repo(repo_path: String) -> bool {
    Repository::open(Path::new(&repo_path)).is_ok()
}

// Real: git2::Repository::init
#[tauri::command]
fn init_repo(repo_path: String) -> Result<(), String> {
    Repository::init(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    Ok(())
}

// Real: walks working tree + index status via git2::Repository::statuses
#[tauri::command]
fn get_changed_files(repo_path: String) -> Result<Vec<ChangedFile>, String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;

    let mut opts = StatusOptions::new();
    // include_ignored defaults to false, but set it explicitly: files/paths
    // matched by the repo's own .gitignore must never show up as "changed".
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    for entry in statuses.iter() {
        let path = match entry.path() {
            Some(p) => p.to_string(),
            None => continue,
        };
        let status = entry.status();
        let label = if status.is_wt_new() || status.is_index_new() {
            "new"
        } else if status.is_wt_modified() || status.is_index_modified() {
            "modified"
        } else if status.is_wt_deleted() || status.is_index_deleted() {
            "deleted"
        } else if status.is_wt_renamed() || status.is_index_renamed() {
            "renamed"
        } else {
            "changed"
        };
        files.push(ChangedFile {
            path,
            status: label.to_string(),
        });
    }

    Ok(files)
}

// Real: diffs a single working-tree file against HEAD (or against "nothing"
// for untracked/new files) and renders it as unified-diff text lines.
#[tauri::command]
fn get_file_diff(repo_path: String, path: String) -> Result<String, String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts
        .pathspec(&path)
        .include_untracked(true)
        .recurse_untracked_dirs(true);

    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_opts))
        .map_err(|e| e.to_string())?;

    let mut out = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let prefix = match line.origin() {
            '+' => "+",
            '-' => "-",
            ' ' => " ",
            _ => "",
        };
        if line.origin() == 'F' || line.origin() == 'H' {
            out.push_str(&String::from_utf8_lossy(line.content()));
        } else {
            out.push_str(prefix);
            out.push_str(&String::from_utf8_lossy(line.content()));
        }
        true
    })
    .map_err(|e| e.to_string())?;

    if out.trim().is_empty() {
        return Ok("(no textual diff available — binary or empty file)".into());
    }

    Ok(out)
}

// Real: discards working-tree changes for a single file, restoring it to
// its HEAD state. For untracked/new files, deletes them from disk instead
// (there is no HEAD version to restore to).
#[tauri::command]
fn discard_file(repo_path: String, path: String) -> Result<(), String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;

    let is_tracked = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_tree().ok())
        .map(|tree| tree.get_path(Path::new(&path)).is_ok())
        .unwrap_or(false);

    if is_tracked {
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.path(&path).force();
        repo.checkout_head(Some(&mut checkout))
            .map_err(|e| e.to_string())?;
    } else {
        let full_path = Path::new(&repo_path).join(&path);
        if full_path.exists() {
            std::fs::remove_file(&full_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// Real: lists local branches via git2::Repository::branches
#[tauri::command]
fn get_branches(repo_path: String) -> Result<Vec<String>, String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| e.to_string())?;

    let mut names = Vec::new();
    for branch in branches {
        let (branch, _) = branch.map_err(|e| e.to_string())?;
        if let Some(name) = branch.name().map_err(|e| e.to_string())? {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

#[tauri::command]
fn get_current_branch(repo_path: String) -> Result<String, String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    Ok(head.shorthand().unwrap_or("HEAD").to_string())
}

// Real: checks out the given local branch
#[tauri::command]
fn switch_branch(repo_path: String, branch: String) -> Result<(), String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let refname = format!("refs/heads/{}", branch);
    let obj = repo.revparse_single(&refname).map_err(|e| e.to_string())?;
    repo.checkout_tree(&obj, None).map_err(|e| e.to_string())?;
    repo.set_head(&refname).map_err(|e| e.to_string())?;
    Ok(())
}

// Real: stages the given paths (or all changes) and creates a commit against
// HEAD using the author identity from the session.
#[tauri::command]
fn real_commit(
    repo_path: String,
    author_name: String,
    author_email: String,
    message: String,
    paths: Vec<String>,
) -> Result<CommitEntry, String> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".into());
    }

    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;

    if paths.is_empty() {
        return Err("No files staged for commit".into());
    }
    for path in &paths {
        index
            .add_path(Path::new(path))
            .map_err(|e| format!("Failed to stage {path}: {e}"))?;
    }
    index.write().map_err(|e| e.to_string())?;

    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let signature = git2::Signature::now(&author_name, &author_email).map_err(|e| e.to_string())?;

    let parent_commit = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok());

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    let commit_oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &parents,
        )
        .map_err(|e| e.to_string())?;

    Ok(CommitEntry {
        hash: commit_oid.to_string()[..7].to_string(),
        message,
        time_ago: "just now".into(),
    })
}

// Real: walks commit history from HEAD via git2::Repository::revwalk
#[tauri::command]
fn get_commit_history(repo_path: String) -> Result<Vec<CommitEntry>, String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;

    // An empty repo (no commits yet) has no valid HEAD to walk from.
    if repo.head().is_err() {
        return Ok(Vec::new());
    }

    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push_head().map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= 50 {
            break;
        }
        let oid = oid.map_err(|e| e.to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
        entries.push(CommitEntry {
            hash: oid.to_string()[..7].to_string(),
            message: commit.summary().unwrap_or("").to_string(),
            time_ago: relative_time(commit.time().seconds()),
        });
    }

    Ok(entries)
}

fn relative_time(commit_secs: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let diff = (now - commit_secs).max(0);

    if diff < 60 {
        "just now".into()
    } else if diff < 3600 {
        format!("{} minutes ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}

#[tauri::command]
fn has_remote(repo_path: String) -> Result<bool, String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let has_origin = repo.find_remote("origin").is_ok();
    Ok(has_origin)
}

#[tauri::command]
fn set_remote(repo_path: String, url: String) -> Result<(), String> {
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", &url)
            .map_err(|e| e.to_string())?;
    } else {
        repo.remote("origin", &url).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// GitHub's HTTPS PAT convention: any non-empty username works, the PAT goes
// in as the password. Using "x-access-token" as the username matches what
// GitHub Actions / most tooling use.
//
// libgit2 re-invokes this callback on every auth failure, retrying with
// whatever we return. If the PAT itself is rejected (bad/expired/no write
// access), returning the same credentials forever just spins until libgit2
// gives up with a generic "too many redirects or authentication replays"
// error that hides the real 401/403. Bailing out after one attempt makes
// the actual HTTP status surface instead.
fn make_remote_callbacks(pat: &str) -> git2::RemoteCallbacks<'_> {
    let mut callbacks = git2::RemoteCallbacks::new();
    let mut attempted = false;
    callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
        if attempted {
            return Err(git2::Error::from_str(
                "Authentication rejected — check that the PAT has write access to this repository",
            ));
        }
        attempted = true;
        git2::Cred::userpass_plaintext("x-access-token", pat)
    });
    callbacks
}

// Real: pushes the current branch to origin over HTTPS using the PAT.
#[tauri::command]
fn real_push(repo_path: String, pat: String) -> Result<String, String> {
    if pat.trim().is_empty() {
        return Err("No PAT available for push".into());
    }
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| "No remote 'origin' configured".to_string())?;

    let head = repo.head().map_err(|e| e.to_string())?;
    let branch_name = head.shorthand().unwrap_or("main");
    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");

    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(make_remote_callbacks(&pat));

    remote
        .push(&[refspec], Some(&mut push_opts))
        .map_err(|e| format!("Push failed ({:?}): {}", e.class(), e.message()))?;

    Ok(format!("Pushed {branch_name} to origin"))
}

// Real: fetches from origin over HTTPS using the PAT. Only updates remote
// tracking refs — never touches the working tree, so it's safe to run
// without any merge/conflict handling.
#[tauri::command]
fn real_fetch(repo_path: String, pat: String) -> Result<String, String> {
    if pat.trim().is_empty() {
        return Err("No PAT available for fetch".into());
    }
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| "No remote 'origin' configured".to_string())?;

    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(make_remote_callbacks(&pat));

    remote
        .fetch(&[] as &[&str], Some(&mut fetch_opts), None)
        .map_err(|e| format!("Fetch failed ({:?}): {}", e.class(), e.message()))?;

    Ok("Fetched from origin".into())
}

// Real: fetches from origin over HTTPS using the PAT, then fast-forwards the
// current branch to the fetched remote-tracking ref. Only fast-forwards —
// if local and remote history have diverged, this returns an error instead
// of attempting a merge, since there's no conflict-resolution UI yet.
#[tauri::command]
fn real_pull(repo_path: String, pat: String) -> Result<String, String> {
    if pat.trim().is_empty() {
        return Err("No PAT available for pull".into());
    }
    let repo = Repository::open(Path::new(&repo_path)).map_err(|e| e.to_string())?;
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| "No remote 'origin' configured".to_string())?;

    let head = repo.head().map_err(|e| e.to_string())?;
    let branch_name = head
        .shorthand()
        .ok_or("HEAD is not a valid branch")?
        .to_string();

    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(make_remote_callbacks(&pat));
    remote
        .fetch(&[&branch_name], Some(&mut fetch_opts), None)
        .map_err(|e| format!("Fetch failed ({:?}): {}", e.class(), e.message()))?;

    let remote_ref_name = format!("refs/remotes/origin/{branch_name}");
    let remote_ref = repo.find_reference(&remote_ref_name).map_err(|_| {
        format!("Remote branch 'origin/{branch_name}' does not exist")
    })?;
    let remote_commit = repo
        .reference_to_annotated_commit(&remote_ref)
        .map_err(|e| e.to_string())?;

    let (merge_analysis, _) = repo
        .merge_analysis(&[&remote_commit])
        .map_err(|e| e.to_string())?;

    if merge_analysis.is_up_to_date() {
        return Ok(format!("Already up to date with origin/{branch_name}"));
    }

    if !merge_analysis.is_fast_forward() {
        return Err(
            "Local and remote branches have diverged — merge manually before pulling".into(),
        );
    }

    let local_refname = format!("refs/heads/{branch_name}");
    let mut local_ref = repo.find_reference(&local_refname).map_err(|e| e.to_string())?;
    let target_oid = remote_commit.id();

    local_ref
        .set_target(target_oid, "ghostgit: fast-forward pull")
        .map_err(|e| e.to_string())?;
    repo.set_head(&local_refname).map_err(|e| e.to_string())?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::new().force(),
    ))
    .map_err(|e| e.to_string())?;

    Ok(format!("Pulled — fast-forwarded {branch_name} to origin"))
}

// Real check: hits GitHub's REST API with the given PAT and reports whether
// it authenticates. This is the only non-mocked network call in this pass.
#[tauri::command]
async fn check_pat(pat: String) -> Result<bool, String> {
    if pat.trim().is_empty() {
        return Ok(false);
    }

    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", pat))
        .header("User-Agent", "ghostgit")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.status().is_success())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            save_session,
            get_session,
            clear_session,
            is_git_repo,
            init_repo,
            get_changed_files,
            get_file_diff,
            discard_file,
            get_branches,
            get_current_branch,
            switch_branch,
            real_commit,
            get_commit_history,
            has_remote,
            set_remote,
            real_push,
            real_fetch,
            real_pull,
            check_pat,
        ])
        .setup(|_app| {
            #[cfg(debug_assertions)]
            {
                let window = _app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    // git2's Signature::now() reads the user's global/system gitconfig,
    // which briefly locks a shared file outside any of our temp repos.
    // Running these tests in parallel (the default) can race on that lock
    // and fail spuriously, so serialize just the commit-creating tests.
    static COMMIT_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn temp_repo_path() -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = format!(
            "ghostgit-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        dir.push(unique);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn relative_time_formats_buckets_correctly() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        assert_eq!(relative_time(now), "just now");
        assert_eq!(relative_time(now - 120), "2 minutes ago");
        assert_eq!(relative_time(now - 7200), "2 hours ago");
        assert_eq!(relative_time(now - 172800), "2 days ago");
    }

    #[test]
    fn is_git_repo_reports_false_for_plain_directory() {
        let dir = temp_repo_path();
        assert!(!is_git_repo(dir.to_str().unwrap().to_string()));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn init_repo_makes_is_git_repo_true() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let dir = temp_repo_path();
        let path = dir.to_str().unwrap().to_string();

        init_repo(path.clone()).expect("init_repo should succeed");
        assert!(is_git_repo(path));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn commit_flow_creates_history_and_clears_changed_files() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let dir = temp_repo_path();
        let path = dir.to_str().unwrap().to_string();

        init_repo(path.clone()).unwrap();
        fs::write(dir.join("hello.txt"), "hello world\n").unwrap();

        let changed = get_changed_files(path.clone()).unwrap();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].path, "hello.txt");
        assert_eq!(changed[0].status, "new");

        let entry = real_commit(
            path.clone(),
            "Test Author".to_string(),
            "test@example.com".to_string(),
            "Initial commit".to_string(),
            vec!["hello.txt".to_string()],
        )
        .expect("commit should succeed");
        assert_eq!(entry.message, "Initial commit");
        assert_eq!(entry.hash.len(), 7);

        let changed_after = get_changed_files(path.clone()).unwrap();
        assert!(changed_after.is_empty());

        let history = get_commit_history(path).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].message, "Initial commit");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn commit_with_empty_message_is_rejected() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let dir = temp_repo_path();
        let path = dir.to_str().unwrap().to_string();
        init_repo(path.clone()).unwrap();
        fs::write(dir.join("a.txt"), "a\n").unwrap();

        let result = real_commit(
            path,
            "Test Author".to_string(),
            "test@example.com".to_string(),
            "   ".to_string(),
            vec!["a.txt".to_string()],
        );
        assert!(result.is_err());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn commit_with_no_staged_paths_is_rejected() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let dir = temp_repo_path();
        let path = dir.to_str().unwrap().to_string();
        init_repo(path.clone()).unwrap();
        fs::write(dir.join("a.txt"), "a\n").unwrap();

        let result = real_commit(
            path,
            "Test Author".to_string(),
            "test@example.com".to_string(),
            "A message".to_string(),
            vec![],
        );
        assert!(result.is_err());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn discard_file_removes_untracked_file() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let dir = temp_repo_path();
        let path = dir.to_str().unwrap().to_string();
        init_repo(path.clone()).unwrap();
        let file_path = dir.join("throwaway.txt");
        fs::write(&file_path, "scratch\n").unwrap();

        assert!(file_path.exists());
        discard_file(path, "throwaway.txt".to_string()).unwrap();
        assert!(!file_path.exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn branch_listing_reflects_current_branch_after_first_commit() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let dir = temp_repo_path();
        let path = dir.to_str().unwrap().to_string();
        init_repo(path.clone()).unwrap();
        fs::write(dir.join("a.txt"), "a\n").unwrap();
        real_commit(
            path.clone(),
            "Test Author".to_string(),
            "test@example.com".to_string(),
            "Initial commit".to_string(),
            vec!["a.txt".to_string()],
        )
        .unwrap();

        let current = get_current_branch(path.clone()).unwrap();
        let branches = get_branches(path).unwrap();
        assert!(branches.contains(&current));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn has_remote_is_false_until_set_remote_is_called() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let dir = temp_repo_path();
        let path = dir.to_str().unwrap().to_string();
        init_repo(path.clone()).unwrap();

        assert!(!has_remote(path.clone()).unwrap());

        set_remote(
            path.clone(),
            "https://example.com/user/repo.git".to_string(),
        )
        .unwrap();
        assert!(has_remote(path).unwrap());

        fs::remove_dir_all(&dir).unwrap();
    }

    // Builds an "origin" repo with one commit on its default branch, and a
    // second repo cloned from it with 'origin' pointed at origin's local
    // path. Local file-path remotes don't invoke the credentials callback,
    // so real_pull's fetch+merge logic can be exercised without a network —
    // only the (non-empty) PAT string is required to pass the empty check.
    fn make_origin_and_clone() -> (std::path::PathBuf, std::path::PathBuf, String) {
        let origin_dir = temp_repo_path();
        let origin_path = origin_dir.to_str().unwrap().to_string();
        init_repo(origin_path.clone()).unwrap();
        fs::write(origin_dir.join("a.txt"), "a\n").unwrap();
        real_commit(
            origin_path.clone(),
            "Origin Author".to_string(),
            "origin@example.com".to_string(),
            "Initial commit".to_string(),
            vec!["a.txt".to_string()],
        )
        .unwrap();
        let branch_name = get_current_branch(origin_path.clone()).unwrap();

        let clone_dir = temp_repo_path();
        Repository::clone(&origin_path, &clone_dir).expect("clone should succeed");

        (origin_dir, clone_dir, branch_name)
    }

    #[test]
    fn pull_fast_forwards_when_origin_has_new_commits() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let (origin_dir, clone_dir, _branch) = make_origin_and_clone();
        let origin_path = origin_dir.to_str().unwrap().to_string();
        let clone_path = clone_dir.to_str().unwrap().to_string();

        // Advance origin with a second commit the clone doesn't have yet.
        fs::write(origin_dir.join("b.txt"), "b\n").unwrap();
        real_commit(
            origin_path,
            "Origin Author".to_string(),
            "origin@example.com".to_string(),
            "Second commit".to_string(),
            vec!["b.txt".to_string()],
        )
        .unwrap();

        let history_before = get_commit_history(clone_path.clone()).unwrap();
        assert_eq!(history_before.len(), 1);

        let msg = real_pull(clone_path.clone(), "dummy-pat".to_string()).unwrap();
        assert!(msg.contains("fast-forwarded"), "unexpected message: {msg}");

        let history_after = get_commit_history(clone_path.clone()).unwrap();
        assert_eq!(history_after.len(), 2);
        assert!(clone_dir.join("b.txt").exists());

        fs::remove_dir_all(&origin_dir).unwrap();
        fs::remove_dir_all(&clone_dir).unwrap();
    }

    #[test]
    fn pull_reports_already_up_to_date() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let (origin_dir, clone_dir, _branch) = make_origin_and_clone();
        let clone_path = clone_dir.to_str().unwrap().to_string();

        let msg = real_pull(clone_path, "dummy-pat".to_string()).unwrap();
        assert!(msg.contains("Already up to date"), "unexpected message: {msg}");

        fs::remove_dir_all(&origin_dir).unwrap();
        fs::remove_dir_all(&clone_dir).unwrap();
    }

    #[test]
    fn pull_rejects_diverged_history() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let (origin_dir, clone_dir, _branch) = make_origin_and_clone();
        let origin_path = origin_dir.to_str().unwrap().to_string();
        let clone_path = clone_dir.to_str().unwrap().to_string();

        // Diverge both sides: origin gets a commit, and the clone gets an
        // unrelated local commit of its own, so neither is an ancestor of
        // the other and a fast-forward is impossible.
        fs::write(origin_dir.join("b.txt"), "b\n").unwrap();
        real_commit(
            origin_path,
            "Origin Author".to_string(),
            "origin@example.com".to_string(),
            "Origin-side commit".to_string(),
            vec!["b.txt".to_string()],
        )
        .unwrap();

        fs::write(clone_dir.join("c.txt"), "c\n").unwrap();
        real_commit(
            clone_path.clone(),
            "Clone Author".to_string(),
            "clone@example.com".to_string(),
            "Clone-side commit".to_string(),
            vec!["c.txt".to_string()],
        )
        .unwrap();

        let result = real_pull(clone_path, "dummy-pat".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("diverged"));

        fs::remove_dir_all(&origin_dir).unwrap();
        fs::remove_dir_all(&clone_dir).unwrap();
    }

    #[test]
    fn pull_requires_non_empty_pat() {
        let _guard = COMMIT_TEST_LOCK.lock().unwrap();
        let (origin_dir, clone_dir, _branch) = make_origin_and_clone();
        let clone_path = clone_dir.to_str().unwrap().to_string();

        let result = real_pull(clone_path, "".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("PAT"));

        fs::remove_dir_all(&origin_dir).unwrap();
        fs::remove_dir_all(&clone_dir).unwrap();
    }
}
