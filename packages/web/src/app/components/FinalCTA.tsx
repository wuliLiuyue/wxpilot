import { FadeUp } from "./animations";

const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";

export function FinalCTA() {
  return (
    <section
      style={{
        background: "#0f161e",
        borderTop: "1px solid rgba(50,68,90,0.4)",
        padding: "120px 0",
        fontFamily: SANS,
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Background radial glow */}
      <div
        style={{
          position: "absolute",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          width: "60%",
          height: "80%",
          background: "radial-gradient(ellipse at center, rgba(77,138,106,0.07) 0%, transparent 70%)",
          pointerEvents: "none",
        }}
      />

      <FadeUp className="max-w-6xl mx-auto px-6 text-center" style={{ position: "relative" }}>
        <div style={{ fontSize: 11, letterSpacing: "0.12em", color: "#4d8a6a", textTransform: "uppercase", marginBottom: 24, fontFamily: MONO }}>
          Get started
        </div>

        <h2
          style={{
            fontSize: "clamp(28px, 4vw, 48px)",
            color: "#e8e6e1",
            fontWeight: 500,
            lineHeight: 1.2,
            letterSpacing: "-0.025em",
            margin: "0 auto 20px",
            maxWidth: 640,
          }}
        >
          把微信小程序自动化
          <br />
          接入你的工作流
        </h2>

        <p style={{ fontSize: 16, color: "#7a8ba8", lineHeight: 1.75, margin: "0 auto 48px", maxWidth: 460 }}>
          从日常操作到 AI 驱动任务，用更自然的方式开始。
        </p>

        <div style={{ display: "flex", gap: 14, justifyContent: "center", flexWrap: "wrap" }}>
          <button
            style={{
              background: "#4d8a6a",
              color: "#e8e6e1",
              border: "none",
              padding: "13px 28px",
              borderRadius: 8,
              fontSize: 15,
              fontFamily: SANS,
              cursor: "pointer",
              letterSpacing: "0.01em",
              boxShadow: "0 0 24px rgba(77,138,106,0.25)",
            }}
            className="hover:opacity-90 transition-opacity duration-200"
          >
            开始使用 wxpilot
          </button>
        </div>
      </FadeUp>
    </section>
  );
}
