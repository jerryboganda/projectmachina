# Toolchain provenance

Toolchain and external-runtime versions are recorded in
`toolchains/versions.toml`. Buf release digests are committed for the supported
Windows and Linux x86_64 binaries and are verified before hosted CI execution.

V8 and Chromium remain external compatibility/runtime inputs. Their versions and
source repositories are recorded, but runtime use requires an immutable source
revision, checksum/signature, build flags, and provenance artifact. The
clean-room policy does not permit treating a mutable source URL as release
integrity evidence.
