//! ATask - Git-based task management system
//! 
//! This crate provides Git and GitHub operations using Rust libraries
//! instead of relying on CLI tools, avoiding pager/editor interaction issues.

pub mod db;
pub mod git_ops;
pub mod kanban;
pub mod web;
