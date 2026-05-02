#!/usr/bin/env python3
"""One-shot serial dump for quick diagnostics.

Usage: python _serial_dump.py [PORT] [SECONDS]
"""
import sys, time
try:
    import serial
except ImportError:
    print("pyserial not installed in this Python env", file=sys.stderr)
    sys.exit(1)

port = sys.argv[1] if len(sys.argv) > 1 else 'COM11'
secs = int(sys.argv[2]) if len(sys.argv) > 2 else 8
baud = 115200

print(f"Reading {port} @ {baud} baud for {secs}s ...\n---")
try:
    s = serial.Serial(port, baud, timeout=0.5)
except Exception as e:
    print(f"OPEN FAILED: {e}")
    sys.exit(1)

print(">>> PRESS RESET ON THE BOARD NOW (or any button that resets) <<<")
s.reset_input_buffer()

end = time.time() + secs
total_bytes = 0
last_report = time.time()
while time.time() < end:
    chunk = s.read(512)
    if chunk:
        total_bytes += len(chunk)
        try:
            sys.stdout.write(chunk.decode('utf-8', errors='replace'))
            sys.stdout.flush()
        except Exception:
            sys.stdout.write(repr(chunk))
            sys.stdout.flush()
    # Heartbeat every 3s to show we're alive
    if time.time() - last_report > 3:
        sys.stderr.write(f"\n[heartbeat: {total_bytes} bytes received so far]\n")
        sys.stderr.flush()
        last_report = time.time()
s.close()
print(f"\n--- total bytes: {total_bytes} ---")
print("---\ndone")
