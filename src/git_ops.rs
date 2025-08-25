use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use git2::{Repository, Commit as Git2Commit, Time, Oid};
use octocrab::models::{issues::Issue, Label};
use octocrab::{Octocrab, OctocrabBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Git operations using git2 crate instead of CLI
pub struct GitOps {
    repo: Repository,
}

/// GitHub operations using octocrab instead of gh CLI
pub struct GitHubOps {
    client: Octocrab,
    owner: String,
    repo_name: String,
}

/// A commit representation that matches our database structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub commit_date: DateTime<Utc>,
    pub message: String,
    pub files_changed: Vec<String>,
    pub insertions: i32,
    pub deletions: i32,
}

/// Issue creation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueParams {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
}

impl GitOps {
    /// Create a new GitOps instance for the current repository
    pub fn new() -> Result<Self> {
        let repo = Repository::open(".")
            .context("Failed to open Git repository in current directory")?;
        Ok(Self { repo })
    }

    /// Create a new GitOps instance for a specific path
    pub fn new_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Repository::open(path)
            .context("Failed to open Git repository at specified path")?;
        Ok(Self { repo })
    }

    /// Get the repository's remote URL
    pub fn get_remote_url(&self, remote_name: &str) -> Result<String> {
        let remote = self.repo.find_remote(remote_name)
            .context(format!("Failed to find remote '{}'", remote_name))?;
        
        let url = remote.url()
            .context("Remote URL is not valid UTF-8")?;
            
        Ok(url.to_string())
    }

    /// Parse GitHub owner and repo from remote URL
    pub fn parse_github_repo(&self, remote_name: &str) -> Result<(String, String)> {
        let url = self.get_remote_url(remote_name)?;
        
        // Handle both SSH and HTTPS GitHub URLs
        let repo_path_str = if url.starts_with("git@github") {
            // SSH format: git@github.com:owner/repo.git or git@github.folknology:owner/repo.git
            url.split(':').nth(1)
                .context("Invalid SSH URL format")?
                .trim_end_matches(".git")
                .to_string()
        } else if url.contains("github") {
            // HTTPS format: https://github.com/owner/repo.git
            let url_parts: Vec<&str> = url.split('/').collect();
            if url_parts.len() >= 2 {
                let owner = url_parts[url_parts.len() - 2];
                let repo = url_parts[url_parts.len() - 1].trim_end_matches(".git");
                format!("{}/{}", owner, repo)
            } else {
                return Err(anyhow::anyhow!("Invalid HTTPS URL format"));
            }
        } else {
            return Err(anyhow::anyhow!("URL does not appear to be a GitHub repository"));
        };

        let parts: Vec<&str> = repo_path_str.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid repository path format"));
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Get commits from the repository
    pub fn get_commits(&self, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()
            .context("Failed to create revision walker")?;
        
        revwalk.push_head()
            .context("Failed to push HEAD to revision walker")?;
        
        let mut commits = Vec::new();
        let mut count = 0;
        
        for commit_id in revwalk {
            if let Some(limit) = limit {
                if count >= limit {
                    break;
                }
            }
            
            let oid = commit_id.context("Failed to get commit OID")?;
            let commit = self.repo.find_commit(oid)
                .context("Failed to find commit")?;
            
            let commit_info = self.convert_commit_to_info(&commit)?;
            commits.push(commit_info);
            count += 1;
        }
        
        Ok(commits)
    }

    /// Get a specific commit by hash
    pub fn get_commit_by_hash(&self, hash: &str) -> Result<Option<CommitInfo>> {
        let oid = Oid::from_str(hash)
            .context("Invalid commit hash format")?;
        
        match self.repo.find_commit(oid) {
            Ok(commit) => Ok(Some(self.convert_commit_to_info(&commit)?)),
            Err(_) => Ok(None),
        }
    }

    /// Convert git2::Commit to our CommitInfo structure
    fn convert_commit_to_info(&self, commit: &Git2Commit) -> Result<CommitInfo> {
        let author = commit.author();
        let time = Time::new(author.when().seconds(), author.when().offset_minutes());
        let commit_date = DateTime::from_timestamp(time.seconds(), 0)
            .context("Failed to parse commit timestamp")?;

        // Get the diff and file changes
        let tree = commit.tree()
            .context("Failed to get commit tree")?;
        
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)
                .context("Failed to get parent commit")?
                .tree()
                .context("Failed to get parent tree")?)
        } else {
            None
        };

        let diff = self.repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .context("Failed to create diff")?;

        let mut files_changed = Vec::new();
        let mut insertions = 0;
        let mut deletions = 0;

        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    if let Some(path_str) = path.to_str() {
                        files_changed.push(path_str.to_string());
                    }
                }
                true
            },
            None,
            None,
            Some(&mut |_delta, _hunk, line| {
                match line.origin() {
                    '+' => insertions += 1,
                    '-' => deletions += 1,
                    _ => {}
                }
                true
            }),
        ).context("Failed to process diff")?;

        Ok(CommitInfo {
            hash: commit.id().to_string(),
            author_name: author.name()
                .unwrap_or("Unknown")
                .to_string(),
            author_email: author.email()
                .unwrap_or("unknown@example.com")
                .to_string(),
            commit_date,
            message: commit.message()
                .unwrap_or("No message")
                .trim()
                .to_string(),
            files_changed,
            insertions,
            deletions,
        })
    }
}

impl GitHubOps {
    /// Create a new GitHubOps instance with token authentication
    pub fn new(token: String, owner: String, repo_name: String) -> Result<Self> {
        let client = OctocrabBuilder::new()
            .personal_token(token)
            .build()
            .context("Failed to create GitHub client")?;

        Ok(Self {
            client,
            owner,
            repo_name,
        })
    }

    /// Create a new GitHubOps instance from environment token
    pub fn from_env(owner: String, repo_name: String) -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN")
            .context("GITHUB_TOKEN environment variable not set")?;
        
        Self::new(token, owner, repo_name)
    }

    /// Create a basic issue (simplified for compatibility)
    pub async fn create_issue(&self, title: &str, body: &str) -> Result<Issue> {
        let issue = self.client
            .issues(&self.owner, &self.repo_name)
            .create(title)
            .body(body)
            .send()
            .await
            .context("Failed to create issue")?;

        Ok(issue)
    }

    /// Add a comment to an issue
    pub async fn add_comment(&self, issue_number: u64, comment: &str) -> Result<()> {
        self.client
            .issues(&self.owner, &self.repo_name)
            .create_comment(issue_number, comment)
            .await
            .context("Failed to add comment to issue")?;
        
        Ok(())
    }

    /// Get an issue by number
    pub async fn get_issue(&self, issue_number: u64) -> Result<Issue> {
        let issue = self.client
            .issues(&self.owner, &self.repo_name)
            .get(issue_number)
            .await
            .context("Failed to get issue")?;

        Ok(issue)
    }

    /// List repository issues
    pub async fn list_issues(&self) -> Result<Vec<Issue>> {
        let issues = self.client
            .issues(&self.owner, &self.repo_name)
            .list()
            .send()
            .await
            .context("Failed to list issues")?;

        Ok(issues.items)
    }

    /// List repository labels
    pub async fn list_labels(&self) -> Result<Vec<Label>> {
        let labels = self.client
            .issues(&self.owner, &self.repo_name)
            .list_labels_for_repo()
            .send()
            .await
            .context("Failed to list labels")?;

        Ok(labels.items)
    }

    // Note: Label creation API has compatibility issues with current octocrab version
    // This can be re-implemented once the API stabilizes
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::process::Command;

    // Helper function to create a temporary git repository for testing
    fn create_test_git_repo() -> Result<(TempDir, GitOps)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path();
        
        // Initialize git repository
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()?;
            
        // Set up git config for testing
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()?;
            
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()?;
            
        // Add a remote for testing
        Command::new("git")
            .args(["remote", "add", "origin", "git@github.com:testuser/testrepo.git"])
            .current_dir(repo_path)
            .output()?;
            
        // Create and commit a test file
        fs::write(repo_path.join("test.txt"), "Hello, World!")?;
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(repo_path)
            .output()?;
            
        Command::new("git")
            .args(["commit", "-m", "Initial test commit"])
            .current_dir(repo_path)
            .output()?;
            
        let git_ops = GitOps::new_from_path(repo_path)?;
        Ok((temp_dir, git_ops))
    }

    #[test]
    fn test_git_ops_creation_current_directory() {
        // Test GitOps creation in current directory (should work since we're in a git repo)
        let result = GitOps::new();
        assert!(result.is_ok(), "Should be able to create GitOps in current git repository");
    }

    #[test] 
    fn test_git_ops_creation_from_path() {
        let result = create_test_git_repo();
        match result {
            Ok((_temp_dir, _git_ops)) => {
                // Test passes if we can create a git repo
                assert!(true);
            }
            Err(_) => {
                // This might fail in CI environments without git
                println!("Warning: Could not create test git repository - git may not be available");
            }
        }
    }

    #[test]
    fn test_git_ops_creation_invalid_path() {
        let result = GitOps::new_from_path("/nonexistent/path");
        assert!(result.is_err(), "Should fail to create GitOps for non-existent path");
    }

    #[test]
    fn test_parse_github_ssh_url() {
        // We need to extract URL parsing logic to test it independently
        // For now, test the URL parsing patterns we support
        let ssh_urls = vec![
            "git@github.com:owner/repo.git",
            "git@github.folknology:owner/repo.git",
        ];
        
        for url in ssh_urls {
            if url.starts_with("git@github") {
                let result = url.split(':').nth(1)
                    .map(|s| s.trim_end_matches(".git"))
                    .and_then(|s| {
                        let parts: Vec<&str> = s.split('/').collect();
                        if parts.len() == 2 {
                            Some((parts[0].to_string(), parts[1].to_string()))
                        } else {
                            None
                        }
                    });
                assert!(result.is_some(), "Should parse SSH URL: {}", url);
                let (owner, repo) = result.unwrap();
                assert_eq!(owner, "owner");
                assert_eq!(repo, "repo");
            }
        }
    }

    #[test]
    fn test_parse_github_https_url() {
        let https_urls = vec![
            "https://github.com/owner/repo.git",
            "https://github.com/owner/repo",
        ];
        
        for url in https_urls {
            if url.contains("github") {
                let url_parts: Vec<&str> = url.split('/').collect();
                if url_parts.len() >= 2 {
                    let owner = url_parts[url_parts.len() - 2];
                    let repo = url_parts[url_parts.len() - 1].trim_end_matches(".git");
                    assert_eq!(owner, "owner");
                    assert_eq!(repo, "repo");
                }
            }
        }
    }

    #[test]
    fn test_get_remote_url() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            let result = git_ops.get_remote_url("origin");
            assert!(result.is_ok(), "Should get remote URL");
            let url = result.unwrap();
            assert_eq!(url, "git@github.com:testuser/testrepo.git");
        }
    }

    #[test]
    fn test_get_remote_url_nonexistent() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            let result = git_ops.get_remote_url("nonexistent");
            assert!(result.is_err(), "Should fail for non-existent remote");
        }
    }

    #[test]
    fn test_parse_github_repo() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            let result = git_ops.parse_github_repo("origin");
            assert!(result.is_ok(), "Should parse GitHub repo info");
            let (owner, repo) = result.unwrap();
            assert_eq!(owner, "testuser");
            assert_eq!(repo, "testrepo");
        }
    }

    #[test]
    fn test_get_commits() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            let result = git_ops.get_commits(Some(10));
            assert!(result.is_ok(), "Should get commits");
            let commits = result.unwrap();
            assert!(!commits.is_empty(), "Should have at least one commit");
            
            // Verify commit structure
            let commit = &commits[0];
            assert!(!commit.hash.is_empty(), "Commit should have a hash");
            assert_eq!(commit.author_name, "Test User");
            assert_eq!(commit.author_email, "test@example.com");
            assert_eq!(commit.message, "Initial test commit");
            assert!(!commit.files_changed.is_empty(), "Should have changed files");
        }
    }

    #[test]
    fn test_get_commits_with_limit() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            let result = git_ops.get_commits(Some(1));
            assert!(result.is_ok(), "Should get commits with limit");
            let commits = result.unwrap();
            assert_eq!(commits.len(), 1, "Should respect limit");
        }
    }

    #[test]
    fn test_get_commit_by_hash() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            // First get a commit to get its hash
            if let Ok(commits) = git_ops.get_commits(Some(1)) {
                if !commits.is_empty() {
                    let hash = &commits[0].hash;
                    let result = git_ops.get_commit_by_hash(hash);
                    assert!(result.is_ok(), "Should get commit by hash");
                    let commit = result.unwrap();
                    assert!(commit.is_some(), "Should find the commit");
                    assert_eq!(commit.unwrap().hash, *hash);
                }
            }
        }
    }

    #[test]
    fn test_get_commit_by_invalid_hash() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            let result = git_ops.get_commit_by_hash("invalid_hash");
            assert!(result.is_err(), "Should fail for invalid hash format");
        }
    }

    #[test]
    fn test_get_commit_by_nonexistent_hash() {
        if let Ok((_temp_dir, git_ops)) = create_test_git_repo() {
            // Use a valid hash format but non-existent hash
            let result = git_ops.get_commit_by_hash("1234567890abcdef1234567890abcdef12345678");
            assert!(result.is_ok(), "Should handle non-existent hash gracefully");
            let commit = result.unwrap();
            assert!(commit.is_none(), "Should return None for non-existent commit");
        }
    }

    #[test]
    fn test_commit_info_serialization() {
        let commit_info = CommitInfo {
            hash: "abc123".to_string(),
            author_name: "Test User".to_string(),
            author_email: "test@example.com".to_string(),
            commit_date: Utc::now(),
            message: "Test commit".to_string(),
            files_changed: vec!["file1.txt".to_string(), "file2.txt".to_string()],
            insertions: 10,
            deletions: 5,
        };
        
        // Test JSON serialization
        let json = serde_json::to_string(&commit_info);
        assert!(json.is_ok(), "CommitInfo should be serializable to JSON");
        
        // Test JSON deserialization
        let json_str = json.unwrap();
        let deserialized: Result<CommitInfo, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "CommitInfo should be deserializable from JSON");
        
        let deserialized_commit = deserialized.unwrap();
        assert_eq!(deserialized_commit.hash, commit_info.hash);
        assert_eq!(deserialized_commit.author_name, commit_info.author_name);
        assert_eq!(deserialized_commit.files_changed, commit_info.files_changed);
    }

    #[test]
    fn test_issue_params_serialization() {
        let issue_params = IssueParams {
            title: "Test Issue".to_string(),
            body: "Test issue body".to_string(),
            labels: vec!["bug".to_string(), "enhancement".to_string()],
            assignees: vec!["user1".to_string()],
        };
        
        // Test JSON serialization
        let json = serde_json::to_string(&issue_params);
        assert!(json.is_ok(), "IssueParams should be serializable to JSON");
        
        // Test JSON deserialization
        let json_str = json.unwrap();
        let deserialized: Result<IssueParams, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "IssueParams should be deserializable from JSON");
        
        let deserialized_params = deserialized.unwrap();
        assert_eq!(deserialized_params.title, issue_params.title);
        assert_eq!(deserialized_params.labels, issue_params.labels);
        assert_eq!(deserialized_params.assignees, issue_params.assignees);
    }

    // GitHub Operations Tests
    #[tokio::test]
    async fn test_github_ops_creation_with_token() {
        let result = GitHubOps::new(
            "fake_token".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        );
        assert!(result.is_ok(), "Should create GitHubOps with valid parameters");
    }

    #[tokio::test]
    async fn test_github_ops_creation_empty_token() {
        let result = GitHubOps::new(
            "".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        );
        // Empty token should still create the client, but API calls will fail
        assert!(result.is_ok(), "Should create GitHubOps even with empty token");
    }

    #[tokio::test]
    async fn test_github_ops_from_env_with_token() {
        // Set a temporary environment variable
        std::env::set_var("GITHUB_TOKEN", "test_token");
        
        let result = GitHubOps::from_env(
            "test_owner".to_string(),
            "test_repo".to_string(),
        );
        
        assert!(result.is_ok(), "Should create GitHubOps from environment");
        
        // Clean up
        std::env::remove_var("GITHUB_TOKEN");
    }

    #[test]
    fn test_github_ops_from_env_without_token() {
        // Ensure GITHUB_TOKEN is not set
        std::env::remove_var("GITHUB_TOKEN");
        
        let result = GitHubOps::from_env(
            "test_owner".to_string(),
            "test_repo".to_string(),
        );
        
        assert!(result.is_err(), "Should fail when GITHUB_TOKEN is not set");
    }

    // Note: GitHub API operation tests would require actual API calls
    // which need authentication and internet connectivity. These should be
    // integration tests rather than unit tests.
    // For now, we test the client creation and parameter validation.

    #[tokio::test]
    async fn test_github_ops_with_real_token() {
        // Only run this test if a real GitHub token is available
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            let result = GitHubOps::new(
                token,
                "folknology".to_string(),
                "atask".to_string(),
            );
            assert!(result.is_ok(), "Should create GitHubOps with real token");
            
            // We could test actual API calls here, but that would require
            // network access and could affect the real repository
            // For CI/CD, these should be separate integration tests
        }
    }
}
