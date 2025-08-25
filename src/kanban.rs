use chrono::{DateTime, Utc};
use octocrab::models::issues::Issue;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::git_ops::GitHubOps;

/// Represents a Kanban board with multiple columns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanBoard {
    pub columns: Vec<KanbanColumn>,
    pub title: String,
    pub last_updated: DateTime<Utc>,
}

/// Represents a column in the Kanban board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanColumn {
    pub id: String,
    pub title: String,
    pub label_name: String,  // GitHub label that maps to this column
    pub cards: Vec<KanbanCard>,
    pub color: String,  // CSS color for the column
}

/// Represents an issue card in a Kanban column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanCard {
    pub issue_number: u64,
    pub title: String,
    pub body: Option<String>,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comments_count: u32,
}

/// Priority levels for issues
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

impl KanbanBoard {
    /// Create a new empty Kanban board with default columns
    pub fn new(title: String) -> Self {
        let columns = vec![
            KanbanColumn::new("preparing".to_string(), "Preparing".to_string(), "Preparing".to_string(), "#fef3c7".to_string()),
            KanbanColumn::new("progressing".to_string(), "Progressing".to_string(), "Progressing".to_string(), "#bfdbfe".to_string()),
            KanbanColumn::new("done".to_string(), "Done".to_string(), "Done".to_string(), "#bbf7d0".to_string()),
            KanbanColumn::new("backlog".to_string(), "Backlog".to_string(), "".to_string(), "#f3f4f6".to_string()),
        ];

        Self {
            columns,
            title,
            last_updated: Utc::now(),
        }
    }

    /// Get total number of cards across all columns
    pub fn total_cards(&self) -> usize {
        self.columns.iter().map(|col| col.cards.len()).sum()
    }
}

impl KanbanColumn {
    /// Create a new Kanban column
    pub fn new(id: String, title: String, label_name: String, color: String) -> Self {
        Self {
            id,
            title,
            label_name,
            cards: Vec::new(),
            color,
        }
    }

    /// Add a card to this column
    pub fn add_card(&mut self, card: KanbanCard) {
        self.cards.push(card);
    }

    /// Remove a card by issue number
    pub fn remove_card(&mut self, issue_number: u64) -> Option<KanbanCard> {
        if let Some(pos) = self.cards.iter().position(|card| card.issue_number == issue_number) {
            Some(self.cards.remove(pos))
        } else {
            None
        }
    }
}

impl KanbanCard {
    /// Create a new Kanban card from a GitHub Issue
    pub fn from_github_issue(issue: &Issue) -> Self {
        Self {
            issue_number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            assignee: issue.assignee.as_ref().map(|a| a.login.clone()),
            labels: issue.labels.iter().map(|label| label.name.clone()).collect(),
            priority: Priority::default(), // We'll determine this from labels later
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            comments_count: issue.comments as u32,
        }
    }

    /// Determine priority from labels
    pub fn set_priority_from_labels(&mut self) {
        self.priority = if self.labels.iter().any(|l| l.contains("critical") || l.contains("Critical")) {
            Priority::Critical
        } else if self.labels.iter().any(|l| l.contains("high") || l.contains("High")) {
            Priority::High
        } else if self.labels.iter().any(|l| l.contains("low") || l.contains("Low")) {
            Priority::Low
        } else {
            Priority::Medium
        };
    }
}

/// Kanban service layer for managing boards and GitHub integration
pub struct KanbanService {
    github_ops: GitHubOps,
}

impl KanbanService {
    /// Create a new KanbanService with GitHub operations
    pub fn new(github_ops: GitHubOps) -> Self {
        Self { github_ops }
    }

    /// Fetch all issues from GitHub and organize them into a Kanban board
    pub async fn fetch_board(&self, board_title: String) -> Result<KanbanBoard> {
        let mut board = KanbanBoard::new(board_title);
        
        // Get all issues from GitHub
        let all_issues = self.github_ops.list_issues().await?;
        
        // Organize issues into columns based on their labels
        for issue in all_issues {
            let mut card = KanbanCard::from_github_issue(&issue);
            card.set_priority_from_labels();
            
            // Determine which column this issue belongs to
            let mut placed = false;
            
            // Check workflow labels first (Preparing, Progressing, Done)
            for column in &mut board.columns {
                if column.id != "backlog" && !column.label_name.is_empty() {
                    if issue.labels.iter().any(|label| label.name == column.label_name) {
                        column.add_card(card.clone());
                        placed = true;
                        break;
                    }
                }
            }
            
            // If no workflow label found, put in backlog
            if !placed {
                if let Some(backlog_column) = board.columns.iter_mut().find(|col| col.id == "backlog") {
                    backlog_column.add_card(card);
                }
            }
        }
        
        board.last_updated = Utc::now();
        Ok(board)
    }

    /// Move an issue from one column to another by updating GitHub labels
    pub async fn move_issue(&self, issue_number: u64, from_column: &str, to_column: &str) -> Result<()> {
        // Remove the old label if it exists
        if !from_column.is_empty() && from_column != "backlog" {
            let _ = self.github_ops.remove_label_from_issue(issue_number, from_column).await;
        }
        
        // Add the new label if it's not backlog
        if !to_column.is_empty() && to_column != "backlog" {
            self.github_ops.add_label_to_issue(issue_number, to_column).await?;
        }
        
        Ok(())
    }

    /// Refresh a single column by fetching issues with specific labels
    pub async fn refresh_column(&self, column: &mut KanbanColumn) -> Result<()> {
        column.cards.clear();
        
        let issues = if column.id == "backlog" {
            self.get_backlog_issues().await?
        } else {
            self.get_issues_for_label(&column.label_name).await?
        };
        
        for issue in issues {
            let mut card = KanbanCard::from_github_issue(&issue);
            card.set_priority_from_labels();
            column.add_card(card);
        }
        
        Ok(())
    }

    /// Get issues for a specific label (used for column population)
    async fn get_issues_for_label(&self, label: &str) -> Result<Vec<Issue>> {
        let all_issues = self.github_ops.list_issues().await?;
        
        let filtered_issues = all_issues
            .into_iter()
            .filter(|issue| {
                issue.labels.iter().any(|issue_label| issue_label.name == label)
            })
            .collect();
            
        Ok(filtered_issues)
    }

    /// Get issues without workflow labels (backlog)
    async fn get_backlog_issues(&self) -> Result<Vec<Issue>> {
        let all_issues = self.github_ops.list_issues().await?;
        let workflow_labels = ["Preparing", "Progressing", "Done"];
        
        let backlog_issues = all_issues
            .into_iter()
            .filter(|issue| {
                // Issues without any workflow labels go to backlog
                !issue.labels.iter().any(|label| {
                    workflow_labels.contains(&label.name.as_str())
                })
            })
            .collect();
            
        Ok(backlog_issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_kanban_board_creation() {
        let board = KanbanBoard::new("Test Project".to_string());
        
        assert_eq!(board.title, "Test Project");
        assert_eq!(board.columns.len(), 4);
        assert_eq!(board.total_cards(), 0);
        
        // Check default columns
        let column_names: Vec<&str> = board.columns.iter().map(|col| col.title.as_str()).collect();
        assert_eq!(column_names, vec!["Preparing", "Progressing", "Done", "Backlog"]);
    }

    #[test]
    fn test_kanban_column_creation() {
        let column = KanbanColumn::new(
            "test".to_string(),
            "Test Column".to_string(), 
            "test-label".to_string(),
            "#ffffff".to_string()
        );
        
        assert_eq!(column.id, "test");
        assert_eq!(column.title, "Test Column");
        assert_eq!(column.label_name, "test-label");
        assert_eq!(column.color, "#ffffff");
        assert_eq!(column.cards.len(), 0);
    }

    #[test]
    fn test_kanban_card_priority_default() {
        let card = KanbanCard {
            issue_number: 1,
            title: "Test Issue".to_string(),
            body: None,
            assignee: None,
            labels: Vec::new(),
            priority: Priority::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            comments_count: 0,
        };
        
        assert_eq!(card.priority, Priority::Medium);
    }

    #[test]
    fn test_kanban_card_priority_from_labels() {
        let mut card = KanbanCard {
            issue_number: 1,
            title: "Critical Bug".to_string(),
            body: None,
            assignee: None,
            labels: vec!["bug".to_string(), "critical".to_string()],
            priority: Priority::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            comments_count: 0,
        };
        
        card.set_priority_from_labels();
        assert_eq!(card.priority, Priority::Critical);
        
        card.labels = vec!["enhancement".to_string(), "high".to_string()];
        card.set_priority_from_labels();
        assert_eq!(card.priority, Priority::High);
        
        card.labels = vec!["documentation".to_string(), "low".to_string()];
        card.set_priority_from_labels();
        assert_eq!(card.priority, Priority::Low);
    }

    #[test]
    fn test_column_add_remove_cards() {
        let mut column = KanbanColumn::new(
            "test".to_string(),
            "Test".to_string(),
            "test".to_string(),
            "#ffffff".to_string()
        );
        
        let card = KanbanCard {
            issue_number: 1,
            title: "Test Issue".to_string(),
            body: None,
            assignee: None,
            labels: Vec::new(),
            priority: Priority::Medium,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            comments_count: 0,
        };
        
        // Test adding card
        column.add_card(card.clone());
        assert_eq!(column.cards.len(), 1);
        
        // Test removing card
        let removed = column.remove_card(1);
        assert!(removed.is_some());
        assert_eq!(column.cards.len(), 0);
        
        // Test removing non-existent card
        let not_removed = column.remove_card(999);
        assert!(not_removed.is_none());
    }

    #[test]
    fn test_board_total_cards() {
        let mut board = KanbanBoard::new("Test".to_string());
        
        let card1 = KanbanCard {
            issue_number: 1,
            title: "Issue 1".to_string(),
            body: None,
            assignee: None,
            labels: Vec::new(),
            priority: Priority::Medium,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            comments_count: 0,
        };
        
        let card2 = KanbanCard {
            issue_number: 2,
            title: "Issue 2".to_string(),
            body: None,
            assignee: None,
            labels: Vec::new(),
            priority: Priority::High,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            comments_count: 0,
        };
        
        board.columns[0].add_card(card1);
        board.columns[1].add_card(card2);
        
        assert_eq!(board.total_cards(), 2);
    }

    // Note: GitHub Issue integration will be tested in integration tests
    // since octocrab::models::issues::Issue is non-exhaustive and can't be
    // created manually in unit tests

    #[test]
    fn test_serialization() {
        let board = KanbanBoard::new("Test Board".to_string());
        
        // Test JSON serialization
        let json = serde_json::to_string(&board);
        assert!(json.is_ok(), "KanbanBoard should be serializable to JSON");
        
        // Test JSON deserialization
        let json_str = json.unwrap();
        let deserialized: Result<KanbanBoard, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "KanbanBoard should be deserializable from JSON");
        
        let deserialized_board = deserialized.unwrap();
        assert_eq!(deserialized_board.title, board.title);
        assert_eq!(deserialized_board.columns.len(), board.columns.len());
    }

    // KanbanService Tests (GREEN phase - now implemented)
    #[tokio::test]
    async fn test_kanban_service_creation() {
        let github_ops = GitHubOps::new(
            "fake_token".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        ).unwrap();
        
        let _service = KanbanService::new(github_ops);
        // Service should be created successfully
        assert!(true);
    }

    // Note: These tests will fail without valid GitHub authentication
    // but they test the method signatures and basic functionality
    #[tokio::test]
    async fn test_fetch_board_method_exists() {
        let github_ops = GitHubOps::new(
            "fake_token".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        ).unwrap();
        
        let service = KanbanService::new(github_ops);
        
        // This will fail with authentication error, but method is implemented
        let result = service.fetch_board("Test Board".to_string()).await;
        assert!(result.is_err(), "Should fail with authentication error for fake token");
        // The error should not be "not yet implemented"
        let error_msg = result.unwrap_err().to_string();
        assert!(!error_msg.contains("not yet implemented"));
    }

    #[tokio::test]
    async fn test_move_issue_method_exists() {
        let github_ops = GitHubOps::new(
            "fake_token".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        ).unwrap();
        
        let service = KanbanService::new(github_ops);
        
        // This will fail with authentication error, but method is implemented
        let result = service.move_issue(1, "Preparing", "Progressing").await;
        assert!(result.is_err(), "Should fail with authentication error for fake token");
        // The error should not be "not yet implemented"
        let error_msg = result.unwrap_err().to_string();
        assert!(!error_msg.contains("not yet implemented"));
    }

    #[tokio::test]
    async fn test_refresh_column_method_exists() {
        let github_ops = GitHubOps::new(
            "fake_token".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        ).unwrap();
        
        let service = KanbanService::new(github_ops);
        let mut column = KanbanColumn::new(
            "test".to_string(),
            "Test".to_string(),
            "test-label".to_string(),
            "#ffffff".to_string()
        );
        
        // This will fail with authentication error, but method is implemented
        let result = service.refresh_column(&mut column).await;
        assert!(result.is_err(), "Should fail with authentication error for fake token");
        // The error should not be "not yet implemented"
        let error_msg = result.unwrap_err().to_string();
        assert!(!error_msg.contains("not yet implemented"));
    }
}
