# Main branch protection

Configure `main` in the GitHub repository with:

- pull requests required; direct pushes disabled after bootstrap;
- one independent approving review, with CODEOWNERS review for owned paths;
- required `fast-gate` status check and merge queue;
- stale approvals dismissed on new commits;
- force pushes and branch deletion disabled;
- conversation resolution required;
- signed commits or the repository's verified-identity equivalent for release
  branches;
- no production, signing, or vault credentials in pull-request jobs.

The first repository owner must apply these settings after the initial local
bootstrap commit. This file records policy; it cannot change GitHub settings
from source control.
