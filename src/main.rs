mod db;
pub mod git_ops;

use anyhow::Result;
use chrono::Utc;
use db::{TaskDatabase, Issue, IssueStatus, IssuePriority};

#[tokio::main]
async fn main() -> Result<()> {
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
        println!("   - {} by {} ({})", 
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
    
    // Create a sample issue if none exist
    if issues.is_empty() {
        println!("\n📝 Creating sample issue...");
        let sample_issue = Issue {
            id: None,
            title: "Setup project documentation".to_string(),
            description: Some("Create README.md and setup documentation for the atask project".to_string()),
            status: IssueStatus::Open,
            priority: IssuePriority::Medium,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            assignee: None,
            labels: vec!["documentation".to_string(), "good first issue".to_string()],
        };
        
        let issue_id = db.insert_issue(&sample_issue).await?;
        println!("✅ Created sample issue #{}", issue_id);
        
        // Fetch and display the updated issues
        let updated_issues = db.get_all_issues().await?;
        for issue in &updated_issues {
            println!("   - #{}: {} [{}] - Labels: {}", 
                issue.id.unwrap_or(0), 
                issue.title,
                issue.status.to_string(),
                issue.labels.join(", ")
            );
        }
    } else {
        for issue in &issues {
            println!("   - #{}: {} [{}] - Labels: {}", 
                issue.id.unwrap_or(0), 
                issue.title,
                issue.status.to_string(),
                issue.labels.join(", ")
            );
        }
    }
    
    println!("\n🎉 ATask database is ready!");
    println!("\n💡 Next steps:");
    println!("   - Add more issues using the database API");
    println!("   - Create custom labels for your workflow");
    println!("   - Track commit history as you develop");
    
    Ok(())
}
