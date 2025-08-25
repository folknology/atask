use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{Builder, Connection, Database};
use serde::{Deserialize, Serialize};
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
            
            // Get labels for this issue
            let labels = self.get_issue_labels(id).await?;
            
            Ok(Some(Issue {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get::<String>(3)?.parse()?,
                priority: row.get::<String>(4)?.parse()?,
                assignee: row.get(5)?,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
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
}
