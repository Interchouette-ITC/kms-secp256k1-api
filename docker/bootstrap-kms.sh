#!/bin/bash
set -euo pipefail

rm -f /tmp/kms.ready

if [ -z "${AWS_REGION:-}" ]; then
  echo "AWS_REGION is not set"
  exit 1
fi

echo "Waiting for LocalStack KMS in region ${AWS_REGION}..."

for _ in $(seq 1 60); do
  if awslocal kms list-keys --region "${AWS_REGION}" >/dev/null 2>&1; then
    touch /tmp/kms.ready
    echo "LocalStack KMS is ready"
    exit 0
  fi
  sleep 2
done

echo "Timed out waiting for LocalStack KMS"
exit 1
