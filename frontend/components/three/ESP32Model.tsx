"use client"

import { useRef } from "react"
import { Group } from "three"
import { RoundedBox } from "@react-three/drei"

interface ESP32ModelProps {
  explode?: number // 0 = assembled, 1 = fully exploded
}

export function ESP32Model({ explode = 0 }: ESP32ModelProps) {
  const groupRef = useRef<Group>(null)

  return (
    <group ref={groupRef}>
      {/* ──────────────── PCB Board ──────────────── */}
      <group position={[0, explode * -1, 0]}>
        {/* Main board */}
        <RoundedBox args={[2.8, 0.08, 1.3]} radius={0.03} smoothness={4}>
          <meshPhysicalMaterial
            color="#1a5c2e"
            roughness={0.6}
            metalness={0.1}
          />
        </RoundedBox>

        {/* Board underside — slightly darker green */}
        <mesh position={[0, -0.042, 0]}>
          <boxGeometry args={[2.75, 0.005, 1.25]} />
          <meshPhysicalMaterial color="#144d24" roughness={0.7} metalness={0.1} />
        </mesh>

        {/* ── Circuit traces (copper lines on PCB) ── */}
        {/* Horizontal traces */}
        {[0.3, 0.1, -0.1, -0.25, 0.45].map((z, i) => (
          <mesh key={`ht-${i}`} position={[0, 0.042, z]}>
            <boxGeometry args={[2.2, 0.002, 0.015]} />
            <meshStandardMaterial color="#2a7a42" roughness={0.4} metalness={0.3} />
          </mesh>
        ))}
        {/* Vertical traces */}
        {[-0.8, -0.4, 0, 0.4, 0.8, -0.6, 0.6].map((x, i) => (
          <mesh key={`vt-${i}`} position={[x, 0.042, 0]}>
            <boxGeometry args={[0.015, 0.002, 0.9]} />
            <meshStandardMaterial color="#2a7a42" roughness={0.4} metalness={0.3} />
          </mesh>
        ))}
        {/* Diagonal trace accents */}
        <mesh position={[0.55, 0.042, 0.2]} rotation={[0, 0.6, 0]}>
          <boxGeometry args={[0.4, 0.002, 0.012]} />
          <meshStandardMaterial color="#2a7a42" roughness={0.4} metalness={0.3} />
        </mesh>
        <mesh position={[-0.55, 0.042, -0.2]} rotation={[0, -0.5, 0]}>
          <boxGeometry args={[0.35, 0.002, 0.012]} />
          <meshStandardMaterial color="#2a7a42" roughness={0.4} metalness={0.3} />
        </mesh>

        {/* ── Solder pads (small copper squares) ── */}
        {[
          [-1.0, 0.3], [-1.0, 0.1], [-1.0, -0.1], [-1.0, -0.3],
          [1.0, 0.3], [1.0, 0.1], [1.0, -0.1], [1.0, -0.3],
        ].map(([x, z], i) => (
          <mesh key={`pad-${i}`} position={[x, 0.042, z]}>
            <boxGeometry args={[0.06, 0.002, 0.06]} />
            <meshStandardMaterial color="#b87333" roughness={0.3} metalness={0.8} />
          </mesh>
        ))}

        {/* ── Silkscreen text outlines (tiny white rectangles as labels) ── */}
        <mesh position={[0.9, 0.042, -0.45]}>
          <boxGeometry args={[0.3, 0.001, 0.06]} />
          <meshStandardMaterial color="#d4d4d4" roughness={0.9} metalness={0} />
        </mesh>
        <mesh position={[-0.9, 0.042, -0.45]}>
          <boxGeometry args={[0.25, 0.001, 0.06]} />
          <meshStandardMaterial color="#d4d4d4" roughness={0.9} metalness={0} />
        </mesh>
      </group>

      {/* ──────────────── ESP32-S3 Main SoC ──────────────── */}
      <group position={[0, 0.06 + explode * 1.5, 0]}>
        {/* Chip body */}
        <RoundedBox args={[0.7, 0.1, 0.7]} radius={0.02} smoothness={4}>
          <meshPhysicalMaterial
            color="#1a1a1a"
            roughness={0.3}
            metalness={0.5}
          />
        </RoundedBox>
        {/* Chip label (metallic dot/marking) */}
        <mesh position={[-0.22, 0.052, -0.22]}>
          <cylinderGeometry args={[0.04, 0.04, 0.005, 16]} />
          <meshStandardMaterial color="#666666" roughness={0.2} metalness={0.8} />
        </mesh>
        {/* Chip label line */}
        <mesh position={[0.05, 0.052, 0]}>
          <boxGeometry args={[0.3, 0.003, 0.04]} />
          <meshStandardMaterial color="#444444" roughness={0.5} metalness={0.3} />
        </mesh>
        {/* QFN pads around the chip (small metal bumps on each side) */}
        {Array.from({ length: 8 }).map((_, i) => (
          <mesh key={`qfn-t-${i}`} position={[(i - 3.5) * 0.08, -0.05, -0.36]}>
            <boxGeometry args={[0.04, 0.02, 0.03]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
        {Array.from({ length: 8 }).map((_, i) => (
          <mesh key={`qfn-b-${i}`} position={[(i - 3.5) * 0.08, -0.05, 0.36]}>
            <boxGeometry args={[0.04, 0.02, 0.03]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
        {Array.from({ length: 8 }).map((_, i) => (
          <mesh key={`qfn-l-${i}`} position={[-0.36, -0.05, (i - 3.5) * 0.08]}>
            <boxGeometry args={[0.03, 0.02, 0.04]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
        {Array.from({ length: 8 }).map((_, i) => (
          <mesh key={`qfn-r-${i}`} position={[0.36, -0.05, (i - 3.5) * 0.08]}>
            <boxGeometry args={[0.03, 0.02, 0.04]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── USB-C Connector ──────────────── */}
      <group position={[0, 0.04 + explode * 0.8, -0.7]}>
        {/* Outer shell */}
        <RoundedBox args={[0.45, 0.15, 0.3]} radius={0.03} smoothness={4}>
          <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
        </RoundedBox>
        {/* USB port opening */}
        <mesh position={[0, 0, -0.151]}>
          <boxGeometry args={[0.32, 0.08, 0.01]} />
          <meshStandardMaterial color="#333333" roughness={0.8} metalness={0.3} />
        </mesh>
        {/* Inner contacts */}
        <mesh position={[0, 0, -0.14]}>
          <boxGeometry args={[0.25, 0.02, 0.02]} />
          <meshStandardMaterial color="#b87333" roughness={0.3} metalness={0.9} />
        </mesh>
      </group>

      {/* ──────────────── LED Indicators ──────────────── */}
      {/* Green LED (power) */}
      <group position={[-0.3, 0.06 + explode * 1.2, 0.5]}>
        <mesh>
          <boxGeometry args={[0.08, 0.04, 0.08]} />
          <meshStandardMaterial
            color="#00FF85"
            emissive="#00FF85"
            emissiveIntensity={2}
          />
        </mesh>
        {/* LED diffuser glow */}
        <mesh position={[0, 0.025, 0]}>
          <sphereGeometry args={[0.05, 12, 12]} />
          <meshStandardMaterial
            color="#00FF85"
            emissive="#00FF85"
            emissiveIntensity={1.5}
            transparent
            opacity={0.3}
          />
        </mesh>
      </group>

      {/* Blue LED (activity) */}
      <group position={[-0.15, 0.06 + explode * 1.2, 0.5]}>
        <mesh>
          <boxGeometry args={[0.08, 0.04, 0.08]} />
          <meshStandardMaterial
            color="#3b82f6"
            emissive="#3b82f6"
            emissiveIntensity={1.5}
          />
        </mesh>
        <mesh position={[0, 0.025, 0]}>
          <sphereGeometry args={[0.05, 12, 12]} />
          <meshStandardMaterial
            color="#3b82f6"
            emissive="#3b82f6"
            emissiveIntensity={1}
            transparent
            opacity={0.25}
          />
        </mesh>
      </group>

      {/* Red LED (error) — dimmer, off by default look */}
      <group position={[0.0, 0.06 + explode * 1.2, 0.5]}>
        <mesh>
          <boxGeometry args={[0.08, 0.04, 0.08]} />
          <meshStandardMaterial
            color="#ef4444"
            emissive="#ef4444"
            emissiveIntensity={0.3}
          />
        </mesh>
      </group>

      {/* ──────────────── WiFi Antenna Area ──────────────── */}
      <group position={[0, 0.05 + explode * 1, 0.55]}>
        {/* Copper antenna trace */}
        <RoundedBox args={[1.2, 0.02, 0.15]} radius={0.01} smoothness={4}>
          <meshStandardMaterial color="#b87333" roughness={0.3} metalness={0.9} />
        </RoundedBox>
        {/* Meandering antenna pattern (zig-zag lines) */}
        {Array.from({ length: 6 }).map((_, i) => (
          <mesh key={`ant-${i}`} position={[-0.4 + i * 0.16, 0.012, 0]}>
            <boxGeometry args={[0.02, 0.005, 0.1]} />
            <meshStandardMaterial color="#c9893d" roughness={0.3} metalness={0.9} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── Pin Headers — Left ──────────────── */}
      <group position={[-1.35, 0.1 + explode * 0.5, 0]}>
        {/* Black plastic housing */}
        <mesh position={[0, -0.02, 0]}>
          <boxGeometry args={[0.12, 0.1, 1.2]} />
          <meshStandardMaterial color="#222222" roughness={0.8} metalness={0.1} />
        </mesh>
        {Array.from({ length: 10 }).map((_, i) => (
          <mesh key={`pin-l-${i}`} position={[0, 0, (i - 4.5) * 0.12]}>
            <cylinderGeometry args={[0.02, 0.02, 0.25, 8]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── Pin Headers — Right ──────────────── */}
      <group position={[1.35, 0.1 + explode * 0.5, 0]}>
        {/* Black plastic housing */}
        <mesh position={[0, -0.02, 0]}>
          <boxGeometry args={[0.12, 0.1, 1.2]} />
          <meshStandardMaterial color="#222222" roughness={0.8} metalness={0.1} />
        </mesh>
        {Array.from({ length: 10 }).map((_, i) => (
          <mesh key={`pin-r-${i}`} position={[0, 0, (i - 4.5) * 0.12]}>
            <cylinderGeometry args={[0.02, 0.02, 0.25, 8]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── Voltage Regulator (AMS1117) ──────────────── */}
      <group position={[0.8, 0.06 + explode * 1.3, -0.3]}>
        <RoundedBox args={[0.25, 0.08, 0.2]} radius={0.01} smoothness={4}>
          <meshPhysicalMaterial color="#1a1a1a" roughness={0.4} metalness={0.4} />
        </RoundedBox>
        {/* Heat pad */}
        <mesh position={[0, 0, 0.11]}>
          <boxGeometry args={[0.2, 0.06, 0.02]} />
          <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
        </mesh>
        {/* Legs */}
        {[-0.08, 0, 0.08].map((x, i) => (
          <mesh key={`vreg-${i}`} position={[x, -0.04, -0.11]}>
            <boxGeometry args={[0.03, 0.02, 0.04]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── Crystal Oscillator ──────────────── */}
      <group position={[-0.6, 0.06 + explode * 1.1, -0.15]}>
        <RoundedBox args={[0.18, 0.06, 0.12]} radius={0.01} smoothness={4}>
          <meshStandardMaterial color="#C0C0C0" roughness={0.3} metalness={0.9} />
        </RoundedBox>
        {/* Pads */}
        {[-0.06, 0.06].map((x, i) => (
          <mesh key={`xtal-${i}`} position={[x, -0.03, 0]}>
            <boxGeometry args={[0.04, 0.01, 0.08]} />
            <meshStandardMaterial color="#999999" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── Reset Button ──────────────── */}
      <group position={[0.5, 0.06 + explode * 1.1, 0.4]}>
        {/* Button housing */}
        <RoundedBox args={[0.15, 0.08, 0.15]} radius={0.01} smoothness={4}>
          <meshStandardMaterial color="#333333" roughness={0.6} metalness={0.2} />
        </RoundedBox>
        {/* Button cap */}
        <mesh position={[0, 0.045, 0]}>
          <cylinderGeometry args={[0.04, 0.04, 0.02, 16]} />
          <meshStandardMaterial color="#666666" roughness={0.3} metalness={0.5} />
        </mesh>
        {/* Legs */}
        {[
          [-0.06, -0.06],
          [0.06, -0.06],
          [-0.06, 0.06],
          [0.06, 0.06],
        ].map(([x, z], i) => (
          <mesh key={`btn-${i}`} position={[x, -0.04, z]}>
            <boxGeometry args={[0.02, 0.02, 0.02]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── Boot Button ──────────────── */}
      <group position={[-0.5, 0.06 + explode * 1.1, 0.4]}>
        <RoundedBox args={[0.15, 0.08, 0.15]} radius={0.01} smoothness={4}>
          <meshStandardMaterial color="#333333" roughness={0.6} metalness={0.2} />
        </RoundedBox>
        <mesh position={[0, 0.045, 0]}>
          <cylinderGeometry args={[0.04, 0.04, 0.02, 16]} />
          <meshStandardMaterial color="#555555" roughness={0.3} metalness={0.5} />
        </mesh>
      </group>

      {/* ──────────────── Capacitors (brown/beige ceramic) ──────────────── */}
      {[
        { pos: [0.4, 0.06, 0.15] as const, size: [0.1, 0.05, 0.06] as const, color: "#8B6914" },
        { pos: [-0.4, 0.06, 0.15] as const, size: [0.1, 0.05, 0.06] as const, color: "#8B6914" },
        { pos: [0.2, 0.06, -0.35] as const, size: [0.08, 0.04, 0.05] as const, color: "#9B7B2C" },
        { pos: [-0.2, 0.06, -0.35] as const, size: [0.08, 0.04, 0.05] as const, color: "#9B7B2C" },
        { pos: [0.65, 0.06, 0.1] as const, size: [0.06, 0.04, 0.04] as const, color: "#8B6914" },
        { pos: [-0.65, 0.06, 0.1] as const, size: [0.06, 0.04, 0.04] as const, color: "#8B6914" },
      ].map(({ pos, size, color }, i) => (
        <mesh
          key={`cap-${i}`}
          position={[pos[0], pos[1] + explode * 0.9, pos[2]]}
        >
          <boxGeometry args={[size[0], size[1], size[2]]} />
          <meshStandardMaterial color={color} roughness={0.7} metalness={0.2} />
        </mesh>
      ))}

      {/* ──────────────── Resistors (tiny dark rectangles) ──────────────── */}
      {[
        [0.3, -0.2],
        [-0.3, -0.2],
        [0.6, -0.4],
        [-0.6, -0.4],
        [0.15, 0.25],
        [-0.15, 0.25],
        [0.85, 0.2],
        [-0.85, 0.2],
      ].map(([x, z], i) => (
        <mesh
          key={`res-${i}`}
          position={[x, 0.055 + explode * 0.7, z]}
        >
          <boxGeometry args={[0.07, 0.03, 0.035]} />
          <meshStandardMaterial color="#2a2a2a" roughness={0.6} metalness={0.3} />
        </mesh>
      ))}

      {/* ──────────────── Electrolytic Capacitor (small cylinder) ──────────────── */}
      <group position={[0.9, 0.1 + explode * 1.4, 0.3]}>
        <mesh>
          <cylinderGeometry args={[0.08, 0.08, 0.15, 16]} />
          <meshStandardMaterial color="#222222" roughness={0.5} metalness={0.3} />
        </mesh>
        {/* Top marking */}
        <mesh position={[0, 0.076, 0]}>
          <cylinderGeometry args={[0.065, 0.065, 0.005, 16]} />
          <meshStandardMaterial color="#444444" roughness={0.6} metalness={0.2} />
        </mesh>
        {/* Stripe */}
        <mesh position={[0.075, 0, 0]} rotation={[0, 0, Math.PI / 2]}>
          <boxGeometry args={[0.15, 0.005, 0.04]} />
          <meshStandardMaterial color="#888888" roughness={0.5} metalness={0.2} />
        </mesh>
      </group>

      {/* ──────────────── Flash Memory Chip (small IC) ──────────────── */}
      <group position={[-0.7, 0.06 + explode * 1.2, 0.2]}>
        <RoundedBox args={[0.3, 0.06, 0.2]} radius={0.01} smoothness={4}>
          <meshPhysicalMaterial color="#1a1a1a" roughness={0.35} metalness={0.45} />
        </RoundedBox>
        {/* Pin 1 marker */}
        <mesh position={[-0.1, 0.032, -0.06]}>
          <sphereGeometry args={[0.015, 8, 8]} />
          <meshStandardMaterial color="#555555" roughness={0.4} metalness={0.3} />
        </mesh>
        {/* SOP pins */}
        {Array.from({ length: 4 }).map((_, i) => (
          <mesh key={`flash-pt-${i}`} position={[-0.1 + i * 0.065, -0.03, -0.105]}>
            <boxGeometry args={[0.03, 0.01, 0.03]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
        {Array.from({ length: 4 }).map((_, i) => (
          <mesh key={`flash-pb-${i}`} position={[-0.1 + i * 0.065, -0.03, 0.105]}>
            <boxGeometry args={[0.03, 0.01, 0.03]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── CP2102 USB-UART Bridge (small IC near USB) ──────────────── */}
      <group position={[0.4, 0.06 + explode * 1.15, -0.4]}>
        <RoundedBox args={[0.22, 0.05, 0.22]} radius={0.01} smoothness={4}>
          <meshPhysicalMaterial color="#1e1e1e" roughness={0.35} metalness={0.4} />
        </RoundedBox>
        {/* Pins */}
        {Array.from({ length: 4 }).map((_, i) => (
          <mesh key={`uart-l-${i}`} position={[-0.115, -0.025, -0.07 + i * 0.045]}>
            <boxGeometry args={[0.025, 0.01, 0.02]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
        {Array.from({ length: 4 }).map((_, i) => (
          <mesh key={`uart-r-${i}`} position={[0.115, -0.025, -0.07 + i * 0.045]}>
            <boxGeometry args={[0.025, 0.01, 0.02]} />
            <meshStandardMaterial color="#C0C0C0" roughness={0.2} metalness={1.0} />
          </mesh>
        ))}
      </group>

      {/* ──────────────── Via holes (small metallic circles on PCB) ──────────────── */}
      {[
        [0.2, 0.0], [-0.2, 0.0], [0.5, 0.3], [-0.5, 0.3],
        [0.7, -0.1], [-0.7, -0.1], [0.3, -0.5], [-0.3, -0.5],
        [0.0, 0.35], [0.0, -0.15],
      ].map(([x, z], i) => (
        <mesh
          key={`via-${i}`}
          position={[x, 0.042 + explode * -1, z]}
          rotation={[Math.PI / 2, 0, 0]}
        >
          <torusGeometry args={[0.02, 0.006, 8, 16]} />
          <meshStandardMaterial color="#b87333" roughness={0.3} metalness={0.9} />
        </mesh>
      ))}
    </group>
  )
}
