import { cubicBezier } from "motion";
import { motion } from "motion/react";

const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";
const EASE = cubicBezier(0.25, 0.46, 0.45, 0.94);

const steps = [
  { label: "Start project", status: "done" },
  { label: "Connect session", status: "done" },
  { label: "View page", status: "active" },
  { label: "Read state", status: "pending" },
  { label: "Save screenshot", status: "pending" },
];

function StepIcon({ status }: { status: string }) {
  if (status === "done")
    return (
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <circle cx="7" cy="7" r="6.5" stroke="#4d8a6a" strokeWidth="1" fill="rgba(77,138,106,0.15)" />
        <path d="M4 7l2 2 4-4" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  if (status === "active")
    return (
      <span style={{ position: "relative", display: "inline-flex", alignItems: "center", justifyContent: "center", width: 14, height: 14 }}>
        <span style={{ width: 6, height: 6, borderRadius: "50%", background: "#7ab89a", display: "block", boxShadow: "0 0 6px rgba(122,184,154,0.8)" }} />
      </span>
    );
  return (
    <span style={{ display: "inline-flex", width: 14, height: 14, alignItems: "center", justifyContent: "center" }}>
      <span style={{ width: 6, height: 6, borderRadius: "50%", border: "1px solid rgba(50,68,90,0.9)", display: "block" }} />
    </span>
  );
}

function AbstractMiniProgram() {
  return (
    <div
      style={{
        background: "#0d1117",
        border: "1px solid rgba(50,68,90,0.5)",
        borderRadius: 12,
        overflow: "hidden",
        width: "100%",
        maxWidth: 160,
        display: "flex",
        flexDirection: "column",
        height: "100%",
      }}
    >
      {/* Status bar */}
      <div style={{ height: 26, background: "#131920", display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0 10px" }}>
        <div style={{ width: 36, height: 4, background: "#2a3547", borderRadius: 2 }} />
        <div style={{ width: 16, height: 4, background: "#2a3547", borderRadius: 2 }} />
      </div>
      {/* Nav tabs */}
      <div style={{ height: 34, background: "#131920", borderBottom: "1px solid rgba(50,68,90,0.4)", display: "flex", alignItems: "center", gap: 8, padding: "0 10px" }}>
        {[true, false, false].map((a, i) => (
          <div key={i} style={{ width: a ? 52 : 36, height: 4, background: a ? "#4d8a6a" : "#2a3547", borderRadius: 2, opacity: a ? 1 : 0.5 }} />
        ))}
      </div>
      {/* Content */}
      <div style={{ flex: 1, padding: 10, display: "flex", flexDirection: "column", gap: 7 }}>
        <div style={{ width: "100%", height: 52, background: "#1a2230", borderRadius: 6, position: "relative", overflow: "hidden" }}>
          <div style={{ position: "absolute", inset: 0, background: "linear-gradient(135deg, #1d2e3f 0%, #1a2230 100%)" }} />
          <div style={{ position: "absolute", bottom: 8, left: 8, width: 70, height: 3, background: "#2a3547", borderRadius: 2 }} />
          <div style={{ position: "absolute", bottom: 14, left: 8, width: 90, height: 3, background: "#2a3547", borderRadius: 2 }} />
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          {[0.9, 1, 0.7].map((op, i) => (
            <div key={i} style={{ flex: 1, height: 38, background: "#1a2230", borderRadius: 5, opacity: op, display: "flex", flexDirection: "column", justifyContent: "flex-end", padding: "6px 5px", gap: 3 }}>
              <div style={{ width: "70%", height: 3, background: "#2a3547", borderRadius: 2 }} />
              <div style={{ width: "50%", height: 3, background: "#2a3547", borderRadius: 2 }} />
            </div>
          ))}
        </div>
        {[1, 0.7, 0.45].map((op, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, opacity: op }}>
            <div style={{ width: 5, height: 5, borderRadius: "50%", background: i === 0 ? "#4d8a6a" : "#2a3547", flexShrink: 0 }} />
            <div style={{ flex: 1, height: 3, background: "#2a3547", borderRadius: 2 }} />
            <div style={{ width: 18, height: 3, background: "#2a3547", borderRadius: 2 }} />
          </div>
        ))}
      </div>
      {/* CTA row */}
      <div style={{ height: 38, borderTop: "1px solid rgba(50,68,90,0.4)", padding: "7px 10px" }}>
        <div style={{ width: "100%", height: "100%", background: "#4d8a6a", borderRadius: 4, opacity: 0.6 }} />
      </div>
    </div>
  );
}

function HeroDemoPanel() {
  return (
    <div
      style={{
        background: "#131920",
        border: "1px solid rgba(50,68,90,0.6)",
        borderRadius: 14,
        overflow: "hidden",
        boxShadow: "0 0 0 1px rgba(77,138,106,0.12), 0 24px 80px rgba(0,0,0,0.6), 0 0 60px rgba(77,138,106,0.06)",
        fontFamily: MONO,
      }}
    >
      {/* Window chrome */}
      <div style={{ height: 40, background: "#0f161e", borderBottom: "1px solid rgba(50,68,90,0.5)", display: "flex", alignItems: "center", padding: "0 14px", gap: 6 }}>
        <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#ff5f57", display: "block" }} />
        <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#ffbd2e", display: "block" }} />
        <span style={{ width: 10, height: 10, borderRadius: "50%", background: "#28c840", display: "block" }} />
        <div style={{ flex: 1, display: "flex", justifyContent: "center" }}>
          <span style={{ color: "#4a5a6e", fontSize: 11 }}>wxpilot — session-01</span>
        </div>
        <span style={{ fontSize: 10, color: "#4d8a6a", display: "flex", alignItems: "center", gap: 4 }}>
          <span style={{ width: 6, height: 6, borderRadius: "50%", background: "#4d8a6a", display: "block", boxShadow: "0 0 5px rgba(77,138,106,0.8)" }} />
          Connected
        </span>
      </div>

      {/* Main content */}
      <div style={{ display: "flex", height: 260 }}>
        {/* Steps sidebar */}
        <div style={{ width: 148, borderRight: "1px solid rgba(50,68,90,0.4)", padding: "16px 0", display: "flex", flexDirection: "column", gap: 2 }}>
          {steps.map((step) => (
            <div
              key={step.label}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 9,
                padding: "7px 14px",
                background: step.status === "active" ? "rgba(77,138,106,0.08)" : "transparent",
                borderLeft: step.status === "active" ? "2px solid #4d8a6a" : "2px solid transparent",
              }}
            >
              <StepIcon status={step.status} />
              <span
                style={{
                  fontSize: 11,
                  color: step.status === "done" ? "#4a6a5a" : step.status === "active" ? "#7ab89a" : "#3a4a5a",
                  letterSpacing: 0,
                  fontFamily: MONO,
                }}
              >
                {step.label}
              </span>
            </div>
          ))}
        </div>

        {/* Main view */}
        <div style={{ flex: 1, padding: 16, display: "flex", gap: 14, alignItems: "flex-start" }}>
          <AbstractMiniProgram />

          {/* Info panel */}
          <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}>
            <div style={{ background: "#0f161e", borderRadius: 6, padding: "8px 10px", border: "1px solid rgba(50,68,90,0.4)" }}>
              <div style={{ fontSize: 10, color: "#4a5a6e", marginBottom: 6 }}>Interface</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {[
                  { label: "Page", val: "home", active: true },
                  { label: "Elements", val: "12" },
                  { label: "State", val: "ready" },
                ].map((row) => (
                  <div key={row.label} style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span style={{ fontSize: 10, color: "#4a5a6e" }}>{row.label}</span>
                    <span style={{ fontSize: 10, color: row.active ? "#7ab89a" : "#5a6a7e", fontFamily: MONO }}>{row.val}</span>
                  </div>
                ))}
              </div>
            </div>

            <div style={{ background: "#0f161e", borderRadius: 6, padding: "8px 10px", border: "1px solid rgba(50,68,90,0.4)", flex: 1 }}>
              <div style={{ fontSize: 10, color: "#4a5a6e", marginBottom: 6 }}>Action queue</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
                {["view", "state", "shot"].map((action, i) => (
                  <div key={action} style={{ display: "flex", alignItems: "center", gap: 5 }}>
                    <div style={{ width: 4, height: 4, borderRadius: "50%", background: i === 0 ? "#4d8a6a" : "#2a3547" }} />
                    <span style={{ fontSize: 10, color: i === 0 ? "#4d8a6a" : "#2a3547", fontFamily: MONO }}>{action}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Terminal output */}
      <div style={{ borderTop: "1px solid rgba(50,68,90,0.4)", background: "#0d1117", padding: "10px 16px" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ color: "#4d8a6a", fontSize: 11 }}>$</span>
            <span style={{ color: "#7a8ba8", fontSize: 11, fontFamily: MONO }}>wxpilot view</span>
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ color: "#4d8a6a", fontSize: 11 }}>$</span>
            <span style={{ color: "#7a8ba8", fontSize: 11, fontFamily: MONO }}>wxpilot state</span>
            <span style={{ color: "#4d8a6a", fontSize: 12, animation: "blink 1.2s step-end infinite" }}>▋</span>
          </div>
        </div>
      </div>

      <style>{`@keyframes blink{0%,50%{opacity:1}51%,100%{opacity:0}}`}</style>
    </div>
  );
}

export function Hero() {
  return (
    <section
      style={{
        background: "#0d1117",
        paddingTop: 120,
        paddingBottom: 100,
        fontFamily: SANS,
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Background glow */}
      <div
        style={{
          position: "absolute",
          top: 0,
          left: "50%",
          transform: "translateX(-50%)",
          width: "80%",
          height: 400,
          background: "radial-gradient(ellipse at 50% 0%, rgba(77,138,106,0.08) 0%, transparent 70%)",
          pointerEvents: "none",
        }}
      />

      <div className="max-w-6xl mx-auto px-6">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 64, alignItems: "center" }}
          className="grid-cols-1 md:grid-cols-2">

          {/* Left: text */}
          <motion.div
            style={{ display: "flex", flexDirection: "column", gap: 28 }}
            initial="hidden"
            animate="show"
            variants={{ hidden: {}, show: { transition: { staggerChildren: 0.12 } } }}
          >
            <motion.div variants={{ hidden: { opacity: 0, y: 22 }, show: { opacity: 1, y: 0, transition: { duration: 0.6, ease: EASE } } }}>
              <div style={{ fontSize: 11, letterSpacing: "0.12em", color: "#4d8a6a", textTransform: "uppercase", marginBottom: 20, fontFamily: MONO }}>
                wxpilot
              </div>
              <h1
                style={{
                  fontSize: "clamp(32px, 4.5vw, 52px)",
                  color: "#e8e6e1",
                  fontWeight: 500,
                  lineHeight: 1.15,
                  letterSpacing: "-0.02em",
                  margin: 0,
                  fontFamily: SANS,
                }}
              >
                让 AI 更自然地
                <br />
                操作微信小程序
              </h1>
            </motion.div>

            <motion.p
              variants={{ hidden: { opacity: 0, y: 18 }, show: { opacity: 1, y: 0, transition: { duration: 0.55, ease: EASE } } }}
              style={{
                fontSize: 16,
                color: "#7a8ba8",
                lineHeight: 1.75,
                maxWidth: 420,
                margin: 0,
              }}
            >
              wxpilot 是面向开发者与 AI 工作流的微信小程序自动化工具，帮助你更顺畅地完成页面浏览、基础交互、状态观察、截图记录与调试辅助。
            </motion.p>

            <motion.div
              variants={{ hidden: { opacity: 0, y: 14 }, show: { opacity: 1, y: 0, transition: { duration: 0.5, ease: EASE } } }}
              style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
              <button
                style={{
                  background: "#4d8a6a",
                  color: "#e8e6e1",
                  border: "none",
                  padding: "11px 24px",
                  borderRadius: 7,
                  fontSize: 14,
                  fontFamily: SANS,
                  cursor: "pointer",
                  letterSpacing: "0.01em",
                }}
                className="hover:opacity-90 transition-opacity duration-200"
              >
                立即开始
              </button>
            </motion.div>

            <motion.p
              variants={{ hidden: { opacity: 0 }, show: { opacity: 1, transition: { duration: 0.5, ease: EASE } } }}
              style={{ fontSize: 13, color: "#4a5a6e", margin: 0, fontFamily: MONO }}
            >
              为开发、测试、演示与重复操作而设计
            </motion.p>
          </motion.div>

          {/* Right: demo panel */}
          <motion.div
            className="hidden md:block"
            initial={{ opacity: 0, x: 32 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.7, ease: EASE, delay: 0.3 }}
          >
            <HeroDemoPanel />
          </motion.div>
        </div>
      </div>

      <style>{`
        @media (max-width: 768px) {
          .hero-grid { grid-template-columns: 1fr !important; }
        }
      `}</style>
    </section>
  );
}
