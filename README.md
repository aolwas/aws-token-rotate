# aws-token-rotate
Simple rust tool to easily rotate AWS token: using current profile, it creates new credentials, saves them
and drops the old ones.

Use AWS_SHARED_CREDENTIALS_FILE (default: `$HOME/.aws/credentials`) and/or AWS_PROFILE (default: `default`) envvars
to specify alternative file and/or profile.

# Installation

This package has only been tested on Ubuntu/Debian Linux OS and MacOS.

## Releases

Pre-built binaries are provided for Linux and MacOS, both in x86_64 and aarch64 variants

## Cargo

```bash
cargo install aws-token-rotate
```

## Sources

```bash
git clone https://github.com/aolwas/aws-token-rotate.git
cd aws-token-rotate
cargo install --path .
```
