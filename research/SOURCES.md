---
title: "Official Research Sources"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Research, Architecture, and Legal"
purpose: "Provide the official-source register used for architecture, tool interoperability, standards, and competitive analysis."
---

# Official Research Sources

Research snapshot date: **2026-08-08**. Revalidate version-sensitive sources when implementation begins and before GA. Public vendor claims remain claims until reproduced.

| Authority | Source | URL | Decision supported |
| --- | --- | --- | --- |
| Google | Autonomous Antigravity pipelines codelab | https://codelabs.developers.google.com/autonomous-ai-developer-pipelines-antigravity | Agents, skills, workflows, iterative handoffs |
| Google | Agents in AI Studio Playground | https://ai.google.dev/gemini-api/docs/aistudio-agents | Managed agent tools, environment, AGENTS.md and skills |
| OpenAI | Codex learning hub | https://developers.openai.com/learn/codex | Codex entry point and workflow references |
| OpenAI | Skills for OSS maintenance | https://developers.openai.com/blog/skills-agents-sdk | AGENTS.md, repo-local skills, CI reuse |
| OpenAI | Agent Skills guide | https://developers.openai.com/api/docs/guides/tools-skills | SKILL.md bundles and versioned reusable workflows |
| Anthropic | Claude Code subagents | https://code.claude.com/docs/en/sub-agents | Specialist agents and independent contexts |
| Anthropic | Claude Code parallel agents | https://code.claude.com/docs/en/agents | Subagents, agent view, teams, dynamic workflows |
| Anthropic | Claude Code agent teams | https://code.claude.com/docs/en/agent-teams | Shared task coordination and documented limitations |
| GitHub | Customize Copilot for a project | https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-copilot-overview | Repository instructions and custom agents |
| GitHub | Custom agents configuration | https://docs.github.com/en/copilot/reference/custom-agents-configuration | Agent profile frontmatter, tools, MCP, versioning |
| GitHub | MCP and Copilot cloud agent | https://docs.github.com/en/copilot/concepts/agents/cloud-agent/mcp-and-cloud-agent | MCP constraints and least-privilege tool review |
| Svelte | Svelte overview | https://svelte.dev/docs/svelte/overview | Compiler-oriented UI rationale |
| Svelte | SvelteKit introduction | https://svelte.dev/docs/kit/introduction | Full application framework rationale |
| Svelte | SvelteKit adapter-static | https://svelte.dev/docs/kit/adapter-static | Prerender/static and mixed rendering choices |
| Lightpanda | Documentation index | https://lightpanda.io/docs/index | Machine-native browser baseline and product claims |
| Lightpanda | Markdown and AXTree | https://lightpanda.io/docs/guides/markdown-axtree | Direct post-JS machine-readable outputs |
| Lightpanda | PandaScript | https://lightpanda.io/docs/usage/pandascript | Deterministic native agent scripts without LLM replay |
| Lightpanda | Agent tutorial | https://lightpanda.io/docs/guides/lightpanda-agent-tutorial | Record/save/replay and MCP usage |
| Lightpanda | Repository | https://github.com/lightpanda-io/browser | Public source/release baseline |
| Lightpanda | Licensing | https://github.com/lightpanda-io/browser/blob/main/LICENSING.md | AGPL-3.0-only default license |
| V8 | Embedding V8 | https://v8.dev/docs/embed | Embedding model and C++ integration |
| V8 | Building V8 | https://v8.dev/docs/build | Pinned reproducible build planning |
| V8 | V8 inspector | https://v8.dev/docs/inspector | Inspector integration for embedders |
| WPT | Web Platform Tests | https://web-platform-tests.org/ | Cross-browser standards test suite |
| WPT | Running tests | https://web-platform-tests.org/running-tests/index.html | Runner and CI planning |
| W3C | WebDriver BiDi | https://www.w3.org/TR/webdriver-bidi/ | Bidirectional browser automation protocol |
| Chromium | Chrome DevTools Protocol | https://chromedevtools.github.io/devtools-protocol/ | CDP domains and protocol source |
| Chromium | CDP tip-of-tree | https://chromedevtools.github.io/devtools-protocol/tot/ | Frequent changes and lack of guaranteed backwards compatibility |
| MCP | Model Context Protocol specification | https://modelcontextprotocol.io/specification/2026-07-28 | Capability/version negotiation and safe agent integration |
| OpenTelemetry | Documentation | https://opentelemetry.io/docs/ | Vendor-neutral traces, metrics, and logs |
| Kubernetes | Documentation | https://kubernetes.io/docs/home/ | Container orchestration and production operations |
| PostgreSQL | Current documentation | https://www.postgresql.org/docs/current/ | Durable relational control-plane state |

## Source-use rules

- Prefer standards bodies, official product documentation, and upstream project documentation.
- Use blogs only for implementation context when an official specification or reference is unavailable.
- Record the exact accessed revision or release for protocol schemas and dependencies.
- Do not use a search snippet as the sole implementation contract; open and archive the authoritative page or schema.
- For performance claims, retain raw benchmark inputs, scripts, environment details, and result artifacts.
- For license decisions, retain the actual license text and obtain legal review; this register is not legal advice.
- When a source changes materially, open a documentation task and update affected ADRs, contracts, and compatibility matrices.
