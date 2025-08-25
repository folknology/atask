#!/bin/bash

# GitHub CLI commands to set up workflow labels and create feature request
# Run these commands after authenticating with: gh auth login

echo "Creating workflow labels for project management..."

# Create workflow stage labels
gh label create "Evaluating" --description "Items being evaluated or analyzed" --color "fef2c0" 
gh label create "Preparing" --description "Items being prepared or planned" --color "fbca04"
gh label create "Progressing" --description "Items actively being worked on" --color "0e8a16"
gh label create "Done" --description "Completed items" --color "6f42c1"

echo "Creating feature request issue..."

# Create the feature request issue
gh issue create \
  --title "Add workflow management with stage labels" \
  --body "## Feature Request: Workflow Management with Stage Labels

### Description
Add the ability to sort and organize work using workflow stage labels to better track project progress.

### Proposed Workflow Labels
- **Evaluating** - Items being evaluated or analyzed
- **Preparing** - Items being prepared or planned  
- **Progressing** - Items actively being worked on
- **Done** - Completed items

### Requirements
1. Create the four workflow stage labels in the repository
2. Update the ATask database to support these workflow labels
3. Add functionality to filter and sort issues by workflow stage
4. Integrate with existing label system in the database

### Acceptance Criteria
- [ ] Workflow labels created in GitHub repository
- [ ] Database supports the new workflow labels
- [ ] Issues can be tagged with workflow stages
- [ ] CLI can filter issues by workflow stage
- [ ] Documentation updated with workflow process

### Implementation Notes
This should integrate with the existing label system in `src/db.rs` and may require:
- Database migration to add workflow-specific fields
- CLI commands for workflow management
- Integration with GitHub API for syncing labels

### Labels
- enhancement
- Evaluating" \
  --label "enhancement,Evaluating"

echo "Setup complete! Labels and feature request created."
