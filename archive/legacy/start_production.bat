@echo off
rem Set DATABASE_URL in your environment first. Never hardcode it here.
if not defined DATABASE_URL ( echo DATABASE_URL is not set & exit /b 1 )
target\debug\dice-coordinator.exe --ws-port 9001 --api-port 8080 --metrics-port 9090 --min-nodes 1 --max-nodes 7 --tls-cert-path pki\production\coordinator.crt --tls-key-path pki\production\coordinator.key --ca-cert-path pki\production\ca_bundle.crt --database-url "%DATABASE_URL%"
