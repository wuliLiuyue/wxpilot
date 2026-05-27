import { FadeUp, StaggerContainer, StaggerItem } from "./animations";

const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";

const values = [
  {
    num: "01",
    title: "更适合 AI 工作流",
    body: "让模型和自动化流程更自然地理解并操作小程序界面。",
  },
  {
    num: "02",
    title: "更简单的开始方式",
    body: "减少接入负担，让团队更快进入可用状态。",
  },
  {
    num: "03",
    title: "更稳定的交互体验",
    body: "围绕常见浏览、点击、输入、观察等任务进行设计。",
  },
  {
    num: "04",
    title: "更顺畅的调试辅助",
    body: "帮助团队更高效地定位问题、记录过程和复现操作。",
  },
];

export function Values() {
  return (
    <section
      style={{
        background: "#0d1117",
        borderTop: "1px solid rgba(50,68,90,0.4)",
        padding: "96px 0",
        fontFamily: SANS,
      }}
    >
      <div className="max-w-6xl mx-auto px-6">
        {/* Header */}
        <FadeUp style={{ marginBottom: 64, maxWidth: 520 }}>
          <div style={{ fontSize: 11, letterSpacing: "0.12em", color: "#4d8a6a", textTransform: "uppercase", marginBottom: 16, fontFamily: MONO }}>
            Why wxpilot
          </div>
          <h2 style={{ fontSize: "clamp(24px, 3vw, 36px)", color: "#e8e6e1", fontWeight: 500, lineHeight: 1.25, letterSpacing: "-0.02em", margin: "0 0 14px 0" }}>
            为真实工作流而构建
          </h2>
          <p style={{ fontSize: 16, color: "#7a8ba8", lineHeight: 1.7, margin: 0 }}>
            不是概念演示，而是面向日常使用的自动化体验。
          </p>
        </FadeUp>

        {/* Value cards */}
        <StaggerContainer
          style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: 2 }}
          className="value-grid"
        >
          {values.map((v, i) => (
            <StaggerItem
              key={v.num}
              style={{
                background: i % 2 === 0 ? "#131920" : "#0f161e",
                border: "1px solid rgba(50,68,90,0.35)",
                borderRadius: i === 0 ? "12px 0 0 0" : i === 1 ? "0 12px 0 0" : i === 2 ? "0 0 0 12px" : "0 0 12px 0",
                padding: "36px 32px",
                display: "flex",
                flexDirection: "column",
                gap: 14,
                transition: "background 0.2s",
                position: "relative",
                overflow: "hidden",
              }}
              className="value-card"
            >
              {/* Subtle corner accent */}
              <div style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: 2,
                height: 40,
                background: "linear-gradient(to bottom, rgba(77,138,106,0.5), transparent)",
              }} />
              <span style={{ fontSize: 11, color: "#2a3d4f", fontFamily: MONO, letterSpacing: "0.05em" }}>{v.num}</span>
              <h3 style={{ fontSize: 18, color: "#e8e6e1", fontWeight: 500, lineHeight: 1.35, letterSpacing: "-0.01em", margin: 0 }}>
                {v.title}
              </h3>
              <p style={{ fontSize: 14, color: "#7a8ba8", lineHeight: 1.7, margin: 0 }}>
                {v.body}
              </p>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </div>

      <style>{`
        .value-grid { grid-template-columns: repeat(2, 1fr); }
        .value-card:hover { background: #161d27 !important; }
        @media (max-width: 640px) {
          .value-grid { grid-template-columns: 1fr !important; }
          .value-card { border-radius: 8px !important; }
        }
      `}</style>
    </section>
  );
}
