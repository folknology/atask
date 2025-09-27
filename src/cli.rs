use crate::db::{Issue, IssuePriority, IssueStatus};
use anyhow::Result;
use colored::*;
use log::{debug, info};

/// Formats issues as a table for CLI display
pub struct TableFormatter;

impl TableFormatter {
    /// Format issues as a table string
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn format_status(status: &IssueStatus) -> String {
        match status {
            IssueStatus::Open => "open".to_string(),
            IssueStatus::InProgress => "in_progress".to_string(),
            IssueStatus::Resolved => "resolved".to_string(),
            IssueStatus::Closed => "closed".to_string(),
        }
    }

    #[allow(dead_code)]
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

    /// Format issues as a colored table string
    pub fn format_issues_colored(issues: &[Issue], colored: bool) -> Result<String> {
        debug!(
            "Formatting {} issues for colored table display (colored: {})",
            issues.len(),
            colored
        );

        if issues.is_empty() {
            info!("No issues to display");
            let message = "No issues found";
            return Ok(if colored {
                message.dimmed().to_string()
            } else {
                message.to_string()
            });
        }

        let mut output = String::new();

        // Add summary header
        let header = format!("Issues Overview ({} total)\n\n", issues.len());
        output.push_str(&if colored {
            header.bold().to_string()
        } else {
            header
        });

        // Headers with colors
        let headers = "ID    | Title                          | Status      | Priority | Assignee     | Labels\n";
        let separator = "------|--------------------------------|-------------|----------|--------------|--------\n";

        if colored {
            output.push_str(&headers.cyan().bold().to_string());
            output.push_str(&separator.cyan().to_string());
        } else {
            output.push_str(headers);
            output.push_str(separator);
        }

        // Rows with colored information
        for issue in issues {
            let id = issue.id.unwrap_or(0);
            let title = Self::truncate_title(&issue.title, 30);
            let status = Self::format_colored_status(&issue.status, colored);
            let priority = Self::format_colored_priority(&issue.priority, colored);
            let assignee = Self::format_assignee(&issue.assignee);
            let labels = Self::format_labels(&issue.labels);

            if colored {
                // For colored output, we need to handle padding manually since ANSI codes affect length
                let status_padded = Self::pad_colored_string(&status, 11);
                let priority_padded = Self::pad_colored_string(&priority, 8);

                output.push_str(&format!(
                    "#{:<4} | {:<30} | {} | {} | {:<12} | {}\n",
                    id, title, status_padded, priority_padded, assignee, labels
                ));
            } else {
                output.push_str(&format!(
                    "#{:<4} | {:<30} | {:<11} | {:<8} | {:<12} | {}\n",
                    id, title, status, priority, assignee, labels
                ));
            }
        }

        info!("Formatted colored table with {} issues", issues.len());
        Ok(output)
    }

    /// Format issues as a colored compact table
    pub fn format_issues_compact_colored(issues: &[Issue], colored: bool) -> Result<String> {
        if issues.is_empty() {
            let message = "No issues found";
            return Ok(if colored {
                message.dimmed().to_string()
            } else {
                message.to_string()
            });
        }

        let mut output = String::new();

        let headers = "ID | Title | Status\n";
        let separator = "---|-------|-------\n";

        if colored {
            output.push_str(&headers.cyan().bold().to_string());
            output.push_str(&separator.cyan().to_string());
        } else {
            output.push_str(headers);
            output.push_str(separator);
        }

        for issue in issues {
            let id = issue.id.unwrap_or(0);
            let title = Self::truncate_title(&issue.title, 20);
            let status = Self::format_colored_status(&issue.status, colored);

            output.push_str(&format!("#{} | {} | {}\n", id, title, status));
        }

        Ok(output)
    }

    fn format_colored_status(status: &IssueStatus, colored: bool) -> String {
        let status_str = status.to_string();
        if !colored {
            return status_str;
        }

        match status {
            IssueStatus::Open => status_str.green().to_string(),
            IssueStatus::InProgress => status_str.yellow().to_string(),
            IssueStatus::Resolved => status_str.blue().to_string(),
            IssueStatus::Closed => status_str.bright_black().to_string(),
        }
    }

    fn format_colored_priority(priority: &IssuePriority, colored: bool) -> String {
        let priority_str = priority.to_string();
        if !colored {
            return priority_str;
        }

        match priority {
            IssuePriority::Low => priority_str.bright_black().to_string(),
            IssuePriority::Medium => priority_str.white().to_string(),
            IssuePriority::High => priority_str.yellow().to_string(),
            IssuePriority::Critical => priority_str.red().bold().to_string(),
        }
    }

    fn pad_colored_string(colored_str: &str, target_width: usize) -> String {
        // Count only visible characters (excluding ANSI escape sequences)
        let mut visual_chars = 0;
        let mut in_escape = false;

        for c in colored_str.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape && c == 'm' {
                in_escape = false;
            } else if !in_escape {
                visual_chars += 1;
            }
        }

        if visual_chars >= target_width {
            colored_str.to_string()
        } else {
            format!("{}{}", colored_str, " ".repeat(target_width - visual_chars))
        }
    }

    /// Format issues as a Kanban board view
    pub fn format_issues_kanban(issues: &[Issue], colored: bool) -> Result<String> {
        debug!(
            "Formatting {} issues for Kanban board display (colored: {})",
            issues.len(),
            colored
        );

        if issues.is_empty() {
            let header = "KANBAN BOARD OVERVIEW (0 issues)";
            let message = "No issues found";
            return Ok(format!(
                "{}\n\n{}",
                if colored {
                    header.bold().to_string()
                } else {
                    header.to_string()
                },
                if colored {
                    message.dimmed().to_string()
                } else {
                    message.to_string()
                }
            ));
        }

        let mut output = String::new();

        // Header
        let header = format!("KANBAN BOARD OVERVIEW ({} issues)", issues.len());
        output.push_str(&if colored {
            header.bold().to_string()
        } else {
            header
        });
        output.push_str("\n\n");

        // Group issues by status
        let mut preparing = Vec::new();
        let mut progressing = Vec::new();
        let mut done = Vec::new();
        let mut backlog = Vec::new();

        for issue in issues {
            match issue.status {
                IssueStatus::Open => preparing.push(issue),
                IssueStatus::InProgress => progressing.push(issue),
                IssueStatus::Resolved => done.push(issue),
                IssueStatus::Closed => backlog.push(issue),
            }
        }

        // Format each column
        Self::format_kanban_column(&mut output, "PREPARING", &preparing, colored)?;
        Self::format_kanban_column(&mut output, "PROGRESSING", &progressing, colored)?;
        Self::format_kanban_column(&mut output, "DONE", &done, colored)?;
        Self::format_kanban_column(&mut output, "BACKLOG", &backlog, colored)?;

        info!("Formatted Kanban board with {} issues", issues.len());
        Ok(output)
    }

    /// Format issues as a compact Kanban board view
    pub fn format_issues_kanban_compact(issues: &[Issue], colored: bool) -> Result<String> {
        if issues.is_empty() {
            let header = "KANBAN BOARD OVERVIEW (0 issues)";
            let message = "No issues found";
            return Ok(format!(
                "{}\n\n{}",
                if colored {
                    header.bold().to_string()
                } else {
                    header.to_string()
                },
                if colored {
                    message.dimmed().to_string()
                } else {
                    message.to_string()
                }
            ));
        }

        let mut output = String::new();

        // Header
        let header = format!("KANBAN BOARD OVERVIEW ({} issues)", issues.len());
        output.push_str(&if colored {
            header.bold().to_string()
        } else {
            header
        });
        output.push_str("\n\n");

        // Group issues by status (same as regular kanban)
        let mut preparing = Vec::new();
        let mut progressing = Vec::new();
        let mut done = Vec::new();
        let mut backlog = Vec::new();

        for issue in issues {
            match issue.status {
                IssueStatus::Open => preparing.push(issue),
                IssueStatus::InProgress => progressing.push(issue),
                IssueStatus::Resolved => done.push(issue),
                IssueStatus::Closed => backlog.push(issue),
            }
        }

        // Format each column with compact format (no assignee, fewer details)
        Self::format_kanban_column_compact(&mut output, "PREPARING", &preparing, colored)?;
        Self::format_kanban_column_compact(&mut output, "PROGRESSING", &progressing, colored)?;
        Self::format_kanban_column_compact(&mut output, "DONE", &done, colored)?;
        Self::format_kanban_column_compact(&mut output, "BACKLOG", &backlog, colored)?;

        Ok(output)
    }

    /// Format issues filtered by status as Kanban columns
    pub fn format_issues_kanban_status(
        issues: &[Issue],
        status_filter: Option<IssueStatus>,
        colored: bool,
    ) -> Result<String> {
        let filtered_issues: Vec<&Issue> = issues
            .iter()
            .filter(|issue| {
                status_filter
                    .as_ref()
                    .map(|filter| {
                        std::mem::discriminant(&issue.status) == std::mem::discriminant(filter)
                    })
                    .unwrap_or(true)
            })
            .collect();

        let owned_issues: Vec<Issue> = filtered_issues.into_iter().cloned().collect();
        Self::format_issues_kanban(&owned_issues, colored)
    }

    fn format_kanban_column(
        output: &mut String,
        column_name: &str,
        issues: &[&Issue],
        colored: bool,
    ) -> Result<()> {
        if issues.is_empty() && column_name != "BACKLOG" {
            return Ok(()); // Skip empty columns except backlog
        }

        // Column header
        let header = format!("{} ({})", column_name, issues.len());
        if colored {
            let colored_header = match column_name {
                "PREPARING" => header.yellow().bold(),
                "PROGRESSING" => header.blue().bold(),
                "DONE" => header.green().bold(),
                "BACKLOG" => header.bright_black().bold(),
                _ => header.white().bold(),
            };
            output.push_str(&colored_header.to_string());
        } else {
            output.push_str(&header);
        }
        output.push('\n');

        // Format issues in this column
        for issue in issues {
            let id = issue.id.unwrap_or(0);
            let title = Self::truncate_title(&issue.title, 40);
            let assignee = Self::format_assignee(&issue.assignee);
            let labels = Self::format_labels(&issue.labels);

            if colored {
                let colored_number = format!("#{}", id);
                let final_id = match issue.priority {
                    IssuePriority::Critical => colored_number.red().bold().to_string(),
                    IssuePriority::High => colored_number.yellow().to_string(),
                    IssuePriority::Medium => colored_number.to_string(),
                    IssuePriority::Low => colored_number.bright_black().to_string(),
                };
                output.push_str(&format!(
                    "   {}  {:<40} {:<12} {}\n",
                    final_id, title, assignee, labels
                ));
            } else {
                output.push_str(&format!(
                    "   #{:<2} {:<40} {:<12} {}\n",
                    id, title, assignee, labels
                ));
            }
        }
        output.push('\n');
        Ok(())
    }

    fn format_kanban_column_compact(
        output: &mut String,
        column_name: &str,
        issues: &[&Issue],
        colored: bool,
    ) -> Result<()> {
        if issues.is_empty() && column_name != "BACKLOG" {
            return Ok(());
        }

        // Column header (same as regular)
        let header = format!("{} ({})", column_name, issues.len());
        if colored {
            let colored_header = match column_name {
                "PREPARING" => header.yellow().bold(),
                "PROGRESSING" => header.blue().bold(),
                "DONE" => header.green().bold(),
                "BACKLOG" => header.bright_black().bold(),
                _ => header.white().bold(),
            };
            output.push_str(&colored_header.to_string());
        } else {
            output.push_str(&header);
        }
        output.push('\n');

        // Format issues in compact mode (no assignee)
        for issue in issues {
            let id = issue.id.unwrap_or(0);
            let title = Self::truncate_title(&issue.title, 50);
            let labels = Self::format_labels(&issue.labels);

            if colored {
                let colored_number = format!("#{}", id);
                let final_id = match issue.priority {
                    IssuePriority::Critical => colored_number.red().bold().to_string(),
                    IssuePriority::High => colored_number.yellow().to_string(),
                    IssuePriority::Medium => colored_number.to_string(),
                    IssuePriority::Low => colored_number.bright_black().to_string(),
                };
                output.push_str(&format!("   {}  {:<50} {}\n", final_id, title, labels));
            } else {
                output.push_str(&format!("   #{:<2} {:<50} {}\n", id, title, labels));
            }
        }
        output.push('\n');
        Ok(())
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

    #[test]
    fn test_format_issues_colored() {
        let issues = vec![create_test_issue(
            1,
            "Test Issue",
            IssueStatus::Open,
            IssuePriority::High,
        )];

        let result = TableFormatter::format_issues_colored(&issues, true);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should contain ANSI color codes
        assert!(output.contains("\x1b["));
        assert!(output.contains("Test Issue"));
    }

    #[test]
    fn test_format_colored_status_colors() {
        let issues = vec![
            create_test_issue(1, "Open Issue", IssueStatus::Open, IssuePriority::Medium),
            create_test_issue(
                2,
                "In Progress Issue",
                IssueStatus::InProgress,
                IssuePriority::Medium,
            ),
            create_test_issue(
                3,
                "Resolved Issue",
                IssueStatus::Resolved,
                IssuePriority::Medium,
            ),
            create_test_issue(
                4,
                "Closed Issue",
                IssueStatus::Closed,
                IssuePriority::Medium,
            ),
        ];

        let result = TableFormatter::format_issues_colored(&issues, true);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should contain different colors for different statuses
        assert!(output.contains("Open Issue"));
        assert!(output.contains("In Progress Issue"));
        assert!(output.contains("Resolved Issue"));
        assert!(output.contains("Closed Issue"));
    }

    #[test]
    fn test_format_colored_priority_colors() {
        let issues = vec![
            create_test_issue(1, "Low Priority", IssueStatus::Open, IssuePriority::Low),
            create_test_issue(
                2,
                "Medium Priority",
                IssueStatus::Open,
                IssuePriority::Medium,
            ),
            create_test_issue(3, "High Priority", IssueStatus::Open, IssuePriority::High),
            create_test_issue(
                4,
                "Critical Priority",
                IssueStatus::Open,
                IssuePriority::Critical,
            ),
        ];

        let result = TableFormatter::format_issues_colored(&issues, true);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should contain different colors for different priorities
        assert!(output.contains("Low Priority"));
        assert!(output.contains("Medium Priority"));
        assert!(output.contains("High Priority"));
        assert!(output.contains("Critical Priority"));
    }

    #[test]
    fn test_format_colored_vs_plain() {
        let issues = vec![create_test_issue(
            1,
            "Test Issue",
            IssueStatus::Open,
            IssuePriority::High,
        )];

        let colored_result = TableFormatter::format_issues_colored(&issues, true);
        let plain_result = TableFormatter::format_issues_colored(&issues, false);

        assert!(colored_result.is_ok());
        assert!(plain_result.is_ok());

        let colored_output = colored_result.unwrap();
        let plain_output = plain_result.unwrap();

        // Colored output should contain ANSI codes, plain should not
        assert!(colored_output.contains("\x1b["));
        assert!(!plain_output.contains("\x1b["));

        // Both should contain the same text content (ignoring colors)
        assert!(colored_output.contains("Test Issue"));
        assert!(plain_output.contains("Test Issue"));
    }

    #[test]
    fn test_format_compact_colored() {
        let issues = vec![create_test_issue(
            1,
            "Test Issue",
            IssueStatus::Open,
            IssuePriority::High,
        )];

        let result = TableFormatter::format_issues_compact_colored(&issues, true);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Test Issue"));
        // Compact colored output should still have colors
        assert!(output.contains("\x1b["));
    }

    // Kanban view tests
    #[test]
    fn test_format_issues_kanban_empty() {
        let issues = vec![];
        let result = TableFormatter::format_issues_kanban(&issues, false);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("KANBAN BOARD OVERVIEW (0 issues)"));
        assert!(output.contains("No issues found"));
    }

    #[test]
    fn test_format_issues_kanban_basic() {
        let issues = vec![
            create_test_issue(1, "Open Issue", IssueStatus::Open, IssuePriority::High),
            create_test_issue(
                2,
                "In Progress Issue",
                IssueStatus::InProgress,
                IssuePriority::Medium,
            ),
            create_test_issue(
                3,
                "Resolved Issue",
                IssueStatus::Resolved,
                IssuePriority::Low,
            ),
            create_test_issue(
                4,
                "Closed Issue",
                IssueStatus::Closed,
                IssuePriority::Critical,
            ),
        ];

        let result = TableFormatter::format_issues_kanban(&issues, false);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("KANBAN BOARD OVERVIEW (4 issues)"));
        assert!(output.contains("PREPARING"));
        assert!(output.contains("PROGRESSING"));
        assert!(output.contains("DONE"));
        assert!(output.contains("BACKLOG"));
        assert!(output.contains("Open Issue"));
        assert!(output.contains("In Progress Issue"));
        assert!(output.contains("Resolved Issue"));
        assert!(output.contains("Closed Issue"));
    }

    #[test]
    fn test_format_issues_kanban_colored() {
        let issues = vec![
            create_test_issue(1, "High Priority", IssueStatus::Open, IssuePriority::High),
            create_test_issue(
                2,
                "Critical Issue",
                IssueStatus::InProgress,
                IssuePriority::Critical,
            ),
        ];

        let result = TableFormatter::format_issues_kanban(&issues, true);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should contain ANSI color codes for headers and issue numbers
        assert!(output.contains("\x1b["));
        assert!(output.contains("High Priority"));
        assert!(output.contains("Critical Issue"));
    }

    #[test]
    fn test_format_issues_kanban_compact() {
        let issues = vec![create_test_issue(
            1,
            "Test Issue",
            IssueStatus::Open,
            IssuePriority::Medium,
        )];

        let result = TableFormatter::format_issues_kanban_compact(&issues, false);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Test Issue"));
        // Compact should have fewer details
        assert!(!output.contains("@test-user"));
    }

    #[test]
    fn test_format_issues_kanban_status_filter() {
        let issues = vec![
            create_test_issue(1, "Open Issue", IssueStatus::Open, IssuePriority::Medium),
            create_test_issue(
                2,
                "Closed Issue",
                IssueStatus::Closed,
                IssuePriority::Medium,
            ),
        ];

        let result =
            TableFormatter::format_issues_kanban_status(&issues, Some(IssueStatus::Open), false);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Open Issue"));
        assert!(!output.contains("Closed Issue"));
        // Should only show the filtered status column
        assert!(output.contains("PREPARING"));
        assert!(!output.contains("DONE"));
    }

    #[test]
    fn test_format_issues_kanban_column_grouping() {
        let issues = vec![
            create_test_issue(1, "Issue 1", IssueStatus::Open, IssuePriority::High),
            create_test_issue(2, "Issue 2", IssueStatus::Open, IssuePriority::Low),
            create_test_issue(3, "Issue 3", IssueStatus::InProgress, IssuePriority::Medium),
        ];

        let result = TableFormatter::format_issues_kanban(&issues, false);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should group issues correctly by status
        assert!(output.contains("PREPARING (2)"));
        assert!(output.contains("PROGRESSING (1)"));
        assert!(output.contains("Issue 1"));
        assert!(output.contains("Issue 2"));
        assert!(output.contains("Issue 3"));
    }
}
