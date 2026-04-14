"""Bounded serial monitor for DICE firmware bring-up tests.

Opens a COM port, reads lines for a fixed duration, and exits.
Unlike `idf.py monitor` which runs indefinitely, this gives us a
scripted way to capture boot logs + steady-state behavior.

Usage:
    python firmware/bounded_monitor.py COM7 30
"""
import sys
import time

import serial

def main():
    if len(sys.argv) < 3:
        print("usage: bounded_monitor.py PORT SECONDS")
        sys.exit(2)

    port = sys.argv[1]
    duration = int(sys.argv[2])

    print(f"[monitor] opening {port} @ 115200 for {duration}s")
    # Reset the ESP32-S3 via RTS/DTR toggling (matches idf.py monitor --no-reset=False).
    # Without this the device may be mid-boot from the previous flash and we'd
    # miss the log lines.
    ser = serial.Serial(
        port=port,
        baudrate=115200,
        bytesize=serial.EIGHTBITS,
        parity=serial.PARITY_NONE,
        stopbits=serial.STOPBITS_ONE,
        timeout=0.5,  # read timeout
    )
    # Toggle RTS+DTR to reset the chip so we catch the boot banner.
    ser.dtr = False
    ser.rts = True
    time.sleep(0.1)
    ser.rts = False
    ser.dtr = False
    time.sleep(0.05)

    start = time.monotonic()
    buf = bytearray()
    total_lines = 0
    while time.monotonic() - start < duration:
        chunk = ser.read(1024)
        if chunk:
            buf.extend(chunk)
            # Flush any complete lines
            while True:
                nl = buf.find(b"\n")
                if nl < 0:
                    break
                line = bytes(buf[:nl]).decode("utf-8", errors="replace").rstrip("\r")
                buf[:] = buf[nl + 1:]
                print(line, flush=True)
                total_lines += 1

    # Print any trailing bytes
    if buf:
        print(buf.decode("utf-8", errors="replace"), end="", flush=True)

    ser.close()
    print(f"\n[monitor] done — {total_lines} lines over {duration}s")

if __name__ == "__main__":
    main()
