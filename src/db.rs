use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use libsql::{Builder, Connection, Database};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitCommit {
    pub id: Option<i64>,
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub commit_date: DateTime<Utc>,
    pub message: String,
    pub files_changed: Vec<String>,
    pub insertions: i32,
    pub deletions: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Issue {
    pub id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    pub priority: IssuePriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Label {
    pub id: Option<i64>,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IssueStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IssuePriority {
    Low,
    Medium,
    High,
    Critical,
}

impl ToString for IssueStatus {
    fn to_string(&self) -> String {
        match self {
            IssueStatus::Open => "open".to_string(),
            IssueStatus::InProgress => "in_progress".to_string(),
            IssueStatus::Resolved => "resolved".to_string(),
            IssueStatus::Closed => "closed".to_string(),
        }
    }
}

impl ToString for IssuePriority {
    fn to_string(&self) -> String {
        match self {
            IssuePriority::Low => "low".to_string(),
            IssuePriority::Medium => "medium".to_string(),
            IssuePriority::High => "high".to_string(),
            IssuePriority::Critical => "critical".to_string(),
        }
    }
}

impl std::str::FromStr for IssueStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "open" => Ok(IssueStatus::Open),
            "in_progress" => Ok(IssueStatus::InProgress),
            "resolved" => Ok(IssueStatus::Resolved),
            "closed" => Ok(IssueStatus::Closed),
            _ => Err(anyhow::anyhow!("Invalid issue status: {}", s)),
        }
    }
}

impl std::str::FromStr for IssuePriority {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "low" => Ok(IssuePriority::Low),
            "medium" => Ok(IssuePriority::Medium),
            "high" => Ok(IssuePriority::High),
            "critical" => Ok(IssuePriority::Critical),
            _ => Err(anyhow::anyhow!("Invalid issue priority: {}", s)),
        }
    }
}

pub struct TaskDatabase {
    #[allow(dead_code)]
    db: Database,
    conn: Connection,
}

impl TaskDatabase {
    pub async fn new(db_path: &str) -> Result<Self> {
        let db = Builder::new_local(db_path).build().await?;
        let conn = db.connect()?;
        
        let instance = Self { db, conn };
        instance.init_schema().await?;
        Ok(instance)
    }

    pub async fn in_memory() -> Result<Self> {
        let db = Builder::new_local(":memory:").build().await?;
        let conn = db.connect()?;
        
        let instance = Self { db, conn };
        instance.init_schema().await?;
        Ok(instance)
    }

    async fn init_schema(&self) -> Result<()> {
        // Create commits table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS commits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash TEXT UNIQUE NOT NULL,
                author_name TEXT NOT NULL,
                author_email TEXT NOT NULL,
                commit_date DATETIME NOT NULL,
                message TEXT NOT NULL,
                files_changed TEXT NOT NULL, -- JSON array
                insertions INTEGER DEFAULT 0,
                deletions INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        ).await?;

        // Create labels table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS labels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                color TEXT NOT NULL DEFAULT '#808080',
                description TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        ).await?;

        // Create issues table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS issues (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'open',
                priority TEXT NOT NULL DEFAULT 'medium',
                assignee TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        ).await?;

        // Create issue_labels junction table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS issue_labels (
                issue_id INTEGER NOT NULL,
                label_id INTEGER NOT NULL,
                PRIMARY KEY (issue_id, label_id),
                FOREIGN KEY (issue_id) REFERENCES issues (id) ON DELETE CASCADE,
                FOREIGN KEY (label_id) REFERENCES labels (id) ON DELETE CASCADE
            )",
            (),
        ).await?;

        // Create indexes
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_commits_hash ON commits(hash)",
            (),
        ).await?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_commits_date ON commits(commit_date)",
            (),
        ).await?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status)",
            (),
        ).await?;

        Ok(())
    }

    // CRUD operations for commits
    pub async fn insert_commit(&self, commit: &GitCommit) -> Result<i64> {
        let files_json = serde_json::to_string(&commit.files_changed)?;
        
        self.conn.execute(
            "INSERT INTO commits (hash, author_name, author_email, commit_date, message, files_changed, insertions, deletions)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            libsql::params![
                commit.hash.clone(),
                commit.author_name.clone(),
                commit.author_email.clone(),
                commit.commit_date.to_rfc3339(),
                commit.message.clone(),
                files_json,
                commit.insertions,
                commit.deletions
            ],
        ).await?;

        // Get the last insert rowid
        let mut rows = self.conn.query("SELECT last_insert_rowid()", ()).await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Err(anyhow::anyhow!("Failed to get last insert rowid"))
        }
    }

    pub async fn get_commit_by_hash(&self, hash: &str) -> Result<Option<GitCommit>> {
        let mut rows = self.conn.query(
            "SELECT id, hash, author_name, author_email, commit_date, message, files_changed, insertions, deletions
             FROM commits WHERE hash = ?",
            libsql::params![hash],
        ).await?;

        if let Some(row) = rows.next().await? {
            let files_json: String = row.get(6)?;
            let files_changed: Vec<String> = serde_json::from_str(&files_json)?;
            let commit_date: String = row.get(4)?;
            
            Ok(Some(GitCommit {
                id: Some(row.get(0)?),
                hash: row.get(1)?,
                author_name: row.get(2)?,
                author_email: row.get(3)?,
                commit_date: DateTime::parse_from_rfc3339(&commit_date)?.with_timezone(&Utc),
                message: row.get(5)?,
                files_changed,
                insertions: row.get(7)?,
                deletions: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_commits(&self) -> Result<Vec<GitCommit>> {
        let mut rows = self.conn.query(
            "SELECT id, hash, author_name, author_email, commit_date, message, files_changed, insertions, deletions
             FROM commits ORDER BY commit_date DESC",
            (),
        ).await?;

        let mut commits = Vec::new();
        while let Some(row) = rows.next().await? {
            let files_json: String = row.get(6)?;
            let files_changed: Vec<String> = serde_json::from_str(&files_json)?;
            let commit_date: String = row.get(4)?;
            
            commits.push(GitCommit {
                id: Some(row.get(0)?),
                hash: row.get(1)?,
                author_name: row.get(2)?,
                author_email: row.get(3)?,
                commit_date: DateTime::parse_from_rfc3339(&commit_date)?.with_timezone(&Utc),
                message: row.get(5)?,
                files_changed,
                insertions: row.get(7)?,
                deletions: row.get(8)?,
            });
        }

        Ok(commits)
    }

    // CRUD operations for labels
    pub async fn insert_label(&self, label: &Label) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO labels (name, color, description) VALUES (?, ?, ?)",
            libsql::params![label.name.clone(), label.color.clone(), label.description.clone()],
        ).await?;

        // Get the last insert rowid
        let mut rows = self.conn.query("SELECT last_insert_rowid()", ()).await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Err(anyhow::anyhow!("Failed to get last insert rowid"))
        }
    }

    pub async fn get_label_by_name(&self, name: &str) -> Result<Option<Label>> {
        let mut rows = self.conn.query(
            "SELECT id, name, color, description, created_at FROM labels WHERE name = ?",
            libsql::params![name],
        ).await?;

        if let Some(row) = rows.next().await? {
            let created_at: String = row.get(4)?;
            let parsed_date = if created_at.contains('T') {
                DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc)
            } else {
                // Handle SQLite datetime format
                DateTime::parse_from_str(&format!("{} +0000", created_at), "%Y-%m-%d %H:%M:%S %z")?
                    .with_timezone(&Utc)
            };
            
            Ok(Some(Label {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
                created_at: parsed_date,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_labels(&self) -> Result<Vec<Label>> {
        let mut rows = self.conn.query(
            "SELECT id, name, color, description, created_at FROM labels ORDER BY name",
            (),
        ).await?;

        let mut labels = Vec::new();
        while let Some(row) = rows.next().await? {
            let created_at: String = row.get(4)?;
            let parsed_date = if created_at.contains('T') {
                DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc)
            } else {
                // Handle SQLite datetime format
                DateTime::parse_from_str(&format!("{} +0000", created_at), "%Y-%m-%d %H:%M:%S %z")?
                    .with_timezone(&Utc)
            };
            
            labels.push(Label {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
                created_at: parsed_date,
            });
        }

        Ok(labels)
    }

    // CRUD operations for issues
    pub async fn insert_issue(&self, issue: &Issue) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO issues (title, description, status, priority, assignee, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![
                issue.title.clone(),
                issue.description.clone(),
                issue.status.to_string(),
                issue.priority.to_string(),
                issue.assignee.clone(),
                issue.updated_at.to_rfc3339()
            ],
        ).await?;

        // Get the last insert rowid
        let mut rows = self.conn.query("SELECT last_insert_rowid()", ()).await?;
        let issue_id: i64 = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(anyhow::anyhow!("Failed to get last insert rowid"));
        };

        // Insert label associations
        for label_name in &issue.labels {
            if let Some(label) = self.get_label_by_name(label_name).await? {
                if let Some(label_id) = label.id {
                    self.conn.execute(
                        "INSERT OR IGNORE INTO issue_labels (issue_id, label_id) VALUES (?, ?)",
                        libsql::params![issue_id, label_id],
                    ).await?;
                }
            }
        }

        Ok(issue_id)
    }

    pub async fn get_issue_by_id(&self, id: i64) -> Result<Option<Issue>> {
        let mut rows = self.conn.query(
            "SELECT id, title, description, status, priority, assignee, created_at, updated_at
             FROM issues WHERE id = ?",
            libsql::params![id],
        ).await?;

        if let Some(row) = rows.next().await? {
            let created_at: String = row.get(6)?;
            let updated_at: String = row.get(7)?;
            
            let parsed_created_at = if created_at.contains('T') {
                DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc)
            } else {
                // Handle SQLite datetime format
                DateTime::parse_from_str(&format!("{} +0000", created_at), "%Y-%m-%d %H:%M:%S %z")?
                    .with_timezone(&Utc)
            };
            
            let parsed_updated_at = if updated_at.contains('T') {
                DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc)
            } else {
                // Handle SQLite datetime format
                DateTime::parse_from_str(&format!("{} +0000", updated_at), "%Y-%m-%d %H:%M:%S %z")?
                    .with_timezone(&Utc)
            };
            
            // Get labels for this issue
            let labels = self.get_issue_labels(id).await?;
            
            Ok(Some(Issue {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get::<String>(3)?.parse()?,
                priority: row.get::<String>(4)?.parse()?,
                assignee: row.get(5)?,
                created_at: parsed_created_at,
                updated_at: parsed_updated_at,
                labels,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_issues(&self) -> Result<Vec<Issue>> {
        let mut rows = self.conn.query(
            "SELECT id, title, description, status, priority, assignee, created_at, updated_at
             FROM issues ORDER BY created_at DESC",
            (),
        ).await?;

        let mut issues = Vec::new();
        while let Some(row) = rows.next().await? {
            let issue_id: i64 = row.get(0)?;
            let created_at: String = row.get(6)?;
            let updated_at: String = row.get(7)?;
            
            let parsed_created_at = if created_at.contains('T') {
                DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc)
            } else {
                DateTime::parse_from_str(&format!("{} +0000", created_at), "%Y-%m-%d %H:%M:%S %z")?
                    .with_timezone(&Utc)
            };
            
            let parsed_updated_at = if updated_at.contains('T') {
                DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc)
            } else {
                DateTime::parse_from_str(&format!("{} +0000", updated_at), "%Y-%m-%d %H:%M:%S %z")?
                    .with_timezone(&Utc)
            };
            
            // Get labels for this issue
            let labels = self.get_issue_labels(issue_id).await?;
            
            issues.push(Issue {
                id: Some(issue_id),
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get::<String>(3)?.parse()?,
                priority: row.get::<String>(4)?.parse()?,
                assignee: row.get(5)?,
                created_at: parsed_created_at,
                updated_at: parsed_updated_at,
                labels,
            });
        }

        Ok(issues)
    }

    async fn get_issue_labels(&self, issue_id: i64) -> Result<Vec<String>> {
        let mut rows = self.conn.query(
            "SELECT l.name FROM labels l 
             JOIN issue_labels il ON l.id = il.label_id 
             WHERE il.issue_id = ?",
            libsql::params![issue_id],
        ).await?;

        let mut labels = Vec::new();
        while let Some(row) = rows.next().await? {
            labels.push(row.get(0)?);
        }

        Ok(labels)
    }

    pub async fn update_issue_status(&self, id: i64, status: IssueStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE issues SET status = ?, updated_at = ? WHERE id = ?",
            libsql::params![status.to_string(), Utc::now().to_rfc3339(), id],
        ).await?;

        Ok(())
    }

    pub async fn delete_issue(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM issues WHERE id = ?",
            libsql::params![id],
        ).await?;

        Ok(())
    }

    // Git integration functions
    pub async fn populate_from_git_history(&self, repo_path: Option<&str>) -> Result<usize> {
        let git_log_output = if let Some(path) = repo_path {
            Command::new("git")
                .current_dir(path)
                .args([
                    "log",
                    "--pretty=format:%H|%an|%ae|%ai|%s",
                    "--numstat",
                ])
                .output()?
        } else {
            Command::new("git")
                .args([
                    "log",
                    "--pretty=format:%H|%an|%ae|%ai|%s",
                    "--numstat",
                ])
                .output()?
        };

        if !git_log_output.status.success() {
            return Err(anyhow::anyhow!("Failed to get git log: {}", 
                String::from_utf8_lossy(&git_log_output.stderr)));
        }

        let output = String::from_utf8(git_log_output.stdout)?;
        let mut commits_inserted = 0;

        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Parse commit info line
            let parts: Vec<&str> = line.splitn(5, '|').collect();
            if parts.len() != 5 {
                i += 1;
                continue;
            }

            let hash = parts[0].to_string();
            let author_name = parts[1].to_string();
            let author_email = parts[2].to_string();
            let date_str = parts[3];
            let message = parts[4].to_string();

            let commit_date = DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S %z")
                .map_err(|e| anyhow::anyhow!("Failed to parse date '{}': {}", date_str, e))?
                .with_timezone(&Utc);

            // Parse file changes
            i += 1;
            let mut files_changed = Vec::new();
            let mut total_insertions = 0;
            let mut total_deletions = 0;

            while i < lines.len() {
                let stat_line = lines[i].trim();
                if stat_line.is_empty() {
                    break;
                }

                // Check if this is the next commit (starts with hash pattern)
                if stat_line.contains('|') && stat_line.len() > 40 {
                    break;
                }

                let parts: Vec<&str> = stat_line.split('\t').collect();
                if parts.len() >= 3 {
                    let insertions: i32 = parts[0].parse().unwrap_or(0);
                    let deletions: i32 = parts[1].parse().unwrap_or(0);
                    let filename = parts[2].to_string();

                    files_changed.push(filename);
                    total_insertions += insertions;
                    total_deletions += deletions;
                }
                i += 1;
            }

            let git_commit = GitCommit {
                id: None,
                hash,
                author_name,
                author_email,
                commit_date,
                message,
                files_changed,
                insertions: total_insertions,
                deletions: total_deletions,
            };

            // Check if commit already exists
            if self.get_commit_by_hash(&git_commit.hash).await?.is_none() {
                self.insert_commit(&git_commit).await?;
                commits_inserted += 1;
            }
        }

        Ok(commits_inserted)
    }

    pub async fn create_default_labels(&self) -> Result<()> {
        let default_labels = vec![
            ("bug", "#d73a4a", "Something isn't working"),
            ("enhancement", "#a2eeef", "New feature or request"),
            ("documentation", "#0075ca", "Improvements or additions to documentation"),
            ("good first issue", "#7057ff", "Good for newcomers"),
            ("help wanted", "#008672", "Extra attention is needed"),
            ("invalid", "#e4e669", "This doesn't seem right"),
            ("question", "#d876e3", "Further information is requested"),
            ("wontfix", "#ffffff", "This will not be worked on"),
        ];

        for (name, color, description) in default_labels {
            if self.get_label_by_name(name).await?.is_none() {
                let label = Label {
                    id: None,
                    name: name.to_string(),
                    color: color.to_string(),
                    description: Some(description.to_string()),
                    created_at: Utc::now(),
                };
                self.insert_label(&label).await?;
            }
        }

        Ok(())
    }

    /// Load GitHub issues using the gh CLI command
    pub async fn load_github_issues_via_cli(&self) -> Result<usize> {
        // First, check if gh CLI is available
        let gh_check = Command::new("gh")
            .args(["--version"])
            .output()
            .context("Failed to check if 'gh' CLI is installed")?;
        
        if !gh_check.status.success() {
            anyhow::bail!("GitHub CLI (gh) is not installed or not available");
        }
        
        // Run gh issue list command to get JSON output
        let output = Command::new("gh")
            .args([
                "issue", "list", 
                "--json", "number,title,body,state,labels,assignees,createdAt,updatedAt",
                "--limit", "100"  // Limit to avoid too many issues
            ])
            .output()
            .context("Failed to execute 'gh issue list' command")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("GitHub CLI command failed: {}", stderr);
        }
        
        let stdout = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in gh command output")?;
        
        let issues_json: Value = serde_json::from_str(&stdout)
            .context("Failed to parse JSON from gh command")?;
        
        let issues_array = issues_json.as_array()
            .context("Expected JSON array from gh issue list")?;
        
        let mut loaded_count = 0;
        
        for issue_value in issues_array {
            let issue_number = issue_value["number"].as_u64()
                .context("Issue number should be a number")?;
            
            // Check if we already have this issue
            if let Ok(existing_issues) = self.get_all_issues().await {
                if existing_issues.iter().any(|issue| {
                    issue.title.contains(&format!("#{}", issue_number)) ||
                    issue.description.as_ref().map(|d| d.contains(&format!("#{}", issue_number))).unwrap_or(false)
                }) {
                    continue; // Skip if already exists
                }
            }
            
            let title = issue_value["title"].as_str()
                .unwrap_or("Untitled Issue")
                .to_string();
            
            let body = issue_value["body"].as_str()
                .map(|s| s.to_string());
            
            let state = issue_value["state"].as_str()
                .unwrap_or("open");
            
            let status = match state.to_lowercase().as_str() {
                "closed" => IssueStatus::Closed,
                _ => IssueStatus::Open,
            };
            
            // Parse labels and create any missing labels in the database
            let labels = if let Some(labels_array) = issue_value["labels"].as_array() {
                let mut issue_labels = Vec::new();
                for label_obj in labels_array {
                    if let Some(label_name) = label_obj["name"].as_str() {
                        // Create the label if it doesn't exist
                        if self.get_label_by_name(label_name).await?.is_none() {
                            let label_color = label_obj["color"].as_str()
                                .unwrap_or("808080"); // Default gray color
                            let label_description = label_obj["description"].as_str()
                                .unwrap_or("");
                            
                            let new_label = Label {
                                id: None,
                                name: label_name.to_string(),
                                color: format!("#{}", label_color),
                                description: if label_description.is_empty() {
                                    None
                                } else {
                                    Some(label_description.to_string())
                                },
                                created_at: Utc::now(),
                            };
                            
                            if let Err(e) = self.insert_label(&new_label).await {
                                eprintln!("⚠️  Failed to create label '{}': {}", label_name, e);
                            }
                        }
                        issue_labels.push(label_name.to_string());
                    }
                }
                issue_labels
            } else {
                vec![]
            };
            
            // Parse assignee
            let assignee = if let Some(assignees_array) = issue_value["assignees"].as_array() {
                assignees_array.first()
                    .and_then(|assignee| assignee["login"].as_str())
                    .map(|s| s.to_string())
            } else {
                None
            };
            
            // Parse dates
            let created_at = if let Some(created_str) = issue_value["createdAt"].as_str() {
                DateTime::parse_from_rfc3339(created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                Utc::now()
            };
            
            let updated_at = if let Some(updated_str) = issue_value["updatedAt"].as_str() {
                DateTime::parse_from_rfc3339(updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                Utc::now()
            };
            
            // Determine priority from labels
            let priority = if labels.iter().any(|l| l.to_lowercase().contains("critical")) {
                IssuePriority::Critical
            } else if labels.iter().any(|l| l.to_lowercase().contains("high")) {
                IssuePriority::High
            } else if labels.iter().any(|l| l.to_lowercase().contains("low")) {
                IssuePriority::Low
            } else {
                IssuePriority::Medium
            };
            
            // Create the issue
            let issue = Issue {
                id: None,
                title: format!("#{}: {}", issue_number, title),
                description: body,
                status,
                priority,
                created_at,
                updated_at,
                assignee,
                labels,
            };
            
            // Insert into database
            match self.insert_issue(&issue).await {
                Ok(_) => {
                    loaded_count += 1;
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to insert issue #{}: {}", issue_number, e);
                }
            }
        }
        
        Ok(loaded_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    // Helper function to create a test database
    async fn create_test_db() -> Result<TaskDatabase> {
        TaskDatabase::in_memory().await
    }

    // Helper function to create a sample commit
    fn create_sample_commit() -> GitCommit {
        GitCommit {
            id: None,
            hash: "abc123def456".to_string(),
            author_name: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            commit_date: Utc::now(),
            message: "Test commit message".to_string(),
            files_changed: vec!["src/main.rs".to_string(), "README.md".to_string()],
            insertions: 10,
            deletions: 5,
        }
    }

    // Helper function to create a sample label
    fn create_sample_label() -> Label {
        Label {
            id: None,
            name: "test-label".to_string(),
            color: "#ff0000".to_string(),
            description: Some("A test label".to_string()),
            created_at: Utc::now(),
        }
    }

    // Helper function to create a sample issue
    fn create_sample_issue() -> Issue {
        Issue {
            id: None,
            title: "Test Issue".to_string(),
            description: Some("This is a test issue".to_string()),
            status: IssueStatus::Open,
            priority: IssuePriority::Medium,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            assignee: Some("test-user".to_string()),
            labels: vec!["test-label".to_string()],
        }
    }

    #[tokio::test]
    async fn test_database_initialization() {
        let db = create_test_db().await;
        assert!(db.is_ok(), "Database should initialize successfully");
    }

    #[tokio::test]
    async fn test_insert_and_retrieve_commit() {
        let db = create_test_db().await.unwrap();
        let commit = create_sample_commit();
        
        // Insert commit
        let commit_id = db.insert_commit(&commit).await.unwrap();
        assert!(commit_id > 0, "Commit ID should be positive");
        
        // Retrieve by hash
        let retrieved = db.get_commit_by_hash(&commit.hash).await.unwrap();
        assert!(retrieved.is_some(), "Commit should be retrievable by hash");
        
        let retrieved_commit = retrieved.unwrap();
        assert_eq!(retrieved_commit.hash, commit.hash);
        assert_eq!(retrieved_commit.author_name, commit.author_name);
        assert_eq!(retrieved_commit.author_email, commit.author_email);
        assert_eq!(retrieved_commit.message, commit.message);
        assert_eq!(retrieved_commit.files_changed, commit.files_changed);
        assert_eq!(retrieved_commit.insertions, commit.insertions);
        assert_eq!(retrieved_commit.deletions, commit.deletions);
    }

    #[tokio::test]
    async fn test_get_all_commits() {
        let db = create_test_db().await.unwrap();
        
        // Insert multiple commits
        let mut commit1 = create_sample_commit();
        commit1.hash = "hash1".to_string();
        let mut commit2 = create_sample_commit();
        commit2.hash = "hash2".to_string();
        
        db.insert_commit(&commit1).await.unwrap();
        db.insert_commit(&commit2).await.unwrap();
        
        // Retrieve all commits
        let commits = db.get_all_commits().await.unwrap();
        assert_eq!(commits.len(), 2, "Should retrieve all inserted commits");
    }

    #[tokio::test]
    async fn test_commit_hash_uniqueness() {
        let db = create_test_db().await.unwrap();
        let commit = create_sample_commit();
        
        // Insert same commit twice
        let result1 = db.insert_commit(&commit).await;
        let result2 = db.insert_commit(&commit).await;
        
        assert!(result1.is_ok(), "First insert should succeed");
        assert!(result2.is_err(), "Second insert with same hash should fail");
    }

    #[tokio::test]
    async fn test_insert_and_retrieve_label() {
        let db = create_test_db().await.unwrap();
        let label = create_sample_label();
        
        // Insert label
        let label_id = db.insert_label(&label).await.unwrap();
        assert!(label_id > 0, "Label ID should be positive");
        
        // Retrieve by name
        let retrieved = db.get_label_by_name(&label.name).await.unwrap();
        assert!(retrieved.is_some(), "Label should be retrievable by name");
        
        let retrieved_label = retrieved.unwrap();
        assert_eq!(retrieved_label.name, label.name);
        assert_eq!(retrieved_label.color, label.color);
        assert_eq!(retrieved_label.description, label.description);
    }

    #[tokio::test]
    async fn test_get_all_labels() {
        let db = create_test_db().await.unwrap();
        
        // Insert multiple labels
        let mut label1 = create_sample_label();
        label1.name = "label1".to_string();
        let mut label2 = create_sample_label();
        label2.name = "label2".to_string();
        
        db.insert_label(&label1).await.unwrap();
        db.insert_label(&label2).await.unwrap();
        
        // Retrieve all labels
        let labels = db.get_all_labels().await.unwrap();
        assert_eq!(labels.len(), 2, "Should retrieve all inserted labels");
    }

    #[tokio::test]
    async fn test_label_name_uniqueness() {
        let db = create_test_db().await.unwrap();
        let label = create_sample_label();
        
        // Insert same label twice
        let result1 = db.insert_label(&label).await;
        let result2 = db.insert_label(&label).await;
        
        assert!(result1.is_ok(), "First insert should succeed");
        assert!(result2.is_err(), "Second insert with same name should fail");
    }

    #[tokio::test]
    async fn test_create_default_labels() {
        let db = create_test_db().await.unwrap();
        
        // Create default labels
        db.create_default_labels().await.unwrap();
        
        // Verify default labels exist
        let bug_label = db.get_label_by_name("bug").await.unwrap();
        assert!(bug_label.is_some(), "Bug label should exist");
        assert_eq!(bug_label.unwrap().color, "#d73a4a");
        
        let enhancement_label = db.get_label_by_name("enhancement").await.unwrap();
        assert!(enhancement_label.is_some(), "Enhancement label should exist");
        assert_eq!(enhancement_label.unwrap().color, "#a2eeef");
        
        // Verify all 8 default labels
        let all_labels = db.get_all_labels().await.unwrap();
        assert_eq!(all_labels.len(), 8, "Should have 8 default labels");
    }

    #[tokio::test]
    async fn test_insert_and_retrieve_issue() {
        let db = create_test_db().await.unwrap();
        
        // Create label first
        let label = create_sample_label();
        db.insert_label(&label).await.unwrap();
        
        // Insert issue
        let issue = create_sample_issue();
        let issue_id = db.insert_issue(&issue).await.unwrap();
        assert!(issue_id > 0, "Issue ID should be positive");
        
        // Retrieve issue
        let retrieved = db.get_issue_by_id(issue_id).await.unwrap();
        assert!(retrieved.is_some(), "Issue should be retrievable by ID");
        
        let retrieved_issue = retrieved.unwrap();
        assert_eq!(retrieved_issue.title, issue.title);
        assert_eq!(retrieved_issue.description, issue.description);
        assert_eq!(retrieved_issue.status.to_string(), issue.status.to_string());
        assert_eq!(retrieved_issue.priority.to_string(), issue.priority.to_string());
        assert_eq!(retrieved_issue.assignee, issue.assignee);
        assert_eq!(retrieved_issue.labels, issue.labels);
    }

    #[tokio::test]
    async fn test_get_all_issues() {
        let db = create_test_db().await.unwrap();
        
        // Create label first
        let label = create_sample_label();
        db.insert_label(&label).await.unwrap();
        
        // Insert multiple issues
        let mut issue1 = create_sample_issue();
        issue1.title = "Issue 1".to_string();
        let mut issue2 = create_sample_issue();
        issue2.title = "Issue 2".to_string();
        
        db.insert_issue(&issue1).await.unwrap();
        db.insert_issue(&issue2).await.unwrap();
        
        // Retrieve all issues
        let issues = db.get_all_issues().await.unwrap();
        assert_eq!(issues.len(), 2, "Should retrieve all inserted issues");
    }

    #[tokio::test]
    async fn test_issue_label_association() {
        let db = create_test_db().await.unwrap();
        
        // Create multiple labels
        let mut label1 = create_sample_label();
        label1.name = "bug".to_string();
        let mut label2 = create_sample_label();
        label2.name = "enhancement".to_string();
        
        db.insert_label(&label1).await.unwrap();
        db.insert_label(&label2).await.unwrap();
        
        // Create issue with multiple labels
        let mut issue = create_sample_issue();
        issue.labels = vec!["bug".to_string(), "enhancement".to_string()];
        
        let issue_id = db.insert_issue(&issue).await.unwrap();
        
        // Retrieve and verify labels
        let retrieved_issue = db.get_issue_by_id(issue_id).await.unwrap().unwrap();
        assert_eq!(retrieved_issue.labels.len(), 2, "Issue should have 2 labels");
        assert!(retrieved_issue.labels.contains(&"bug".to_string()));
        assert!(retrieved_issue.labels.contains(&"enhancement".to_string()));
    }

    #[tokio::test]
    async fn test_update_issue_status() {
        let db = create_test_db().await.unwrap();
        
        // Create label first
        let label = create_sample_label();
        db.insert_label(&label).await.unwrap();
        
        // Insert issue
        let issue = create_sample_issue();
        let issue_id = db.insert_issue(&issue).await.unwrap();
        
        // Update status
        db.update_issue_status(issue_id, IssueStatus::InProgress).await.unwrap();
        
        // Verify update
        let updated_issue = db.get_issue_by_id(issue_id).await.unwrap().unwrap();
        assert_eq!(updated_issue.status.to_string(), "in_progress");
    }

    #[tokio::test]
    async fn test_delete_issue() {
        let db = create_test_db().await.unwrap();
        
        // Create label first
        let label = create_sample_label();
        db.insert_label(&label).await.unwrap();
        
        // Insert issue
        let issue = create_sample_issue();
        let issue_id = db.insert_issue(&issue).await.unwrap();
        
        // Verify issue exists
        let retrieved = db.get_issue_by_id(issue_id).await.unwrap();
        assert!(retrieved.is_some(), "Issue should exist before deletion");
        
        // Delete issue
        db.delete_issue(issue_id).await.unwrap();
        
        // Verify issue is deleted
        let deleted = db.get_issue_by_id(issue_id).await.unwrap();
        assert!(deleted.is_none(), "Issue should not exist after deletion");
    }

    #[tokio::test]
    async fn test_issue_status_string_conversion() {
        assert_eq!(IssueStatus::Open.to_string(), "open");
        assert_eq!(IssueStatus::InProgress.to_string(), "in_progress");
        assert_eq!(IssueStatus::Resolved.to_string(), "resolved");
        assert_eq!(IssueStatus::Closed.to_string(), "closed");
        
        assert!("open".parse::<IssueStatus>().is_ok());
        assert!("in_progress".parse::<IssueStatus>().is_ok());
        assert!("resolved".parse::<IssueStatus>().is_ok());
        assert!("closed".parse::<IssueStatus>().is_ok());
        assert!("invalid".parse::<IssueStatus>().is_err());
    }

    #[tokio::test]
    async fn test_issue_priority_string_conversion() {
        assert_eq!(IssuePriority::Low.to_string(), "low");
        assert_eq!(IssuePriority::Medium.to_string(), "medium");
        assert_eq!(IssuePriority::High.to_string(), "high");
        assert_eq!(IssuePriority::Critical.to_string(), "critical");
        
        assert!("low".parse::<IssuePriority>().is_ok());
        assert!("medium".parse::<IssuePriority>().is_ok());
        assert!("high".parse::<IssuePriority>().is_ok());
        assert!("critical".parse::<IssuePriority>().is_ok());
        assert!("invalid".parse::<IssuePriority>().is_err());
    }

    #[tokio::test]
    async fn test_git_commit_json_serialization() {
        let commit = create_sample_commit();
        let files_json = serde_json::to_string(&commit.files_changed).unwrap();
        let deserialized: Vec<String> = serde_json::from_str(&files_json).unwrap();
        assert_eq!(commit.files_changed, deserialized);
    }

    #[tokio::test]
    async fn test_database_schema_initialization() {
        let db = create_test_db().await.unwrap();
        
        // Test that we can perform operations on all tables
        // This implicitly tests that all tables were created correctly
        
        // Test commits table
        let commits = db.get_all_commits().await.unwrap();
        assert_eq!(commits.len(), 0);
        
        // Test labels table
        let labels = db.get_all_labels().await.unwrap();
        assert_eq!(labels.len(), 0);
        
        // Test issues table
        let issues = db.get_all_issues().await.unwrap();
        assert_eq!(issues.len(), 0);
    }

    #[tokio::test]
    async fn test_nonexistent_commit_retrieval() {
        let db = create_test_db().await.unwrap();
        let result = db.get_commit_by_hash("nonexistent").await.unwrap();
        assert!(result.is_none(), "Should return None for nonexistent commit");
    }

    #[tokio::test]
    async fn test_nonexistent_label_retrieval() {
        let db = create_test_db().await.unwrap();
        let result = db.get_label_by_name("nonexistent").await.unwrap();
        assert!(result.is_none(), "Should return None for nonexistent label");
    }

    #[tokio::test]
    async fn test_nonexistent_issue_retrieval() {
        let db = create_test_db().await.unwrap();
        let result = db.get_issue_by_id(999).await.unwrap();
        assert!(result.is_none(), "Should return None for nonexistent issue");
    }

    #[tokio::test]
    async fn test_issue_without_labels() {
        let db = create_test_db().await.unwrap();
        
        // Create issue without labels
        let mut issue = create_sample_issue();
        issue.labels = vec![];
        
        let issue_id = db.insert_issue(&issue).await.unwrap();
        let retrieved = db.get_issue_by_id(issue_id).await.unwrap().unwrap();
        
        assert_eq!(retrieved.labels.len(), 0, "Issue should have no labels");
    }

    #[tokio::test]
    async fn test_issue_with_nonexistent_labels() {
        let db = create_test_db().await.unwrap();
        
        // Create issue with non-existent label
        let mut issue = create_sample_issue();
        issue.labels = vec!["nonexistent-label".to_string()];
        
        let issue_id = db.insert_issue(&issue).await.unwrap();
        let retrieved = db.get_issue_by_id(issue_id).await.unwrap().unwrap();
        
        // Should succeed but have no labels since the label doesn't exist
        assert_eq!(retrieved.labels.len(), 0, "Issue should have no labels when referenced labels don't exist");
    }
}

