"use client"

import { motion } from "framer-motion"
import { CornerBox } from "@/components/shared/CornerBox"
import { BracketLink } from "@/components/shared/BracketButton"
import { SectionHeader } from "@/components/landing/HowItWorks"

const fadeUp = {
  hidden: { opacity: 0, y: 16 },
  visible: (i: number) => ({
    opacity: 1, y: 0,
    transition: { delay: 0.1 * i, duration: 0.5, ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number] },
  }),
}

/**
 * Operator pitch — delivered as a three-box diagram instead of prose cards.
 * The scroll-driven ESP32 tour handles the hardware story; this block
 * converts the "why run one" into three pixel-font readouts.
 */
export function ForOperators() {
  return (
    <section className="relative py-28 border-b border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <SectionHeader
          index="04"
          name="operators"
          title="Run a node. Earn SOL."
          lead="No cloud, no staking, no lockups. $8 of hardware on your shelf joins the entropy mesh the moment it pings the coordinator."
        />

        <div className="mt-12 grid grid-cols-1 sm:grid-cols-3 gap-5">
          {(
            [
              { k: "DEVICE", v: "$8", unit: "esp32-s3" },
              { k: "REVENUE SHARE", v: "70%", unit: "per request" },
              { k: "MAINTENANCE", v: "0", unit: "once online" },
            ] as const
          ).map((s, i) => (
            <motion.div
              key={s.k}
              custom={i}
              variants={fadeUp}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-40px" }}
            >
              <CornerBox title={s.k}>
                <p className="font-pixel text-foreground text-5xl md:text-6xl tabular-nums leading-none">
                  {s.v}
                </p>
                <p className="mt-3 ascii-label text-[10px]">{s.unit}</p>
              </CornerBox>
            </motion.div>
          ))}
        </div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true, margin: "-40px" }}
          transition={{ duration: 0.4, delay: 0.25 }}
          className="mt-10"
        >
          <BracketLink href="/preorder" variant="primary">Pre-order_a_Node</BracketLink>
        </motion.div>
      </div>
    </section>
  )
}

export default ForOperators
