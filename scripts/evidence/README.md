# Evidence primitives

`manifest.mjs` hashes repository-relative artifact files, rejects absolute or
parent-escaping paths, sorts artifacts deterministically, and records only
classified metadata. It never embeds artifact contents. Secret/page-content
redaction remains a separate required boundary before evidence creation.
