use crate::db::{Issue, IssuePriority, IssueStatus};
use anyhow::Result;
use log::{debug, info};

/// Formats issues as a table for CLI display
pub struct TableFormatter;

impl TableFormatter {
    /// Format issues as a table string
    pub fn format_issues(issues: &[Issue]) -> Result<String> {
        debug!("Formatting {} issues for table display", issues.len());

        if issues.is_empty() {
            info!("No issues to display");
            return Ok("No issues found".to_string());
        }

        let mut output = String::new();

        // Add summary header
        output.push_str(&format!("Issues Overview ({} total)\n\n", issues.len()));

        // Headers with better spacing and alignment
        output.push_str("ID    | Title                          | Status      | Priority | Assignee     | Labels\n");
        output.push_str("------|--------------------------------|-------------|----------|--------------|--------\n");

        // Rows with enhanced information
        for issue in issues {
            let id = issue.id.unwrap_or(0);
            let title = Self::truncate_title(&issue.title, 30);
            let status = Self::format_status(&issue.status);
            let priority = Self::format_priority(&issue.priority);
            let assignee = Self::format_assignee(&issue.assignee);
            let labels = Self::format_labels(&issue.labels);

            output.push_str(&format!(
                "#{:<4} | {:<30} | {:<11} | {:<8} | {:<12} | {}\n",
                id, title, status, priority, assignee, labels
            ));
        }

        info!("Formatted table with {} issues", issues.len());
        Ok(output)
    }

    /// Format issues with filtering options
    pub fn format_issues_filtered(
        issues: &[Issue],
        status_filter: Option<IssueStatus>,
        priority_filter: Option<IssuePriority>,
    ) -> Result<String> {
        let filtered_issues: Vec<&Issue> = issues
            .iter()
            .filter(|issue| {
                let status_match = status_filter
                    .as_ref()
                    .map(|filter| {
                        std::mem::discriminant(&issue.status) == std::mem::discriminant(filter)
                    })
                    .unwrap_or(true);

                let priority_match = priority_filter
                    .as_ref()
                    .map(|filter| {
                        std::mem::discriminant(&issue.priority) == std::mem::discriminant(filter)
                    })
                    .unwrap_or(true);

                status_match && priority_match
            })
            .collect();

        let owned_issues: Vec<Issue> = filtered_issues.into_iter().cloned().collect();
        Self::format_issues(&owned_issues)
    }

    /// Format a compact table view
    pub fn format_issues_compact(issues: &[Issue]) -> Result<String> {
        if issues.is_empty() {
            return Ok("No issues found".to_string());
        }

        let mut output = String::new();
        output.push_str("ID | Title | Status\n");
        output.push_str("---|-------|-------\n");

        for issue in issues {
            let id = issue.id.unwrap_or(0);
            let title = Self::truncate_title(&issue.title, 20);
            let status = issue.status.to_string();

            output.push_str(&format!("#{} | {} | {}\n", id, title, status));
        }

        Ok(output)
    }

    fn truncate_title(title: &str, max_len: usize) -> String {
        if title.len() <= max_len {
            title.to_string()
        } else {
            format!("{}...", &title[..max_len - 3])
        }
    }

    fn format_status(status: &IssueStatus) -> String {
        match status {
            IssueStatus::Open => "open".to_string(),
            IssueStatus::InProgress => "in_progress".to_string(),
            IssueStatus::Resolved => "resolved".to_string(),
            IssueStatus::Closed => "closed".to_string(),
        }
    }

    fn format_priority(priority: &IssuePriority) -> String {
        match priority {
            IssuePriority::Low => "low".to_string(),
            IssuePriority::Medium => "medium".to_string(),
            IssuePriority::High => "high".to_string(),
            IssuePriority::Critical => "critical".to_string(),
        }
    }

    fn format_assignee(assignee: &Option<String>) -> String {
        assignee
            .as_ref()
            .map(|a| Self::truncate_title(a, 12))
            .unwrap_or_else(|| "unassigned".to_string())
    }

    fn format_labels(labels: &[String]) -> String {
        if labels.is_empty() {
            "none".to_string()
        } else if labels.len() == 1 {
            labels[0].clone()
        } else {
            format!("{} (+{})", labels[0], labels.len() - 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_issue(
        id: i64,
        title: &str,
        status: IssueStatus,
        priority: IssuePriority,
    ) -> Issue {
        Issue {
            id: Some(id),
            title: title.to_string(),
            description: Some("Test description".to_string()),
            status,
            priority,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            assignee: Some("test-user".to_string()),
            labels: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_format_empty_issues() {
        let issues = vec![];
        let result = TableFormatter::format_issues(&issues);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("No issues found"));
    }

    #[test]
    fn test_format_single_issue() {
        let issues = vec![create_test_issue(
            1,
            "Test Issue",
            IssueStatus::Open,
            IssuePriority::High,
        )];

        let result = TableFormatter::format_issues(&issues);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Test Issue"));
        assert!(output.contains("open"));
        assert!(output.contains("high"));
        assert!(output.contains("#1"));
    }

    #[test]
    fn test_format_multiple_issues() {
        let issues = vec![
            create_test_issue(1, "First Issue", IssueStatus::Open, IssuePriority::High),
            create_test_issue(
                2,
                "Second Issue",
                IssueStatus::InProgress,
                IssuePriority::Medium,
            ),
            create_test_issue(3, "Third Issue", IssueStatus::Closed, IssuePriority::Low),
        ];

        let result = TableFormatter::format_issues(&issues);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("First Issue"));
        assert!(output.contains("Second Issue"));
        assert!(output.contains("Third Issue"));
        assert!(output.contains("#1"));
        assert!(output.contains("#2"));
        assert!(output.contains("#3"));
    }

    #[test]
    fn test_format_issues_with_status_filter() {
        let issues = vec![
            create_test_issue(1, "Open Issue", IssueStatus::Open, IssuePriority::High),
            create_test_issue(
                2,
                "Closed Issue",
                IssueStatus::Closed,
                IssuePriority::Medium,
            ),
        ];

        let result = TableFormatter::format_issues_filtered(&issues, Some(IssueStatus::Open), None);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Open Issue"));
        assert!(!output.contains("Closed Issue"));
    }

    #[test]
    fn test_format_issues_with_priority_filter() {
        let issues = vec![
            create_test_issue(1, "High Priority", IssueStatus::Open, IssuePriority::High),
            create_test_issue(2, "Low Priority", IssueStatus::Open, IssuePriority::Low),
        ];

        let result =
            TableFormatter::format_issues_filtered(&issues, None, Some(IssuePriority::High));
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("High Priority"));
        assert!(!output.contains("Low Priority"));
    }

    #[test]
    fn test_format_issues_with_combined_filters() {
        let issues = vec![
            create_test_issue(1, "Open High", IssueStatus::Open, IssuePriority::High),
            create_test_issue(2, "Closed High", IssueStatus::Closed, IssuePriority::High),
            create_test_issue(3, "Open Low", IssueStatus::Open, IssuePriority::Low),
        ];

        let result = TableFormatter::format_issues_filtered(
            &issues,
            Some(IssueStatus::Open),
            Some(IssuePriority::High),
        );
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Open High"));
        assert!(!output.contains("Closed High"));
        assert!(!output.contains("Open Low"));
    }

    #[test]
    fn test_format_compact_view() {
        let issues = vec![create_test_issue(
            1,
            "Test Issue",
            IssueStatus::Open,
            IssuePriority::High,
        )];

        let result = TableFormatter::format_issues_compact(&issues);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Compact view should have shorter format
        assert!(output.len() < TableFormatter::format_issues(&issues).unwrap().len());
    }

    #[test]
    fn test_table_headers_present() {
        let issues = vec![create_test_issue(
            1,
            "Test Issue",
            IssueStatus::Open,
            IssuePriority::High,
        )];

        let result = TableFormatter::format_issues(&issues);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("ID"));
        assert!(output.contains("Title"));
        assert!(output.contains("Status"));
        assert!(output.contains("Priority"));
    }

    #[test]
    fn test_issue_truncation_for_long_titles() {
        let issues = vec![
            create_test_issue(1, "This is a very long issue title that should be truncated in the table view to maintain readability", IssueStatus::Open, IssuePriority::High)
        ];

        let result = TableFormatter::format_issues(&issues);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should contain truncated version
        assert!(output.contains("This is a very long"));
        assert!(output.contains("..."));
    }
}
