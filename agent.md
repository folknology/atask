# ATask Project - Git Task Manager

**Project Status**: Active Development  
**Created**: 2025-08-25  
**Language**: Rust  
**Database**: LibSQL  

## Project Overview

ATask is a Rust-based task management system that integrates with Git to automatically track commits and manage issues with custom labels. The project provides a foundation for building comprehensive task management workflows with Git integration.

## Development Preferences & Guidelines

### Test-Driven Development (TDD)
- **Mandatory Approach**: Use strict Red-Green-Blue TDD methodology
- **Reference Documentation**: `~/.agents/development/tdd-development.md`
- **Process**:
  1. **Red Phase**: Write failing tests first (verified automatically)
  2. **Green Phase**: Minimal implementation to pass tests
  3. **Blue Phase**: Refactor while maintaining green state
- **Enforcement**: Use automated test failure validation during Red phase
- **No Exceptions**: All new functionality must follow TDD cycle

### Code Quality Standards
- **No Emojis/Unicode**: Keep code professional unless specifically requested
- **Logging**: Use `log` crate instead of `println!` or `eprintln!`
- **Diff Visibility**: Always show code diffs for transparency
- **Memory Safety**: Leverage Rust's safety features, minimize unsafe code

### Version Control Practices
- **Reference Documentation**: `~/.agents/development/version-control.md`
- **Commit Strategy**: Commit after each development step
- **Tools**: Use both `git` and `gh` (GitHub CLI) command line tools
- **Code Reviews**: Create code reviews on plan completion
- **Transparency**: Always allow viewing and editing of commits, PRs, and reviews

### Security Practices
- **Regular Audits**: Conduct security audits using available tools
- **Tools**: Use `cargo-audit`, `cargo-fuzz`, clippy for Rust-specific security
- **Reference Documentation**: 
  - `~/.agents/security/code-security-audit.md`
  - `~/.agents/security/sql-security-audit.md`
- **Dependency Security**: Monitor for vulnerable dependencies

### Development Workflow
1. **Planning**: Create step-by-step plans with clear checkboxes
2. **Implementation**: Follow TDD methodology strictly
3. **Testing**: Comprehensive test coverage with automated validation
4. **Review**: Code reviews for quality and consistency
5. **Security**: Regular security audits and vulnerability assessments
6. **Documentation**: Maintain clear, professional documentation

## Reference Documentation

The following files in `~/.agents/` should be checked regularly for guidance:

### Development Guidelines
- `~/.agents/development/tdd-development.md` - Complete TDD methodology with Red-Green-Blue cycle
- `~/.agents/development/development.md` - General development practices and tools
- `~/.agents/development/version-control.md` - Git and GitHub workflow standards

### Security Guidelines
- `~/.agents/security/code-security-audit.md` - Comprehensive security audit procedures
- `~/.agents/security/sql-security-audit.md` - Database-specific security guidelines

## Current Architecture

### Core Components
- `src/main.rs` - Entry point and CLI interface
- `src/db.rs` - Database module with CRUD operations
- `atask.db` - LibSQL database file
- `Cargo.toml` - Project configuration and dependencies

### Database Schema
- **commits**: Git commit tracking with file changes and statistics
- **issues**: Task management with status, priority, and labels
- **labels**: Customizable categorization system
- **issue_labels**: Many-to-many relationship between issues and labels

### Key Features Implemented
- Git history integration with automatic commit import
- CRUD operations for commits, issues, and labels
- Default GitHub-style label system
- Comprehensive error handling with proper date parsing
- Sample data generation for demonstration

## Development History

### Completed Features
- [x] LibSQL database integration and schema design
- [x] Git commit history parsing and import
- [x] CRUD operations for all entities
- [x] Default label system with GitHub-style colors
- [x] Issue management with status and priority tracking
- [x] Many-to-many label associations
- [x] Robust date parsing for different timestamp formats
- [x] Comprehensive README documentation
- [x] Project setup with proper SSH credentials

### Next Steps
- [ ] Implement comprehensive test suite following TDD methodology
- [ ] Add CLI commands for issue management
- [ ] Conduct security audit using `~/.agents/security/` guidelines
- [ ] Add integration tests for database operations
- [ ] Implement error handling improvements
- [ ] Add configuration management
- [ ] Create web interface foundation

## Testing Strategy

### Current Status
- **Test Framework**: Rust built-in testing with `cargo test`
- **Coverage**: Basic integration testing needed
- **TDD Compliance**: Future development must follow strict TDD methodology

### Testing Requirements
- Unit tests for all CRUD operations
- Integration tests for Git parsing functionality
- Error handling test coverage
- Database schema validation tests
- Security vulnerability testing

## Quality Assurance Checklist

### Pre-Commit Checklist
- [ ] All tests pass (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Linter passes (`cargo clippy`)
- [ ] Security audit clean (using `~/.agents/security/` tools)
- [ ] Documentation updated
- [ ] TDD methodology followed for new features

### Regular Maintenance
- [ ] Weekly security audits
- [ ] Monthly dependency updates
- [ ] Quarterly architecture review
- [ ] Continuous integration setup

## Notes

- Project configured for personal GitHub account using `github.folknology` SSH credentials
- Database file (`atask.db`) is included in repository for demo purposes
- All new development must strictly follow TDD Red-Green-Blue methodology
- Reference `~/.agents/` documentation for all development practices
- Maintain professional code style without emojis/unicode symbols
