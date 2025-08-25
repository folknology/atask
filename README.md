# ATask - Agent Task Manager

A Rust-based agentic task management system that integrates with Git to track commits and manage issues with custom labels.

## Features

- **Git Integration**: Automatically imports and tracks git commit history
- **Issue Management**: Create, track, and manage issues with priorities and statuses
- **Label System**: Use predefined or custom labels to categorize issues
- **LibSQL Database**: Fast, local database storage with CRUD operations
- **Command Line Interface**: Simple CLI for viewing status and data

## Database Schema

### Commits Table
- `id`: Primary key
- `hash`: Git commit hash (unique)
- `author_name`: Commit author name
- `author_email`: Commit author email
- `commit_date`: When the commit was made
- `message`: Commit message
- `files_changed`: JSON array of changed files
- `insertions`: Number of line insertions
- `deletions`: Number of line deletions

### Issues Table
- `id`: Primary key
- `title`: Issue title
- `description`: Issue description (optional)
- `status`: Issue status (open, in_progress, resolved, closed)
- `priority`: Issue priority (low, medium, high, critical)
- `assignee`: Assigned person (optional)
- `created_at`: Creation timestamp
- `updated_at`: Last update timestamp

### Labels Table
- `id`: Primary key
- `name`: Label name (unique)
- `color`: Hex color code
- `description`: Label description (optional)
- `created_at`: Creation timestamp

### Issue-Labels Junction Table
- Many-to-many relationship between issues and labels

## Default Labels

The system comes with 8 default GitHub-style labels:

- **bug** (`#d73a4a`) - Something isn't working
- **enhancement** (`#a2eeef`) - New feature or request
- **documentation** (`#0075ca`) - Improvements or additions to documentation
- **good first issue** (`#7057ff`) - Good for newcomers
- **help wanted** (`#008672`) - Extra attention is needed
- **invalid** (`#e4e669`) - This doesn't seem right
- **question** (`#d876e3`) - Further information is requested
- **wontfix** (`#ffffff`) - This will not be worked on

## Usage

### Running the Application

```bash
cargo run
```

This will:
1. Initialize the database (`atask.db`)
2. Create default labels if they don't exist
3. Import new git commits from the current repository
4. Display current database statistics
5. Create a sample issue if none exist

### Current Functionality

The application currently provides:
- Automatic git history import
- Database initialization and schema creation
- Default label creation
- Sample issue creation
- Status reporting

### CRUD Operations Available

The `TaskDatabase` struct provides comprehensive CRUD operations:

#### Commits
- `insert_commit(&self, commit: &GitCommit) -> Result<i64>`
- `get_commit_by_hash(&self, hash: &str) -> Result<Option<GitCommit>>`
- `get_all_commits(&self) -> Result<Vec<GitCommit>>`

#### Labels
- `insert_label(&self, label: &Label) -> Result<i64>`
- `get_label_by_name(&self, name: &str) -> Result<Option<Label>>`
- `get_all_labels(&self) -> Result<Vec<Label>>`

#### Issues
- `insert_issue(&self, issue: &Issue) -> Result<i64>`
- `get_issue_by_id(&self, id: i64) -> Result<Option<Issue>>`
- `get_all_issues(&self) -> Result<Vec<Issue>>`
- `update_issue_status(&self, id: i64, status: IssueStatus) -> Result<()>`
- `delete_issue(&self, id: i64) -> Result<()>`

#### Git Integration
- `populate_from_git_history(&self, repo_path: Option<&str>) -> Result<usize>`
- `create_default_labels(&self) -> Result<()>`

## Dependencies

- `libsql` (0.9.20) - Database engine
- `tokio` (1.0) - Async runtime
- `serde` (1.0) - Serialization
- `serde_json` (1.0) - JSON handling
- `chrono` (0.4) - Date/time handling
- `anyhow` (1.0) - Error handling
- `clap` (4.0) - Command line parsing (for future CLI expansion)

## Architecture

The project follows a modular architecture:

- `main.rs` - Entry point and CLI interface
- `db.rs` - Database module with all CRUD operations and data structures

## Future Enhancements

This foundation supports many potential enhancements:

- Full CLI with commands for creating/managing issues
- Web interface for issue management
- GitHub/GitLab integration
- Project templates and workflows
- Time tracking and reporting
- Team collaboration features
- Export/import functionality

## Development

To extend the functionality, you can:

1. Add new CLI commands using the existing CRUD operations
2. Create additional database tables for new features
3. Implement REST API endpoints
4. Add web frontend
5. Integrate with external services

The database module provides a solid foundation for any task management features you want to build on top of it.
