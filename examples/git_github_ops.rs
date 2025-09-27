use anyhow::Result;
use atask::git_ops::{GitHubOps, GitOps};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Git and GitHub Operations Example");

    // Initialize Git operations for current repository
    match GitOps::new() {
        Ok(git_ops) => {
            println!("✅ Successfully connected to Git repository");

            // Get remote URL
            match git_ops.get_remote_url("origin") {
                Ok(url) => {
                    println!("📡 Remote URL: {}", url);

                    // Parse GitHub owner and repo
                    match git_ops.parse_github_repo("origin") {
                        Ok((owner, repo)) => {
                            println!("👤 Owner: {}, Repository: {}", owner, repo);

                            // Get recent commits
                            match git_ops.get_commits(Some(5)) {
                                Ok(commits) => {
                                    println!("📝 Recent commits ({} found):", commits.len());
                                    for (i, commit) in commits.iter().enumerate() {
                                        println!(
                                            "   {}. {} - {} by {} ({} files, +{} -{} lines)",
                                            i + 1,
                                            &commit.hash[..8],
                                            commit.message.lines().next().unwrap_or("No message"),
                                            commit.author_name,
                                            commit.files_changed.len(),
                                            commit.insertions,
                                            commit.deletions
                                        );
                                    }
                                }
                                Err(e) => println!("⚠️  Could not retrieve commits: {}", e),
                            }

                            // Try GitHub operations if token is available
                            if let Ok(_token) = std::env::var("GITHUB_TOKEN") {
                                println!("\n🐙 Attempting GitHub operations...");

                                match GitHubOps::from_env(owner, repo) {
                                    Ok(github_ops) => {
                                        println!("✅ Successfully connected to GitHub API");

                                        // List issues
                                        match github_ops.list_issues().await {
                                            Ok(issues) => {
                                                println!(
                                                    "🎯 Repository issues ({} found):",
                                                    issues.len()
                                                );
                                                for issue in issues.iter().take(5) {
                                                    println!(
                                                        "   #{}: {} [{:?}]",
                                                        issue.number, issue.title, issue.state
                                                    );
                                                }
                                            }
                                            Err(e) => println!("⚠️  Could not list issues: {}", e),
                                        }

                                        // List labels
                                        match github_ops.list_labels().await {
                                            Ok(labels) => {
                                                println!(
                                                    "🏷️  Repository labels ({} found):",
                                                    labels.len()
                                                );
                                                for label in labels.iter().take(8) {
                                                    println!(
                                                        "   - {} ({})",
                                                        label.name, label.color
                                                    );
                                                }
                                            }
                                            Err(e) => println!("⚠️  Could not list labels: {}", e),
                                        }
                                    }
                                    Err(e) => println!("❌ Could not connect to GitHub API: {}", e),
                                }
                            } else {
                                println!("ℹ️  Set GITHUB_TOKEN environment variable to try GitHub API operations");
                            }
                        }
                        Err(e) => println!("⚠️  Could not parse GitHub repository info: {}", e),
                    }
                }
                Err(e) => println!("⚠️  Could not get remote URL: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Could not connect to Git repository: {}", e);
            println!("   Make sure you're in a Git repository directory");
        }
    }

    println!("\n💡 Benefits of using Rust crates over CLI:");
    println!("   - No dependency on external CLI tools");
    println!("   - No issues with pager/editor interactions");
    println!("   - Better error handling and type safety");
    println!("   - Direct access to Git objects and GitHub API");
    println!("   - Programmatic control without shell escaping");

    Ok(())
}
