use std::process::Command;
use tempfile::TempDir;

struct TestRepo {
    temp_dir: TempDir,
}

impl TestRepo {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        // initialize git repo
        Command::new("git")
            .args(&["init"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to init git repo");

        // configure git
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git email");

        // create initial commit (needed for stashes to work)
        std::fs::write(repo_path.join("initial.txt"), "initial content")
            .expect("Failed to write initial file");

        Command::new("git")
            .args(&["add", "."])
            .current_dir(repo_path)
            .output()
            .expect("Failed to add initial files");

        Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to create initial commit");

        Self { temp_dir }
    }

    fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    fn create_stash(&self, message: &str) {
        // create and stash some changes
        std::fs::write(self.path().join("test.txt"), "modified content")
            .expect("Failed to write file");

        Command::new("git")
            .args(&["add", "."])
            .current_dir(self.path())
            .output()
            .expect("Failed to add files");

        Command::new("git")
            .args(&["stash", "push", "-m", message])
            .current_dir(self.path())
            .output()
            .expect("Failed to create stash");
    }

    fn get_stash_messages(&self) -> Vec<String> {
        let output = Command::new("git")
            .args(&["stash", "list", "--format=%s"])
            .current_dir(self.path())
            .output()
            .expect("Failed to get stash list");

        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stash list");
        stdout.lines().map(|line| line.to_string()).collect()
    }

    fn get_stash_messages_without_prefix(&self) -> Vec<String> {
        self.get_stash_messages()
            .into_iter()
            .map(|msg| {
                // Remove "On <branch>: " prefix that git adds automatically
                if let Some(colon_pos) = msg.find(": ") {
                    msg[colon_pos + 2..].to_string()
                } else {
                    msg
                }
            })
            .collect()
    }

    fn get_stash_count(&self) -> usize {
        let output = Command::new("git")
            .args(&["stash", "list"])
            .current_dir(self.path())
            .output()
            .expect("Failed to get stash list");

        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stash list");
        stdout.lines().count()
    }

    fn run_git_rename_stash(&self, message: &str, index: usize) -> std::process::Output {
        Command::new("cargo")
            .args(&[
                "run",
                "--",
                "-r",
                self.path().to_str().unwrap(),
                "-m",
                message,
                &index.to_string(),
            ])
            .output()
            .expect("Failed to run git-rename-stash")
    }
}

#[test]
fn test_single_rename() {
    let repo = TestRepo::new();

    repo.create_stash("Original message");

    // verify stash was created
    let messages_before = repo.get_stash_messages_without_prefix();
    assert_eq!(messages_before.len(), 1);
    assert_eq!(messages_before[0], "Original message");

    // rename the stash
    let output = repo.run_git_rename_stash("New message", 0);
    assert!(output.status.success(), "git-rename-stash should succeed");

    // verify the stash was renamed
    let messages_after = repo.get_stash_messages_without_prefix();
    assert_eq!(messages_after.len(), 1);
    assert_eq!(messages_after[0], "New message");
}

#[test]
fn test_multiple_stashes_rename_middle() {
    let repo = TestRepo::new();

    // create multiple stashes
    repo.create_stash("First stash");
    repo.create_stash("Second stash");
    repo.create_stash("Third stash");

    // verify all stashes were created (most recent first in git stash list)
    let messages_before = repo.get_stash_messages_without_prefix();
    assert_eq!(messages_before.len(), 3);
    assert_eq!(messages_before[0], "Third stash"); // most recent
    assert_eq!(messages_before[1], "Second stash"); // middle
    assert_eq!(messages_before[2], "First stash"); // oldest

    let output = repo.run_git_rename_stash("Renamed middle stash", 1);
    assert!(output.status.success(), "git-rename-stash should succeed");

    let messages_after = repo.get_stash_messages_without_prefix();
    assert_eq!(messages_after.len(), 3);

    // verify exact order
    assert_eq!(messages_after[0], "Renamed middle stash"); // most recent (index 0)
    assert_eq!(messages_after[1], "Third stash"); // middle renamed (index 1)
    assert_eq!(messages_after[2], "First stash"); // oldest (index 2)

    // double-check the old message is gone
    assert!(!messages_after.contains(&"Second stash".to_string()));
}

#[test]
fn test_nonexistent_stash() {
    let repo = TestRepo::new();

    repo.create_stash("Only stash");

    // try to rename a non-existent stash (index 5)
    let output = repo.run_git_rename_stash("Should fail", 5);
    assert!(
        !output.status.success(),
        "Should fail when renaming non-existent stash"
    );

    // verify original stash is unchanged
    let messages = repo.get_stash_messages_without_prefix();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "Only stash");
}

#[test]
fn test_no_stashes() {
    let repo = TestRepo::new();

    // don't create any stashes
    assert_eq!(repo.get_stash_count(), 0);

    // try to rename a stash when none exist
    let output = repo.run_git_rename_stash("Should fail", 0);
    assert!(
        !output.status.success(),
        "Should fail when no stashes exist"
    );

    // verify still no stashes
    assert_eq!(repo.get_stash_count(), 0);
}

#[test]
fn test_stash_functionality_preserved() {
    let repo = TestRepo::new();

    // create initial state
    std::fs::write(repo.path().join("work.txt"), "original work")
        .expect("Failed to write work file");

    // create a stash
    std::fs::write(repo.path().join("work.txt"), "modified work")
        .expect("Failed to modify work file");

    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo.path())
        .output()
        .expect("Failed to add files");

    Command::new("git")
        .args(&["stash", "push", "-m", "Work in progress"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to create stash");

    // rename the stash
    let output = repo.run_git_rename_stash("Renamed work", 0);
    assert!(output.status.success());

    // verify stash can still be applied
    let output = Command::new("git")
        .args(&["stash", "pop"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to pop stash");

    assert!(
        output.status.success(),
        "Stash should still be functional after rename"
    );

    // verify the content was restored
    let content =
        std::fs::read_to_string(repo.path().join("work.txt")).expect("Failed to read work file");
    assert_eq!(content, "modified work");
}
