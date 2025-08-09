---
inclusion: always
---

# Task Management & Development Workflow

## Task Prioritization
- Use clear priority levels: Critical, High, Medium, Low
- Critical: Blocking issues, security vulnerabilities, production failures
- High: Core features, performance issues, user-facing bugs
- Medium: Enhancements, refactoring, technical debt
- Low: Nice-to-have features, documentation improvements

## Task Documentation
- Include clear acceptance criteria for each task
- Reference related issues, PRs, or documentation
- Estimate complexity using story points or time estimates
- Tag tasks with relevant components (core, bindings, tests, docs)

## Development Workflow
- Break large features into smaller, testable increments
- Complete tasks in dependency order
- Write tests before marking tasks complete
- Update documentation when adding new features
- Review code quality before task completion

## Task Status Tracking
- TODO: Not started, clearly defined
- IN_PROGRESS: Actively being worked on
- BLOCKED: Waiting on dependencies or external factors
- REVIEW: Code complete, awaiting review
- TESTING: Under validation or QA
- DONE: Completed and verified

## Code Completion Standards
- All new code must include appropriate tests
- Public APIs require documentation
- Breaking changes need migration guides
- Performance-critical code needs benchmarks
- Multi-language bindings require validation across all targets

## Task Categories
- FEATURE: New functionality or capabilities
- BUG: Defect fixes and error corrections
- REFACTOR: Code improvement without behavior changes
- DOCS: Documentation updates and improvements
- TEST: Test coverage and quality improvements
- CHORE: Maintenance tasks and tooling updates