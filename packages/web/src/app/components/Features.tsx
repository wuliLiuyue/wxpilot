import { FadeUp, StaggerContainer, StaggerItem } from "./animations";

const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";

const features = [
  {
    id: "explore",
    label: "页面浏览",
    body: "更高效地查看当前页面与操作上下文。",
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <rect x="2" y="3" width="16" height="11" rx="2" stroke="#4d8a6a" strokeWidth="1.2" />
        <path d="M6 17h8" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" />
        <path d="M5 7h10M5 10h6" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" />
      </svg>
    ),
    visual: (
      <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
        {[85, 60, 75, 45].map((w, i) => (
          <div key={i} style={{ height: 4, width: `${w}%`, background: i === 0 ? "rgba(77,138,106,0.4)" : "rgba(50,68,90,0.5)", borderRadius: 2 }} />
        ))}
      </div>
    ),
  },
  {
    id: "interact",
    label: "界面交互",
    body: "完成常见页面操作，帮助流程顺畅推进。",
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M8 4v8M12 4v8" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" />
        <path d="M5 10l3 6 2-4 2 4 3-6" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
    visual: (
      <div style={{ display: "flex", gap: 5, alignItems: "center" }}>
        {["tap", "input", "scroll"].map((a, i) => (
          <div key={a} style={{ padding: "3px 8px", borderRadius: 4, background: i === 0 ? "rgba(77,138,106,0.2)" : "rgba(50,68,90,0.3)", border: `1px solid ${i === 0 ? "rgba(77,138,106,0.4)" : "rgba(50,68,90,0.4)"}` }}>
            <span style={{ fontSize: 9, color: i === 0 ? "#4d8a6a" : "#3a4a5a", fontFamily: MONO }}>{a}</span>
          </div>
        ))}
      </div>
    ),
  },
  {
    id: "observe",
    label: "状态观察",
    body: "读取当前界面和任务过程中的关键信息。",
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <circle cx="10" cy="10" r="3" stroke="#4d8a6a" strokeWidth="1.2" />
        <path d="M2 10c2-4 5-6 8-6s6 2 8 6c-2 4-5 6-8 6s-6-2-8-6z" stroke="#4d8a6a" strokeWidth="1.2" />
      </svg>
    ),
    visual: (
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {[["state", "ready"], ["page", "home"], ["ctx", "active"]].map(([k, v]) => (
          <div key={k} style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ fontSize: 9, color: "#3a4a5a", fontFamily: MONO }}>{k}</span>
            <span style={{ fontSize: 9, color: "#4d8a6a", fontFamily: MONO }}>{v}</span>
          </div>
        ))}
      </div>
    ),
  },
  {
    id: "capture",
    label: "截图记录",
    body: "保留执行结果，用于验证、汇报或协作。",
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <rect x="2" y="4" width="16" height="13" rx="2" stroke="#4d8a6a" strokeWidth="1.2" />
        <circle cx="10" cy="10.5" r="2.5" stroke="#4d8a6a" strokeWidth="1.2" />
        <path d="M7 4l1-1h4l1 1" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" />
      </svg>
    ),
    visual: (
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
        {[0.9, 0.6, 0.7, 0.4].map((op, i) => (
          <div key={i} style={{ height: 22, background: `rgba(77,138,106,${op * 0.12})`, borderRadius: 3, border: `1px solid rgba(77,138,106,${op * 0.25})` }} />
        ))}
      </div>
    ),
  },
  {
    id: "debug",
    label: "调试辅助",
    body: "帮助团队更顺畅地处理联调、排查与复现。",
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M7 3l6 0M6 6c0-1.1.9-2 2-2h4a2 2 0 012 2v6a2 2 0 01-2 2H8a2 2 0 01-2-2V6z" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" />
        <path d="M10 9v3M4 8H6M14 8h2M4 12H6M14 12h2" stroke="#4d8a6a" strokeWidth="1.2" strokeLinecap="round" />
      </svg>
    ),
    visual: (
      <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
        {["▸ inspect element", "▸ trace actions", "▸ export log"].map((cmd, i) => (
          <div key={cmd} style={{ opacity: 1 - i * 0.25 }}>
            <span style={{ fontSize: 9, color: i === 0 ? "#4d8a6a" : "#3a4a5a", fontFamily: MONO }}>{cmd}</span>
          </div>
        ))}
      </div>
    ),
  },
];

export function Features() {
  return (
    <section
      id="features"
      style={{
        background: "#0d1117",
        borderTop: "1px solid rgba(50,68,90,0.4)",
        padding: "96px 0",
        fontFamily: SANS,
      }}
    >
      <div className="max-w-6xl mx-auto px-6">
        <FadeUp style={{ marginBottom: 64 }}>
          <div style={{ fontSize: 11, letterSpacing: "0.12em", color: "#4d8a6a", textTransform: "uppercase", marginBottom: 16, fontFamily: MONO }}>
            Capabilities
          </div>
          <h2 style={{ fontSize: "clamp(24px, 3vw, 36px)", color: "#e8e6e1", fontWeight: 500, lineHeight: 1.25, letterSpacing: "-0.02em", margin: 0 }}>
            聚焦你真正会用到的能力
          </h2>
        </FadeUp>

        <StaggerContainer style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 1 }} className="features-grid">
          {features.map((f, i) => (
            <StaggerItem
              key={f.id}
              style={{
                background: "#131920",
                border: "1px solid rgba(50,68,90,0.35)",
                borderRadius: i === 0 ? "12px 0 0 0" : i === 1 ? 0 : i === 2 ? "0 12px 0 0" : i === 3 ? "0 0 0 12px" : i === 4 ? "0 0 12px 0" : 0,
                padding: "28px 28px 24px",
                display: "flex",
                flexDirection: "column",
                gap: 16,
                transition: "background 0.2s",
              }}
              className="feature-card"
            >
              <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between" }}>
                <div style={{ width: 38, height: 38, borderRadius: 8, background: "rgba(77,138,106,0.08)", border: "1px solid rgba(77,138,106,0.15)", display: "flex", alignItems: "center", justifyContent: "center" }}>
                  {f.icon}
                </div>
                <div style={{ padding: "6px 10px", background: "#0d1117", borderRadius: 6, border: "1px solid rgba(50,68,90,0.35)", minWidth: 90 }}>
                  {f.visual}
                </div>
              </div>
              <div>
                <h3 style={{ fontSize: 15, color: "#e8e6e1", fontWeight: 500, lineHeight: 1.35, margin: "0 0 8px 0" }}>
                  {f.label}
                </h3>
                <p style={{ fontSize: 13, color: "#7a8ba8", lineHeight: 1.7, margin: 0 }}>
                  {f.body}
                </p>
              </div>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </div>

      <style>{`
        .features-grid { grid-template-columns: repeat(3, 1fr); }
        .feature-card:hover { background: #161d27 !important; }
        @media (max-width: 768px) {
          .features-grid { grid-template-columns: repeat(2, 1fr) !important; }
          .feature-card { border-radius: 8px !important; }
        }
        @media (max-width: 480px) {
          .features-grid { grid-template-columns: 1fr !important; }
        }
      `}</style>
    </section>
  );
}
