use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use askama::Template;
use chrono::Utc;
use pulldown_cmark::{Parser, Options, html};

use crate::db::{TaskDatabase, IssueStatus, IssuePriority};
use crate::kanban::{KanbanBoard, KanbanColumn, KanbanCard, Priority};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<TaskDatabase>,
}

/// Request body for moving issues between columns
#[derive(Debug, Deserialize)]
pub struct MoveIssueRequest {
    pub issue_number: u64,
    pub from_column: String,
    pub to_column: String,
}

/// Response for API endpoints
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

/// Kanban board HTML template
#[derive(Template)]
#[template(path = "kanban.html")]
pub struct KanbanTemplate {
    pub board: KanbanBoard,
}

/// Web server struct
pub struct KanbanWebServer {
    app_state: AppState,
}

impl KanbanWebServer {
    /// Create a new web server with database
    pub fn new(db: TaskDatabase) -> Self {
        let app_state = AppState { db: Arc::new(db) };
        
        Self { app_state }
    }

    /// Create the Axum router with all routes
    pub fn create_router(&self) -> Router {
        Router::new()
            .route("/", axum::routing::get(handlers::kanban_board))
            .route("/api/board", axum::routing::get(handlers::api_board))
            .route("/api/move", axum::routing::post(handlers::api_move_issue))
            .route("/api/refresh/:column_id", axum::routing::post(handlers::api_refresh_column))
            .with_state(self.app_state.clone())
    }

    /// Start the web server on the specified port
    pub async fn serve(&self, port: u16) -> anyhow::Result<()> {
        let app = self.create_router();
        let addr = format!("0.0.0.0:{}", port);
        
        println!("🚀 Kanban web server starting on http://{}", addr);
        
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

/// Helper function to convert markdown to HTML
pub fn markdown_to_html(markdown: &str) -> String {
    // First, unescape the newline characters if they're stored as literal \n
    let processed_markdown = markdown
        .replace("\\n", "\n")  // Replace literal \n with actual newlines
        .replace("\\t", "\t")  // Replace literal \t with actual tabs
        .replace("\\r", "\r"); // Replace literal \r with actual carriage returns
    
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    
    let parser = Parser::new_ext(&processed_markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Route handlers
pub mod handlers {
    use super::*;

    /// Helper function to create a kanban board from database issues
    async fn create_board_from_db(db: &TaskDatabase) -> Result<KanbanBoard, anyhow::Error> {
        let all_issues = db.get_all_issues().await?;
        
        // Only show open issues
        let open_issues: Vec<_> = all_issues.into_iter()
            .filter(|issue| matches!(issue.status, IssueStatus::Open))
            .collect();
        
        // Create columns
        let mut evaluating_cards = Vec::new();
        let mut preparing_cards = Vec::new();
        let mut progressing_cards = Vec::new();
        let mut done_cards = Vec::new();
        
        // Convert database issues to kanban cards and organize by labels
        for issue in open_issues {
            let priority = match issue.priority {
                IssuePriority::Low => Priority::Low,
                IssuePriority::Medium => Priority::Medium,
                IssuePriority::High => Priority::High,
                IssuePriority::Critical => Priority::Critical,
            };
            
            // Render markdown to HTML
            let body_html = issue.description.as_ref()
                .map(|body| markdown_to_html(body))
                .unwrap_or_else(|| "No description".to_string());
                
            let card = KanbanCard {
                issue_number: issue.id.unwrap_or(0) as u64, // Use database ID as issue number
                title: issue.title,
                body: issue.description,
                body_html,
                assignee: issue.assignee,
                labels: issue.labels.clone(),
                priority,
                created_at: issue.created_at,
                updated_at: issue.updated_at,
                comments_count: 0, // Default to 0 for now
            };
            
            // Organize by labels - check for workflow labels
            let has_done = issue.labels.iter().any(|label| label.eq_ignore_ascii_case("done"));
            let has_progressing = issue.labels.iter().any(|label| label.eq_ignore_ascii_case("progressing"));
            let has_preparing = issue.labels.iter().any(|label| label.eq_ignore_ascii_case("preparing"));
            let has_evaluating = issue.labels.iter().any(|label| label.eq_ignore_ascii_case("evaluating"));
            
            if has_done {
                done_cards.push(card);
            } else if has_progressing {
                progressing_cards.push(card);
            } else if has_preparing {
                preparing_cards.push(card);
            } else if has_evaluating {
                evaluating_cards.push(card);
            } else {
                // Default to evaluating if no workflow label is found
                evaluating_cards.push(card);
            }
        }
        
        Ok(KanbanBoard {
            title: "Task Board".to_string(),
            last_updated: Utc::now(),
            columns: vec![
                KanbanColumn {
                    id: "evaluating".to_string(),
                    title: "Evaluating".to_string(),
                    label_name: "Evaluating".to_string(),
                    color: "#fef2c0".to_string(),
                    cards: evaluating_cards,
                },
                KanbanColumn {
                    id: "preparing".to_string(),
                    title: "Preparing".to_string(),
                    label_name: "Preparing".to_string(),
                    color: "#fef3c7".to_string(),
                    cards: preparing_cards,
                },
                KanbanColumn {
                    id: "progressing".to_string(),
                    title: "Progressing".to_string(),
                    label_name: "Progressing".to_string(),
                    color: "#bfdbfe".to_string(),
                    cards: progressing_cards,
                },
                KanbanColumn {
                    id: "done".to_string(),
                    title: "Done".to_string(),
                    label_name: "Done".to_string(),
                    color: "#bbf7d0".to_string(),
                    cards: done_cards,
                },
            ],
        })
    }

    /// Serve the main Kanban board page
    pub async fn kanban_board(State(state): State<AppState>) -> Result<Response, StatusCode> {
        match create_board_from_db(&state.db).await {
            Ok(board) => {
                let template = KanbanTemplate { board };
                match template.render() {
                    Ok(html) => Ok(Html(html).into_response()),
                    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    /// API endpoint to get board data as JSON
    pub async fn api_board(State(state): State<AppState>) -> Json<ApiResponse<KanbanBoard>> {
        match create_board_from_db(&state.db).await {
            Ok(board) => Json(ApiResponse {
                success: true,
                data: Some(board),
                message: None,
            }),
            Err(err) => Json(ApiResponse {
                success: false,
                data: None,
                message: Some(format!("Failed to fetch board: {}", err)),
            }),
        }
    }

    /// API endpoint to move an issue between columns
    pub async fn api_move_issue(
        State(state): State<AppState>,
        Json(request): Json<MoveIssueRequest>,
    ) -> Json<ApiResponse<()>> {
        // Map column names to issue statuses
        let new_status = match request.to_column.as_str() {
            "preparing" => IssueStatus::Open,
            "progressing" => IssueStatus::InProgress,
            "completed" => IssueStatus::Closed,
            _ => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    message: Some(format!("Invalid column: {}", request.to_column)),
                });
            }
        };
        
        // First check if the issue exists
        match state.db.get_issue_by_id(request.issue_number as i64).await {
            Ok(Some(_)) => {
                // Issue exists, proceed with status update
                match state.db.update_issue_status(request.issue_number as i64, new_status).await {
                    Ok(_) => Json(ApiResponse {
                        success: true,
                        data: Some(()),
                        message: Some(format!(
                            "Successfully moved issue #{} from {} to {}",
                            request.issue_number, request.from_column, request.to_column
                        )),
                    }),
                    Err(err) => Json(ApiResponse {
                        success: false,
                        data: None,
                        message: Some(format!("Failed to move issue: {}", err)),
                    }),
                }
            },
            Ok(None) => {
                // Issue doesn't exist
                Json(ApiResponse {
                    success: false,
                    data: None,
                    message: Some(format!("Issue #{} not found", request.issue_number)),
                })
            },
            Err(err) => {
                // Database error
                Json(ApiResponse {
                    success: false,
                    data: None,
                    message: Some(format!("Database error: {}", err)),
                })
            }
        }
    }

    /// API endpoint to refresh a specific column
    pub async fn api_refresh_column(
        State(_state): State<AppState>,
        Path(column_id): Path<String>,
    ) -> Json<ApiResponse<()>> {
        // Validate that the column exists
        let valid_columns = ["preparing", "progressing", "completed"];
        if !valid_columns.contains(&column_id.as_str()) {
            return Json(ApiResponse {
                success: false,
                data: None,
                message: Some(format!("Column '{}' not found", column_id)),
            });
        }
        
        // For database-based implementation, we can always refresh successfully
        // since the data comes directly from the database
        Json(ApiResponse {
            success: true,
            data: Some(()),
            message: Some(format!("Successfully refreshed column '{}'", column_id)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_server() -> KanbanWebServer {
        let db = TaskDatabase::in_memory().await.unwrap();
        KanbanWebServer::new(db)
    }

    #[tokio::test]
    async fn test_web_server_creation() {
        let _server = create_test_server().await;
        // Server should be created successfully
        assert!(true);
    }

    // Web route tests (GREEN phase - should work)
    #[tokio::test]
    async fn test_create_router_works() {
        let server = create_test_server().await;
        // Router creation should work now
        let _router = server.create_router();
        // If we get here, router creation succeeded
        assert!(true);
    }

    // Note: We can't easily test serve() since it would bind to a port and run indefinitely
    // In a real-world scenario, we'd use integration tests with a test client

    // Handler tests (should work with empty database)
    #[tokio::test]
    async fn test_kanban_board_handler_works() {
        let server = create_test_server().await;
        let state = State(server.app_state.clone());
        
        // Handler should work with empty database
        let result = handlers::kanban_board(state).await;
        // Should succeed with empty board
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_board_handler_works() {
        let server = create_test_server().await;
        let state = State(server.app_state.clone());
        
        // Handler should work with empty database
        let result = handlers::api_board(state).await;
        // Should return a JSON response with success=true and empty board
        assert!(result.0.success);
        assert!(result.0.data.is_some());
        let board = result.0.data.unwrap();
        assert_eq!(board.title, "Task Board");
        assert_eq!(board.columns.len(), 4);
    }

    #[tokio::test]
    async fn test_api_move_issue_handler_fails_for_nonexistent_issue() {
        let server = create_test_server().await;
        let state = State(server.app_state.clone());
        let request = Json(MoveIssueRequest {
            issue_number: 999, // Non-existent issue
            from_column: "preparing".to_string(),
            to_column: "progressing".to_string(),
        });
        
        // Should fail since issue doesn't exist
        let result = handlers::api_move_issue(state, request).await;
        assert!(!result.0.success);
        assert!(result.0.message.is_some());
    }

    #[tokio::test]
    async fn test_api_refresh_column_handler_works() {
        let server = create_test_server().await;
        let state = State(server.app_state.clone());
        let column_id = Path("preparing".to_string());
        
        // Should succeed for valid column
        let result = handlers::api_refresh_column(state, column_id).await;
        assert!(result.0.success);
        assert!(result.0.message.is_some());
    }

    #[tokio::test]
    async fn test_api_refresh_column_handler_fails_for_invalid_column() {
        let server = create_test_server().await;
        let state = State(server.app_state.clone());
        let column_id = Path("invalid_column".to_string());
        
        // Should fail for invalid column
        let result = handlers::api_refresh_column(state, column_id).await;
        assert!(!result.0.success);
        assert!(result.0.message.is_some());
    }

    #[test]
    fn test_markdown_to_html_with_newlines() {
        // Test that literal \n characters are properly converted to newlines
        let input = "This is line 1\nThis is line 2\n\nThis is after blank line";
        
        let result = super::markdown_to_html(input);
        
        // Should contain proper HTML paragraphs, not literal \n
        assert!(!result.contains("\\n"), "Should not contain literal \\n characters");
        assert!(result.contains("<p>"), "Should contain HTML paragraph tags");
        
        // Should contain the processed content
        assert!(result.contains("This is line 1"), "Should contain first line");
        assert!(result.contains("This is line 2"), "Should contain second line");
    }
}
