@echo off
set "DATABASE_URL=postgresql://neondb_owner:npg_mr5BDTjnRFz9@ep-rough-unit-amap2seh-pooler.c-5.us-east-1.aws.neon.tech/neondb?sslmode=require"
target\debug\dice-coordinator.exe --ws-port 9001 --api-port 8080 --metrics-port 9090 --min-nodes 1 --max-nodes 7 --tls-cert-path pki\production\coordinator.crt --tls-key-path pki\production\coordinator.key --ca-cert-path pki\production\ca_bundle.crt --database-url "%DATABASE_URL%"
