@echo off
set DATABASE_URL=postgresql://neondb_owner:npg_mr5BDTjnRFz9@ep-rough-unit-amap2seh-pooler.c-5.us-east-1.aws.neon.tech/neondb?sslmode=require
target\debug\dice-coordinator.exe --ws-port 9001 --api-port 8080 --metrics-port 9090 --min-nodes 1 --max-nodes 1 --tls-cert-path certs\coordinator.crt --tls-key-path certs\coordinator.key --ca-cert-path certs\ca.crt
