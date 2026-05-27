import { cubicBezier } from "motion";
import { motion } from "motion/react";

const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";
const EASE = cubicBezier(0.25, 0.46, 0.45, 0.94);

const commands = [
  "wxpilot start",
  "wxpilot connect",
  "wxpilot view",
  "wxpilot state",
  "wxpilot shot result.png",
];

export function CLIShowcase() {
  return (
    <section
      id="cli"
      style={{
        background: "#0d1117",
        borderTop: "1px solid rgba(50,68,90,0.4)",
        padding: "96px 0",
        fontFamily: SANS,
      }}
    >
      <div className="max-w-6xl mx-auto px-6">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 64, alignItems: "center" }} className="cli-grid">
          {/* Left: text */}
          <motion.div
            initial={{ opacity: 0, x: -28 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true, margin: "-72px" }}
            transition={{ duration: 0.6, ease: EASE }}
          >
            <div style={{ fontSize: 11, letterSpacing: "0.12em", color: "#4d8a6a", textTransform: "uppercase", marginBottom: 16, fontFamily: MONO }}>
              CLI
            </div>
            <h2 style={{ fontSize: "clamp(24px, 3vw, 36px)", color: "#e8e6e1", fontWeight: 500, lineHeight: 1.25, letterSpacing: "-0.02em", margin: "0 0 16px 0" }}>
              保持简洁，也保持可控
            </h2>
            <p style={{ fontSize: 16, color: "#7a8ba8", lineHeight: 1.75, margin: "0 0 32px 0" }}>
              围绕清晰的命令与稳定的执行体验设计，方便接入你的开发与自动化流程。
            </p>

            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {[
                "清晰的命令语义",
                "稳定的执行反馈",
                "方便接入自动化管线",
              ].map((item) => (
                <div key={item} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <div style={{ width: 5, height: 5, borderRadius: "50%", background: "#4d8a6a", flexShrink: 0 }} />
                  <span style={{ fontSize: 14, color: "#7a8ba8" }}>{item}</span>
                </div>
              ))}
            </div>
          </motion.div>

          {/* Right: terminal */}
          <motion.div
            initial={{ opacity: 0, x: 28 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true, margin: "-72px" }}
            transition={{ duration: 0.6, ease: EASE, delay: 0.1 }}
          >
            <div
              style={{
                background: "#0a0f16",
                border: "1px solid rgba(50,68,90,0.5)",
                borderRadius: 12,
                overflow: "hidden",
                boxShadow: "0 0 0 1px rgba(77,138,106,0.08), 0 32px 80px rgba(0,0,0,0.6), 0 0 40px rgba(77,138,106,0.04)",
                fontFamily: MONO,
              }}
            >
              {/* Window chrome */}
              <div
                style={{
                  height: 38,
                  background: "#0d1117",
                  borderBottom: "1px solid rgba(50,68,90,0.45)",
                  display: "flex",
                  alignItems: "center",
                  padding: "0 14px",
                  gap: 6,
                }}
              >
                <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#3a2520", display: "block" }} />
                <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#3a3020", display: "block" }} />
                <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#1a3020", display: "block" }} />
                <span style={{ fontSize: 11, color: "#3a4a5a", marginLeft: 8 }}>terminal</span>
              </div>

              {/* Terminal body */}
              <div style={{ padding: "24px 20px", display: "flex", flexDirection: "column", gap: 12 }}>
                {commands.map((cmd, i) => (
                  <div key={cmd} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ color: "#4d8a6a", fontSize: 13, userSelect: "none", flexShrink: 0 }}>$</span>
                    <span style={{ color: "#c8d4e0", fontSize: 13, fontFamily: MONO }}>{cmd}</span>
                    {i === commands.length - 1 && (
                      <span style={{ color: "#4d8a6a", fontSize: 13, animation: "blink 1.2s step-end infinite" }}>▋</span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </motion.div>
        </div>
      </div>

      <style>{`
        @keyframes blink { 0%,50%{opacity:1}51%,100%{opacity:0} }
        .cli-grid { grid-template-columns: 1fr 1fr; }
        @media (max-width: 768px) {
          .cli-grid { grid-template-columns: 1fr !important; }
        }
      `}</style>
    </section>
  );
}
