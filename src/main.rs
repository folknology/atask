mod cli;
mod db;
pub mod git_ops;
pub mod kanban;
pub mod web;

use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use cli::TableFormatter;
use db::{Issue, IssuePriority, IssueStatus, TaskDatabase};
use web::KanbanWebServer;

#[derive(Parser)]
#[command(name = "atask")]
#[command(about = "A GitHub-based task management CLI with Kanban board visualization")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the database and show current status
    Init,
    /// List all issues from the database
    ListIssues,
    /// Show database statistics
    DbStats,
    /// Show git commit history
    Commits {
        /// Number of commits to show
        #[arg(short, long, default_value_t = 10)]
        count: usize,
    },
    /// Display issues in a formatted table
    Table {
        /// Filter by issue status
        #[arg(long)]
        status: Option<String>,
        /// Filter by issue priority
        #[arg(long)]
        priority: Option<String>,
        /// Use compact view
        #[arg(long)]
        compact: bool,
    },
    /// Start the Kanban web server (requires GitHub token)
    Web {
        /// Port to run the web server on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            init_database().await?;
        }
        Commands::ListIssues => {
            let db = TaskDatabase::new("atask.db").await?;
            let issues = db.get_all_issues().await?;

            println!("📝 Issues ({}):", issues.len());
            for issue in &issues {
                println!(
                    "   - #{}: {} [{}] - Labels: {}",
                    issue.id.unwrap_or(0),
                    issue.title,
                    issue.status,
                    issue.labels.join(", ")
                );
            }
        }
        Commands::DbStats => {
            let db = TaskDatabase::new("atask.db").await?;
            let commits = db.get_all_commits().await?;
            let labels = db.get_all_labels().await?;
            let issues = db.get_all_issues().await?;

            println!("📊 Database Statistics:");
            println!("   Commits: {}", commits.len());
            println!("   Labels: {}", labels.len());
            println!("   Issues: {}", issues.len());
        }
        Commands::Commits { count } => {
            let db = TaskDatabase::new("atask.db").await?;
            let commits = db.get_all_commits().await?;

            println!("📦 Git Commits ({}):", commits.len().min(count));
            for commit in commits.iter().take(count) {
                println!(
                    "   - {} by {} ({})",
                    &commit.hash[..8],
                    commit.author_name,
                    commit.commit_date.format("%Y-%m-%d %H:%M")
                );
            }
        }
        Commands::Table {
            status,
            priority,
            compact,
        } => {
            let db = TaskDatabase::new("atask.db").await?;
            let issues = db.get_all_issues().await?;

            // Parse filter options
            let status_filter = status
                .as_ref()
                .map(|s| s.parse::<IssueStatus>())
                .transpose()?;

            let priority_filter = priority
                .as_ref()
                .map(|p| p.parse::<IssuePriority>())
                .transpose()?;

            // Format and display table
            let table_output = if compact {
                TableFormatter::format_issues_compact(&issues)?
            } else if status_filter.is_some() || priority_filter.is_some() {
                TableFormatter::format_issues_filtered(&issues, status_filter, priority_filter)?
            } else {
                TableFormatter::format_issues(&issues)?
            };

            println!("{}", table_output);
        }
        Commands::Web { port } => {
            println!("🚀 Starting Kanban Web Server...");

            // Initialize database
            let db = TaskDatabase::new("atask.db").await?;
            println!("✅ Database initialized");

            // Create web server with database
            let server = KanbanWebServer::new(db);

            println!("🌐 Starting web server on port {}", port);

            server.serve(port).await?;
        }
    }

    Ok(())
}

async fn init_database() -> Result<()> {
    println!("🚀 Initializing ATask - Git Task Manager");

    // Initialize database
    let db = TaskDatabase::new("atask.db").await?;
    println!("✅ Database initialized");

    // Create default labels
    db.create_default_labels().await?;
    println!("✅ Default labels created");

    // Populate from git history if available
    match db.populate_from_git_history(None).await {
        Ok(count) => {
            if count > 0 {
                println!("✅ Populated {} commits from git history", count);
            } else {
                println!("ℹ️  No new commits to import");
            }
        }
        Err(e) => {
            println!("⚠️  Could not populate from git history: {}", e);
            println!("   This is normal for a new repository with no commits");
        }
    }

    // Display current data
    println!("\n📊 Current Database State:");

    // Show commits
    let commits = db.get_all_commits().await?;
    println!("   Commits: {}", commits.len());
    for commit in commits.iter().take(3) {
        println!(
            "   - {} by {} ({})",
            &commit.hash[..8],
            commit.author_name,
            commit.commit_date.format("%Y-%m-%d %H:%M")
        );
    }
    if commits.len() > 3 {
        println!("   ... and {} more", commits.len() - 3);
    }

    // Show labels
    let labels = db.get_all_labels().await?;
    println!("   Labels: {}", labels.len());
    for label in labels.iter().take(5) {
        println!("   - {} ({})", label.name, label.color);
    }
    if labels.len() > 5 {
        println!("   ... and {} more", labels.len() - 5);
    }

    // Show issues
    let issues = db.get_all_issues().await?;
    println!("   Issues: {}", issues.len());

    // Try to load GitHub issues using gh CLI
    println!("\n🔍 Attempting to load GitHub issues...");
    match db.load_github_issues_via_cli().await {
        Ok(count) => {
            if count > 0 {
                println!("✅ Loaded {} GitHub issues", count);
            } else {
                println!("ℹ️  No GitHub issues found or already up to date");
            }
        }
        Err(e) => {
            println!("⚠️  Could not load GitHub issues via gh CLI: {}", e);
            println!("   This is normal if 'gh' is not installed or not authenticated");

            // Create a sample issue if none exist and gh loading failed
            let current_issues = db.get_all_issues().await?;
            if current_issues.is_empty() {
                println!("\n📝 Creating sample issue...");
                let sample_issue = Issue {
                    id: None,
                    title: "Setup project documentation".to_string(),
                    description: Some(
                        "Create README.md and setup documentation for the atask project"
                            .to_string(),
                    ),
                    status: IssueStatus::Open,
                    priority: IssuePriority::Medium,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    assignee: None,
                    labels: vec!["documentation".to_string(), "good first issue".to_string()],
                };

                let issue_id = db.insert_issue(&sample_issue).await?;
                println!("✅ Created sample issue #{}", issue_id);
            }
        }
    }

    // Display final issue count
    let final_issues = db.get_all_issues().await?;
    println!("   Final issue count: {}", final_issues.len());
    for issue in final_issues.iter().take(5) {
        println!(
            "   - #{}: {} [{}] - Labels: {}",
            issue.id.unwrap_or(0),
            issue.title,
            issue.status,
            issue.labels.join(", ")
        );
    }
    if final_issues.len() > 5 {
        println!("   ... and {} more", final_issues.len() - 5);
    }

    println!("\n🎉 ATask database is ready!");
    println!("\n💡 Next steps:");
    println!("   - Use 'atask list-issues' to see all issues");
    println!("   - Use 'atask web' to start the Kanban web interface");
    println!("   - Use 'atask commits' to see git history");

    Ok(())
}
