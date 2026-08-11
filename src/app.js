const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;
const { open: openUrl } = window.__TAURI__.shell;

// ---------- Floating logos (onboarding background) ----------

(function spawnFloatingLogos() {
  const container = document.getElementById("floatingLogos");
  if (!container) return;
  const COUNT = 44;

  // Every logo travels the exact same fixed direction: diagonally from the
  // bottom-left area toward the top-right, off-screen. Each one just starts at a
  // random point along the left edge or the bottom edge, so spawns are spread
  // across the whole left+bottom border rather than clumped at the corner.
  // Distance to travel varies, so some finish fast and some slow, but the
  // direction (the slope of the line) is identical for all of them.
  const DIAG = 140; // vw/vh units traveled from start to (off-screen) end

  // Represent each start point in the same 0..200 coordinate running along the
  // left edge (bottom to top) then continuing along the bottom edge (left to
  // right), so "distance apart" along the border is a single comparable number.
  const BORDER_LEN = 200; // 100 for left edge + 100 for bottom edge
  const MIN_GAP = BORDER_LEN / COUNT / 1.3;

  const positions = [];
  for (let i = 0; i < COUNT; i++) {
    let p;
    let attempts = 0;
    do {
      p = Math.random() * BORDER_LEN;
      attempts++;
    } while (positions.some((q) => Math.abs(q - p) < MIN_GAP) && attempts < 40);
    positions.push(p);
  }

  const frag = document.createDocumentFragment();
  positions.forEach((p) => {
    const img = document.createElement("img");
    img.className = "floating-logo";
    img.src = "assets/logo.png";
    img.alt = "";

    let startX, startY;
    if (p < 100) {
      // Left edge, bottom (vh=100) to top (vh=0).
      startX = -10;
      startY = 100 - p;
    } else {
      // Bottom edge, left (vw=0) to right (vw=100).
      startX = p - 100;
      startY = 110;
    }
    const endX = startX + DIAG;
    const endY = startY - DIAG;

    const size = Math.round(40 + Math.random() * 170);
    const dur = 12 + Math.random() * 30; // seconds to cross: some fast, some slow
    // Random start phase so instances are continuously in flight, not synchronized.
    const delay = -(Math.random() * dur).toFixed(1);
    img.style.cssText =
      `--sx:${startX.toFixed(1)}vw; --sy:${startY.toFixed(1)}vh; ` +
      `--ex:${endX.toFixed(1)}vw; --ey:${endY.toFixed(1)}vh; ` +
      `--size:${size}px; --dur:${dur.toFixed(1)}s; animation-delay:${delay}s;`;
    frag.appendChild(img);
  });
  container.appendChild(frag);
})();

// ---------- Elements ----------

const onboardingScreen = document.getElementById("onboarding");
const workspaceScreen = document.getElementById("workspace");
const noRepoScreen = document.getElementById("noRepo");

const repoPathInput = document.getElementById("repoPath");
const pickFolderBtn = document.getElementById("pickFolderBtn");
const authorNameInput = document.getElementById("authorName");
const authorEmailInput = document.getElementById("authorEmail");
const usernameInput = document.getElementById("username");
const patInput = document.getElementById("pat");
const genTokenLink = document.getElementById("genTokenLink");
const continueBtn = document.getElementById("continueBtn");
const tabOpenFolder = document.getElementById("tabOpenFolder");
const tabClone = document.getElementById("tabClone");
const openFolderPane = document.getElementById("openFolderPane");
const clonePane = document.getElementById("clonePane");
const cloneUrlInput = document.getElementById("cloneUrl");
const cloneDestPathInput = document.getElementById("cloneDestPath");
const pickCloneDestBtn = document.getElementById("pickCloneDestBtn");
const cloneBtn = document.getElementById("cloneBtn");
const browseMyReposBtn = document.getElementById("browseMyReposBtn");
const myReposSelect = document.getElementById("myReposSelect");

const repoPathLabel = document.getElementById("repoPathLabel");
const branchSelect = document.getElementById("branchSelect");
const modeBadge = document.getElementById("modeBadge");
const fetchBtn = document.getElementById("fetchBtn");
const pullBtn = document.getElementById("pullBtn");
const pushBtn = document.getElementById("pushBtn");
const closeSessionBtn = document.getElementById("closeSessionBtn");
const noRemoteBar = document.getElementById("noRemoteBar");
const remoteUrlInput = document.getElementById("remoteUrlInput");
const setRemoteBtn = document.getElementById("setRemoteBtn");
const previewNoRepoBtn = document.getElementById("previewNoRepoBtn");
const noRepoPathEl = document.getElementById("noRepoPath");
const initRepoBtn = document.getElementById("initRepoBtn");
const backToOnboardingBtn = document.getElementById("backToOnboardingBtn");
const fileListEl = document.getElementById("fileList");
const selectAllBtn = document.getElementById("selectAllBtn");
const selectNoneBtn = document.getElementById("selectNoneBtn");
const diffHeading = document.getElementById("diffHeading");
const diffEmpty = document.getElementById("diffEmpty");
const diffView = document.getElementById("diffView");
const commitSummaryInput = document.getElementById("commitSummary");
const commitMessageInput = document.getElementById("commitMessage");
const commitBtn = document.getElementById("commitBtn");
const commitHistoryEl = document.getElementById("commitHistory");
const toast = document.getElementById("toast");

// Demo mode fallback data — used only when no real folder is selected.
const DEMO_FILES = [
  { path: "src/main.rs", status: "modified" },
  { path: "README.md", status: "modified" },
  { path: "src-tauri/tauri.conf.json", status: "new" },
  { path: "Cargo.toml", status: "modified" },
];
const DEMO_BRANCHES = ["main", "develop", "feature/onboarding-ui"];
const DEMO_DIFF = `@@ -1,3 +1,4 @@
 fn main() {
-    println!("hello");
+    println!("hello, world!");
+    println!("(demo diff — no real repo selected)");
 }`;
const DEMO_COMMITS = [
  { hash: "9c8d7e6", message: "Add gitignore", time_ago: "5 hours ago" },
  { hash: "e4f5a6b", message: "Fix README typo", time_ago: "1 day ago" },
  { hash: "a1b2c3d", message: "Initial commit", time_ago: "3 days ago" },
];

// Demo: no folder picked, everything is mocked.
// Local: real folder + real git ops, but no PAT — remote actions
//        (push/pull/fetch) are disabled since there's no way to auth.
// Full: real folder + PAT — everything enabled.
function resolveMode(session) {
  const hasRepo = Boolean(session.repo_path) && session.repo_path !== "(no folder selected)";
  const hasPat = Boolean(session.pat && session.pat.trim());
  if (!hasRepo) return "demo";
  if (!hasPat) return "local";
  return "full";
}

// ---------- Toast ----------

const plingSound = new Audio("assets/pling.wav");
const errorSound = new Audio("assets/error.wav");
const commitSound = new Audio("assets/commit.wav");
const pullSound = new Audio("assets/pull.wav");
const fetchSound = new Audio("assets/fetch.wav");
const pushSound = new Audio("assets/push.wav");
const checkSound = new Audio("assets/check.wav");

function playSound(audio) {
  audio.currentTime = 0;
  audio.play().catch(() => {});
}

let toastTimer = null;
function showToast(message, variant = "neutral") {
  toast.textContent = message;
  toast.className = `toast ${variant}`;
  // Force reflow so the sparkle animation restarts on repeated triggers.
  void toast.offsetWidth;
  toast.classList.add(variant === "success" ? "sparkle" : "show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.classList.add("hidden");
  }, 2400);

  if (variant === "success") {
    fireConfetti();
    plingSound.currentTime = 0;
    plingSound.play().catch(() => {});
  } else if (variant === "error") {
    errorSound.currentTime = 0;
    errorSound.play().catch(() => {});
  }
}

// Low-budget confetti: a handful of tiny white squares burst upward from
// the toast and fall away. Pure DOM + CSS animation, no libraries.
function fireConfetti() {
  const rect = toast.getBoundingClientRect();
  const originX = rect.left + rect.width / 2;
  const originY = rect.top;

  for (let i = 0; i < 10; i++) {
    const piece = document.createElement("span");
    piece.className = "confetti-piece";
    const angle = Math.random() * Math.PI * 2; // spread in all directions
    const distance = 24 + Math.random() * 36;
    const dx = Math.cos(angle) * distance;
    const dy = Math.sin(angle) * distance;
    const rotation = Math.random() * 360;
    const delay = Math.random() * 80;

    piece.style.left = `${originX}px`;
    piece.style.top = `${originY}px`;
    piece.style.setProperty("--dx", `${dx}px`);
    piece.style.setProperty("--dy", `${dy}px`);
    piece.style.setProperty("--rot", `${rotation}deg`);
    piece.style.animationDelay = `${delay}ms`;

    document.body.appendChild(piece);
    piece.addEventListener("animationend", () => piece.remove());
  }
}

// ---------- Onboarding ----------

pickFolderBtn.addEventListener("click", async () => {
  // TODO: once real git logic exists, validate this path is an actual git repo
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    repoPathInput.value = selected;
  }
});

function setSourceTab(tab) {
  const isClone = tab === "clone";
  tabOpenFolder.classList.toggle("active", !isClone);
  tabClone.classList.toggle("active", isClone);
  tabOpenFolder.setAttribute("aria-selected", String(!isClone));
  tabClone.setAttribute("aria-selected", String(isClone));
  openFolderPane.classList.toggle("hidden", isClone);
  clonePane.classList.toggle("hidden", !isClone);
  updateBrowseReposVisibility();
}

tabOpenFolder.addEventListener("click", () => setSourceTab("folder"));
tabClone.addEventListener("click", () => setSourceTab("clone"));

// Offering "browse my repos" only makes sense once there's a PAT to call
// the GitHub API with — otherwise it's just a button that always errors.
function updateBrowseReposVisibility() {
  const hasPat = Boolean(patInput.value.trim());
  const cloneOpen = !clonePane.classList.contains("hidden");
  browseMyReposBtn.classList.toggle("hidden", !(hasPat && cloneOpen));
}

patInput.addEventListener("input", updateBrowseReposVisibility);

browseMyReposBtn.addEventListener("click", async () => {
  const pat = patInput.value.trim();
  if (!pat) {
    showToast("Enter a Personal Access Token first", "error");
    return;
  }
  browseMyReposBtn.disabled = true;
  browseMyReposBtn.textContent = "Loading repositories…";
  try {
    const repos = await invoke("list_my_repos", { pat });
    if (repos.length === 0) {
      showToast("No repositories found for this account", "error");
      return;
    }
    myReposSelect.innerHTML = "";
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = `Select a repository (${repos.length} found)…`;
    placeholder.disabled = true;
    placeholder.selected = true;
    myReposSelect.appendChild(placeholder);
    repos.forEach((repo) => {
      const option = document.createElement("option");
      option.value = repo.clone_url;
      option.textContent = repo.private ? `🔒 ${repo.full_name}` : repo.full_name;
      myReposSelect.appendChild(option);
    });
    myReposSelect.classList.remove("hidden");
  } catch (err) {
    console.error("Failed to list repositories:", err);
    showToast(`Error: ${err}`, "error");
  } finally {
    browseMyReposBtn.disabled = false;
    browseMyReposBtn.textContent = "Browse my repositories →";
  }
});

myReposSelect.addEventListener("change", () => {
  if (myReposSelect.value) {
    cloneUrlInput.value = myReposSelect.value;
  }
});

pickCloneDestBtn.addEventListener("click", async () => {
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    cloneDestPathInput.value = selected;
  }
});

cloneBtn.addEventListener("click", async () => {
  const url = cloneUrlInput.value.trim();
  const destPath = cloneDestPathInput.value.trim();
  const pat = patInput.value.trim();

  if (!url) {
    showToast("Enter a repository URL to clone", "error");
    return;
  }
  if (!destPath) {
    showToast("Select a destination folder first", "error");
    return;
  }
  if (!pat) {
    showToast("Enter a Personal Access Token first", "error");
    return;
  }

  cloneBtn.disabled = true;
  cloneBtn.textContent = "Cloning…";
  try {
    await invoke("real_clone", { url, destPath, pat });
    showToast("Repository cloned!", "success");
    repoPathInput.value = destPath;
    setSourceTab("folder");
    cloneUrlInput.value = "";
    cloneDestPathInput.value = "";
    myReposSelect.classList.add("hidden");
  } catch (err) {
    console.error("Clone failed:", err);
    showToast(`Error: ${err}`, "error");
  } finally {
    cloneBtn.disabled = false;
    cloneBtn.textContent = "Clone Repository";
  }
});

async function checkPatAndToast() {
  const pat = patInput.value.trim();
  if (!pat) {
    showToast("PAT invalid or missing!", "error");
    return;
  }
  try {
    const valid = await invoke("check_pat", { pat });
    showToast(
      valid ? "PAT valid!" : "PAT invalid or missing!",
      valid ? "success" : "error"
    );
  } catch (err) {
    console.error("PAT check failed:", err);
    showToast("PAT invalid or missing!", "error");
  }
}

patInput.addEventListener("blur", checkPatAndToast);

genTokenLink.addEventListener("click", async (e) => {
  e.preventDefault();
  // Classic PAT creation page has no URL param for expiration — GitHub only
  // exposes that via the fine-grained token endpoint. User must pick
  // Expiration -> Custom -> 1 day manually on the page that opens.
  await openUrl(
    "https://github.com/settings/tokens/new?scopes=repo&description=ghostgit"
  );
});

continueBtn.addEventListener("click", async () => {
  // No validation blocks proceeding — every field is accepted as-is, even
  // empty. The PAT check just gives feedback via toast, it never blocks.
  await checkPatAndToast();

  // Optional fields fall back to anonymous "ghost" defaults matching the
  // incognito theme; required fields (repo path, PAT) are left as-is so
  // their absence stays visible rather than being silently papered over.
  // Defaults are applied only to the saved session, never written back into
  // the inputs — the fields must stay visibly empty (showing their gray
  // placeholder) when the user hasn't actually typed anything, so returning
  // via Close Session doesn't make defaults look like real entries.
  const session = {
    repo_path: repoPathInput.value || "(no folder selected)",
    username: usernameInput.value.trim() || "ghost",
    pat: patInput.value,
    author_name: authorNameInput.value.trim() || "Ghost",
    author_email: authorEmailInput.value.trim() || "ghost@ghostgit.local",
  };

  try {
    await invoke("save_session", { session });

    const hasRealRepo = repoPathInput.value.trim().length > 0;
    if (hasRealRepo) {
      const isRepo = await invoke("is_git_repo", { repoPath: session.repo_path });
      if (!isRepo) {
        noRepoPathEl.textContent = session.repo_path;
        onboardingScreen.classList.add("hidden");
        noRepoScreen.classList.remove("hidden");
        return;
      }
    }

    await enterWorkspace(session);
  } catch (err) {
    console.error("Failed to continue:", err);
    showToast(`Error: ${err}`);
  }
});

// ---------- "No Git Initialized" screen ----------

previewNoRepoBtn.addEventListener("click", () => {
  noRepoPathEl.textContent = repoPathInput.value || "~/example-folder";
  onboardingScreen.classList.add("hidden");
  noRepoScreen.classList.remove("hidden");
});

backToOnboardingBtn.addEventListener("click", () => {
  noRepoScreen.classList.add("hidden");
  onboardingScreen.classList.remove("hidden");
});

initRepoBtn.addEventListener("click", async () => {
  const repoPath = repoPathInput.value.trim();
  if (!repoPath) {
    showToast("Repository initialized (mock)", "success");
    noRepoScreen.classList.add("hidden");
    onboardingScreen.classList.remove("hidden");
    return;
  }
  try {
    await invoke("init_repo", { repoPath });
    showToast("Repository initialized!", "success");
    noRepoScreen.classList.add("hidden");
    onboardingScreen.classList.remove("hidden");
  } catch (err) {
    console.error("Failed to init repo:", err);
    showToast(`Error: ${err}`, "error");
  }
});

// ---------- Workspace ----------

// null in demo mode (no real folder picked); otherwise the picked repo path.
let currentRepoPath = null;
let currentSession = null;

function applyMode(session) {
  const mode = resolveMode(session);
  const remoteDisabled = mode !== "full";

  modeBadge.textContent = mode === "demo" ? "Demo" : mode === "local" ? "Local" : "Full";
  modeBadge.className = `mode-badge mode-${mode}`;

  fetchBtn.disabled = remoteDisabled;
  pullBtn.disabled = remoteDisabled;
  pushBtn.disabled = remoteDisabled;

  return mode;
}

async function populateBranches() {
  branchSelect.innerHTML = "";

  let branches = DEMO_BRANCHES;
  let current = DEMO_BRANCHES[0];

  if (currentRepoPath) {
    try {
      branches = await invoke("get_branches", { repoPath: currentRepoPath });
      current = await invoke("get_current_branch", { repoPath: currentRepoPath });
      if (branches.length === 0) {
        // Empty repo (init'd but no commits yet) has no branches to list.
        branches = [current];
      }
    } catch (err) {
      console.error("Failed to load branches:", err);
      branches = DEMO_BRANCHES;
      current = DEMO_BRANCHES[0];
    }
  }

  branches.forEach((branch) => {
    const option = document.createElement("option");
    option.value = branch;
    option.textContent = `⎇ ${branch}`;
    branchSelect.appendChild(option);
  });
  branchSelect.value = current;
}

branchSelect.addEventListener("change", async () => {
  const branch = branchSelect.value;
  if (!currentRepoPath) {
    showToast(`Switched to ${branch} (mock)`);
    return;
  }
  try {
    await invoke("switch_branch", { repoPath: currentRepoPath, branch });
    showToast(`Switched to ${branch}`);
    await refreshFileList();
    await refreshCommitHistory();
  } catch (err) {
    console.error("Failed to switch branch:", err);
    showToast(`Error: ${err}`, "error");
  }
});

async function enterWorkspace(session) {
  currentSession = session;
  currentRepoPath = repoPathInput.value.trim() || null;

  repoPathLabel.textContent = session.repo_path;
  applyMode(session);
  await populateBranches();
  await refreshFileList();
  await refreshCommitHistory();
  await refreshRemoteBar();

  onboardingScreen.classList.add("hidden");
  noRepoScreen.classList.add("hidden");
  workspaceScreen.classList.remove("hidden");
}

async function refreshRemoteBar() {
  if (!currentRepoPath) {
    noRemoteBar.classList.add("hidden");
    return;
  }
  try {
    const hasOrigin = await invoke("has_remote", { repoPath: currentRepoPath });
    noRemoteBar.classList.toggle("hidden", hasOrigin);
  } catch (err) {
    console.error("Failed to check remote:", err);
    noRemoteBar.classList.add("hidden");
  }
}

setRemoteBtn.addEventListener("click", async () => {
  const url = remoteUrlInput.value.trim();
  if (!url) {
    showToast("Enter a remote URL first", "error");
    return;
  }
  if (!currentRepoPath) {
    showToast("Set Remote (mock)");
    return;
  }
  try {
    await invoke("set_remote", { repoPath: currentRepoPath, url });
    showToast("Remote 'origin' set!", "success");
    remoteUrlInput.value = "";
    await refreshRemoteBar();
  } catch (err) {
    console.error("Failed to set remote:", err);
    showToast(`Error: ${err}`, "error");
  }
});

async function refreshFileList() {
  let files = DEMO_FILES;
  if (currentRepoPath) {
    try {
      files = await invoke("get_changed_files", { repoPath: currentRepoPath });
    } catch (err) {
      console.error("Failed to load changed files:", err);
      files = [];
    }
  }
  renderFileList(files);
}

let selectedDiffPath = null;

function renderFileList(files) {
  fileListEl.innerHTML = "";
  files.forEach(({ path, status }) => {
    const li = document.createElement("li");
    li.className = `status-${status || "changed"}`;

    const dot = document.createElement("span");
    dot.className = "status-dot";
    dot.title = status || "changed";

    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = true; // visual staging state; commit uses checked paths
    checkbox.dataset.path = path;
    checkbox.addEventListener("change", (e) => {
      e.stopPropagation();
      playSound(checkSound);
    });

    const span = document.createElement("span");
    span.className = "file-path";
    span.textContent = path;

    const discardBtn = document.createElement("button");
    discardBtn.className = "discard-btn";
    discardBtn.textContent = "↺";
    discardBtn.title = "Discard changes";
    discardBtn.addEventListener("click", async (e) => {
      e.stopPropagation();
      await discardFile(path);
    });

    li.appendChild(dot);
    li.appendChild(checkbox);
    li.appendChild(span);
    li.appendChild(discardBtn);
    li.addEventListener("click", () => showDiff(path));
    fileListEl.appendChild(li);
  });

  if (selectedDiffPath && !files.some((f) => f.path === selectedDiffPath)) {
    clearDiff();
  }
}

selectAllBtn.addEventListener("click", () => {
  fileListEl.querySelectorAll('input[type="checkbox"]').forEach((cb) => {
    cb.checked = true;
  });
  playSound(checkSound);
});

selectNoneBtn.addEventListener("click", () => {
  fileListEl.querySelectorAll('input[type="checkbox"]').forEach((cb) => {
    cb.checked = false;
  });
  playSound(checkSound);
});

// ---------- Diff view ----------

function clearDiff() {
  selectedDiffPath = null;
  diffHeading.textContent = "Diff";
  diffEmpty.classList.remove("hidden");
  diffView.classList.add("hidden");
  fileListEl.querySelectorAll("li.selected").forEach((li) => li.classList.remove("selected"));
}

function renderDiffText(text) {
  diffView.innerHTML = "";
  text.split("\n").forEach((line) => {
    const div = document.createElement("div");
    if (line.startsWith("+") && !line.startsWith("+++")) {
      div.className = "diff-add";
    } else if (line.startsWith("-") && !line.startsWith("---")) {
      div.className = "diff-remove";
    } else if (line.startsWith("@@")) {
      div.className = "diff-hunk";
    } else {
      div.className = "diff-context";
    }
    div.textContent = line;
    diffView.appendChild(div);
  });
}

async function showDiff(path) {
  selectedDiffPath = path;
  diffHeading.textContent = path;
  diffEmpty.classList.add("hidden");
  diffView.classList.remove("hidden");

  fileListEl.querySelectorAll("li").forEach((li) => {
    const cb = li.querySelector('input[type="checkbox"]');
    li.classList.toggle("selected", cb && cb.dataset.path === path);
  });

  if (!currentRepoPath) {
    renderDiffText(DEMO_DIFF);
    return;
  }

  try {
    const diffText = await invoke("get_file_diff", { repoPath: currentRepoPath, path });
    renderDiffText(diffText);
  } catch (err) {
    console.error("Failed to load diff:", err);
    renderDiffText(`Error loading diff: ${err}`);
  }
}

async function discardFile(path) {
  if (!currentRepoPath) {
    showToast("Discarded (mock)");
    return;
  }
  try {
    await invoke("discard_file", { repoPath: currentRepoPath, path });
    showToast("Changes discarded");
    if (selectedDiffPath === path) clearDiff();
    await refreshFileList();
  } catch (err) {
    console.error("Failed to discard file:", err);
    showToast(`Error: ${err}`, "error");
  }
}

async function refreshCommitHistory() {
  let commits = DEMO_COMMITS;
  if (currentRepoPath) {
    try {
      commits = await invoke("get_commit_history", { repoPath: currentRepoPath });
    } catch (err) {
      console.error("Failed to load commit history:", err);
      commits = [];
    }
  }
  commitHistoryEl.innerHTML = "";
  commits.forEach(addCommitToList);
}

function addCommitToList(commit) {
  const li = document.createElement("li");
  li.innerHTML = `
    <span class="commit-hash mono">${commit.hash}</span>
    <span class="commit-message">${escapeHtml(commit.message)}</span>
    <span class="commit-meta">${commit.time_ago}</span>
  `;
  commitHistoryEl.prepend(li);
}

function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

commitBtn.addEventListener("click", async () => {
  const summary = commitSummaryInput.value.trim();
  const description = commitMessageInput.value.trim();
  if (!summary) {
    showToast("Enter a commit summary first");
    return;
  }
  const message = description ? `${summary}\n\n${description}` : summary;

  if (!currentRepoPath) {
    // Demo mode: no real repo, keep the original mocked feel.
    const entry = { hash: Math.random().toString(16).slice(2, 9), message: summary, time_ago: "just now" };
    addCommitToList(entry);
    commitSummaryInput.value = "";
    commitMessageInput.value = "";
    playSound(commitSound);
    showToast("Committed (mock)");
    return;
  }

  const stagedPaths = Array.from(
    fileListEl.querySelectorAll('input[type="checkbox"]:checked')
  ).map((cb) => cb.dataset.path);

  if (stagedPaths.length === 0) {
    showToast("No files staged for commit", "error");
    return;
  }

  try {
    const entry = await invoke("real_commit", {
      repoPath: currentRepoPath,
      authorName: currentSession.author_name,
      authorEmail: currentSession.author_email,
      message,
      paths: stagedPaths,
    });
    addCommitToList(entry);
    commitSummaryInput.value = "";
    commitMessageInput.value = "";
    playSound(commitSound);
    showToast("Committed!");
    clearDiff();
    await refreshFileList();
  } catch (err) {
    console.error("Commit failed:", err);
    showToast(`Error: ${err}`, "error");
  }
});

fetchBtn.addEventListener("click", async () => {
  if (!currentRepoPath) {
    playSound(fetchSound);
    showToast("Fetched (mock)");
    return;
  }
  try {
    const msg = await invoke("real_fetch", {
      repoPath: currentRepoPath,
      pat: currentSession.pat,
    });
    playSound(fetchSound);
    showToast(msg, "success");
  } catch (err) {
    console.error("Fetch failed:", err);
    showToast(`Error: ${err}`, "error");
  }
});

pullBtn.addEventListener("click", async () => {
  if (!currentRepoPath) {
    playSound(pullSound);
    showToast("Pulled (mock)");
    return;
  }
  try {
    const msg = await invoke("real_pull", {
      repoPath: currentRepoPath,
      pat: currentSession.pat,
    });
    playSound(pullSound);
    showToast(msg, "success");
    await refreshFileList();
    await refreshCommitHistory();
    await populateBranches();
  } catch (err) {
    console.error("Pull failed:", err);
    showToast(`Error: ${err}`, "error");
  }
});

pushBtn.addEventListener("click", async () => {
  if (!currentRepoPath) {
    playSound(pushSound);
    showToast("Pushed (mock)");
    return;
  }
  try {
    const msg = await invoke("real_push", {
      repoPath: currentRepoPath,
      pat: currentSession.pat,
    });
    playSound(pushSound);
    showToast(msg, "success");
  } catch (err) {
    console.error("Push failed:", err);
    showToast(`Error: ${err}`, "error");
  }
});

closeSessionBtn.addEventListener("click", async () => {
  // Session data (repo path, name, email, username, PAT) is kept in both
  // the Rust session struct and the onboarding form on purpose — this just
  // navigates back so the user can tweak one field and re-continue without
  // retyping everything. Nothing is cleared here.
  playSound(pushSound);
  commitMessageInput.value = "";

  workspaceScreen.classList.add("hidden");
  onboardingScreen.classList.remove("hidden");
});
