# aws-token-rotate
Simple rust tool to easily rotate AWS token: using current profile, it creates new credentials, saves them and drops the old ones.

Use AWS_SHARED_CREDENTIALS_FILE or AWS_PROFILE envvars to specify alternative file and/or profile.
