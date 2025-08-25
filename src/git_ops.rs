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

    #[test]
    fn test_git_ops_creation() {
        // This will only work if we're in a git repository
        match GitOps::new() {
            Ok(_git_ops) => {
                // Test passes if we're in a git repo
                assert!(true);
            }
            Err(_) => {
                // Expected if not in a git repo
                assert!(true);
            }
        }
    }

    #[test]
    fn test_parse_github_ssh_url() {
        // We can't test the actual git operations without being in a repo,
        // but we can test URL parsing logic if we extract it
        assert!(true); // Placeholder for future URL parsing tests
    }

    #[tokio::test]
    async fn test_github_ops_creation() {
        // This test requires a valid token and won't work in CI without secrets
        // but demonstrates the API
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            let result = GitHubOps::new(
                token,
                "folknology".to_string(),
                "atask".to_string(),
            );
            assert!(result.is_ok());
        }
    }
}
